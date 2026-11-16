mod cell_editor;
mod cell_edits;
mod connection;
mod files;
mod focus;
mod grid;
mod jobs;
mod modal;
mod modal_selection;
mod palette;
mod parameters;
mod profiles;
mod query;
mod references;
mod row_deletion;
mod row_keys;
mod run_confirmation;
mod selection;
mod sidebar;
mod switch;
mod table;
mod tabs;

use crate::command::Command;
use crate::msg::Msg;
use crate::state::{AppState, Modal, Notice, NoticeLevel, Screen, Workspace};

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Command> {
    match msg {
        Msg::Quit => {
            state.should_quit = true;
            Vec::new()
        }
        Msg::DismissNotice => {
            state.notice = None;
            Vec::new()
        }
        Msg::OpenHelp => {
            state.modal = Some(Modal::Help);
            Vec::new()
        }
        Msg::Notified(notice) => {
            state.notice = Some(notice);
            Vec::new()
        }
        Msg::ToggleFocus => focus::toggle(state),
        Msg::FocusPane(direction) => focus::move_towards(state, direction),
        Msg::OpenSchemaPicker => open_schema_picker(state),
        Msg::OpenPalette => palette::open(state),
        Msg::OpenProfilePicker => profiles::open(state),
        Msg::Tabs(tabs_msg) => tabs::update(state, tabs_msg),
        Msg::RefreshCatalog => refresh_catalog(state),
        Msg::Reconnect => connection::reconnect(state),
        Msg::Disconnect => connection::disconnect(state),
        Msg::Connected {
            profile_index,
            result,
        } => connection::connected(state, profile_index, result),
        Msg::CatalogLoaded(result) => connection::catalog_loaded(state, result),
        Msg::Profiles(profiles_msg) => profiles::update(state, profiles_msg),
        Msg::Sidebar(sidebar_msg) => sidebar::update(state, sidebar_msg),
        Msg::Table(table_msg) => table::update(state, table_msg),
        Msg::PageLoaded { request, result } => table::page_loaded(state, request, result),
        Msg::DetailsLoaded { relation, result } => table::details_loaded(state, relation, result),
        Msg::Query(query_msg) => query::update(state, query_msg),
        Msg::ScriptFinished { tab_id, report } => query::script_finished(state, tab_id, report),
        Msg::FileSaved { name, result } => query::file_saved(state, name, result),
        Msg::Files(files_msg) => files::update(state, files_msg),
        Msg::SqlFilesListed(result) => files::listed(state, result),
        Msg::FilePreviewLoaded { name, result } => files::preview_loaded(state, name, result),
        Msg::FileOpened {
            name,
            run_after_open,
            result,
        } => files::opened(state, name, run_after_open, result),
        Msg::Dump(dump_msg) => jobs::update_dump(state, dump_msg),
        Msg::Restore(restore_msg) => jobs::update_restore(state, restore_msg),
        Msg::DumpsListed(result) => jobs::dumps_listed(state, result),
        Msg::JobLogged { job, line } => jobs::logged(state, job, line),
        Msg::JobFinished { job, result } => jobs::finished(state, job, result),
        Msg::Modal(modal_msg) => modal::update(state, modal_msg),
        Msg::CellEditsSaved { tab_id, result } => cell_edits::saved(state, tab_id, result),
        Msg::RowsDeleted { tab_id, result } => row_deletion::deleted(state, tab_id, result),
    }
}

pub(crate) fn workspace_mut(screen: &mut Screen) -> Option<&mut Workspace> {
    match screen {
        Screen::Workspace(workspace) => Some(workspace.as_mut()),
        Screen::Profiles(_) => None,
    }
}

pub(crate) fn notice(level: NoticeLevel, text: impl Into<String>) -> Option<Notice> {
    Some(Notice::new(level, text))
}

pub(crate) fn clamp_index(index: usize, length: usize) -> usize {
    index.min(length.saturating_sub(1))
}

pub(crate) fn step_index(index: usize, length: usize, is_forward: bool) -> usize {
    if is_forward {
        clamp_index(index + 1, length)
    } else {
        index.saturating_sub(1)
    }
}

fn open_schema_picker(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = state.workspace() else {
        return Vec::new();
    };
    if workspace.catalog.ready().is_none() {
        return Vec::new();
    }
    let selected_schema_index = workspace
        .schema_names()
        .iter()
        .position(|schema_name| *schema_name == workspace.schema_name)
        .unwrap_or(0);
    state.modal = Some(Modal::SchemaPicker {
        selected_schema_index,
    });
    Vec::new()
}

fn refresh_catalog(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if workspace.catalog.ready().is_none() {
        workspace.catalog = crate::loadable::Loadable::Loading;
    }
    vec![Command::LoadCatalog]
}
