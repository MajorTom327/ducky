use ducky_core::Catalog;

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::Connection;
use crate::state::{AppState, Focus, NoticeLevel, ProfilePicker, Screen, TabContent, Workspace};
use crate::update::switch::switched;
use crate::update::table::reload_all_pages;
use crate::update::{clamp_index, notice, workspace_mut};

pub(crate) fn connected(
    state: &mut AppState,
    profile_index: usize,
    result: Result<Connection, String>,
) -> Vec<Command> {
    let Some(workspace) = state.workspace() else {
        return picker_connected(state, profile_index, result);
    };
    if workspace.profile_index == profile_index {
        reconnected(state, result)
    } else if workspace.switching_profile_index == Some(profile_index) {
        switched(state, profile_index, result)
    } else {
        Vec::new()
    }
}

fn picker_connected(
    state: &mut AppState,
    profile_index: usize,
    result: Result<Connection, String>,
) -> Vec<Command> {
    let Screen::Profiles(picker) = &mut state.screen else {
        return Vec::new();
    };
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

fn reconnected(state: &mut AppState, result: Result<Connection, String>) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    match result {
        Err(message) => {
            state.notice = notice(NoticeLevel::Error, message);
            Vec::new()
        }
        Ok(connection) => {
            workspace.session = connection.session;
            workspace.tools = connection.tools;
            state.notice = notice(NoticeLevel::Success, "Reconnected");
            let mut commands = vec![Command::LoadCatalog];
            commands.extend(reload_all_pages(workspace));
            commands
        }
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
    close_vanished_tables(workspace, &catalog);
    workspace.catalog = Loadable::Ready(catalog);
    workspace.sidebar.selected_item_index = clamp_index(
        workspace.sidebar.selected_item_index,
        workspace.sidebar_items().len(),
    );
    Vec::new()
}

fn close_vanished_tables(workspace: &mut Workspace, catalog: &Catalog) {
    let active_tab_id = workspace.active_tab_id();
    workspace.tabs.retain(|tab| match &tab.content {
        TabContent::Table(table) => catalog.relation(&table.relation).is_some(),
        _ => true,
    });
    workspace.active_tab_index = active_tab_id
        .and_then(|tab_id| workspace.tab_index(tab_id))
        .unwrap_or_else(|| clamp_index(workspace.active_tab_index, workspace.tabs.len()));
    if workspace.tabs.is_empty() {
        workspace.focus = Focus::Sidebar;
    }
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
