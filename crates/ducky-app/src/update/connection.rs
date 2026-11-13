use ducky_core::Catalog;

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::Connection;
use crate::state::{AppState, MainView, NoticeLevel, ProfilePicker, Screen, Workspace};
use crate::update::{clamp_index, notice, workspace_mut};

pub(crate) fn connected(
    state: &mut AppState,
    profile_index: usize,
    result: Result<Connection, String>,
) -> Vec<Command> {
    match &mut state.screen {
        Screen::Profiles(picker) => {
            if picker.connecting_profile_index != Some(profile_index) {
                return Vec::new();
            }
            picker.connecting_profile_index = None;
            match result {
                Err(message) => {
                    picker.connection_error = Some(message);
                    Vec::new()
                }
                Ok(connection) => {
                    let Some(profile) = state.profiles.get(profile_index) else {
                        return Vec::new();
                    };
                    let workspace = Workspace::new(profile_index, profile.clone(), connection);
                    state.screen = Screen::Workspace(Box::new(workspace));
                    vec![Command::LoadCatalog]
                }
            }
        }
        Screen::Workspace(workspace) => match result {
            Err(message) => {
                state.notice = notice(NoticeLevel::Error, message);
                Vec::new()
            }
            Ok(connection) => {
                if workspace.profile_index != profile_index {
                    return Vec::new();
                }
                workspace.session = connection.session;
                workspace.tools = connection.tools;
                state.notice = notice(NoticeLevel::Success, "Reconnected");
                let mut commands = vec![Command::LoadCatalog];
                if let Some(table) = &mut workspace.table {
                    table.page = Loadable::Loading;
                    commands.push(Command::LoadPage(table.request.clone()));
                }
                commands
            }
        },
    }
}

pub(crate) fn reconnect(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = state.workspace() else {
        return Vec::new();
    };
    let command = Command::Connect {
        profile_index: workspace.profile_index,
        profile: workspace.profile.clone(),
    };
    state.notice = notice(NoticeLevel::Info, "Reconnecting…");
    vec![command]
}

pub(crate) fn disconnect(state: &mut AppState) -> Vec<Command> {
    let Some(profile_index) = state.workspace().map(|workspace| workspace.profile_index) else {
        return Vec::new();
    };
    state.screen = Screen::Profiles(ProfilePicker {
        selected_profile_index: profile_index,
        connecting_profile_index: None,
        connection_error: None,
    });
    state.modal = None;
    vec![Command::Disconnect]
}

pub(crate) fn catalog_loaded(
    state: &mut AppState,
    result: Result<Catalog, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let catalog = match result {
        Ok(catalog) => catalog,
        Err(message) => {
            workspace.catalog = Loadable::Failed(message);
            return Vec::new();
        }
    };
    workspace.schema_name = fallback_schema_name(&catalog, &workspace.schema_name);
    let is_open_relation_gone = workspace
        .table
        .as_ref()
        .is_some_and(|table| catalog.relation(&table.relation).is_none());
    if is_open_relation_gone {
        workspace.table = None;
        if workspace.view == MainView::Table {
            workspace.view = MainView::Welcome;
        }
    }
    workspace.catalog = Loadable::Ready(catalog);
    workspace.sidebar.selected_item_index = clamp_index(
        workspace.sidebar.selected_item_index,
        workspace.sidebar_items().len(),
    );
    Vec::new()
}

fn fallback_schema_name(catalog: &Catalog, current_schema_name: &str) -> String {
    if catalog.schema(current_schema_name).is_some() {
        return current_schema_name.to_owned();
    }
    if catalog.schema("public").is_some() {
        return "public".to_owned();
    }
    catalog
        .schemas
        .first()
        .map(|schema| schema.name.clone())
        .unwrap_or_default()
}
