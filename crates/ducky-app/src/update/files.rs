use ducky_core::FileEntry;

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::FilesMsg;
use crate::state::{AppState, FilesView, NoticeLevel, TabContent};
use crate::update::parameters::request_run;
use crate::update::tabs::open_document;
use crate::update::{clamp_index, notice, step_index, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: FilesMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if !workspace.is_active_content(|content| matches!(content, TabContent::Files)) {
        return Vec::new();
    }
    let files = &mut workspace.files;
    match msg {
        FilesMsg::Next | FilesMsg::Previous => {
            let Some(entry_count) = files.entries.ready().map(Vec::len) else {
                return Vec::new();
            };
            let next_index = step_index(
                files.selected_entry_index,
                entry_count,
                msg == FilesMsg::Next,
            );
            if next_index == files.selected_entry_index {
                return Vec::new();
            }
            files.selected_entry_index = next_index;
            load_preview(files)
        }
        FilesMsg::Open | FilesMsg::Execute => match files.selected_entry() {
            Some(entry) => vec![Command::OpenFile {
                name: entry.name.clone(),
                run_after_open: msg == FilesMsg::Execute,
            }],
            None => Vec::new(),
        },
        FilesMsg::Reload => {
            files.entries = Loadable::Loading;
            vec![Command::ListSqlFiles]
        }
    }
}

fn load_preview(files: &mut FilesView) -> Vec<Command> {
    let Some(name) = files.selected_entry().map(|entry| entry.name.clone()) else {
        files.preview = Loadable::Idle;
        return Vec::new();
    };
    files.preview = Loadable::Loading;
    vec![Command::LoadFilePreview { name }]
}

pub(crate) fn listed(state: &mut AppState, result: Result<Vec<FileEntry>, String>) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let files = &mut workspace.files;
    files.entries = Loadable::from_result(result);
    let Some(entry_count) = files.entries.ready().map(Vec::len) else {
        return Vec::new();
    };
    files.selected_entry_index = clamp_index(files.selected_entry_index, entry_count);
    load_preview(files)
}

pub(crate) fn preview_loaded(
    state: &mut AppState,
    name: String,
    result: Result<String, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let files = &mut workspace.files;
    if files
        .selected_entry()
        .is_some_and(|entry| entry.name == name)
    {
        files.preview = Loadable::from_result(result);
    }
    Vec::new()
}

pub(crate) fn opened(
    state: &mut AppState,
    name: String,
    run_after_open: bool,
    result: Result<String, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let text = match result {
        Ok(text) => text,
        Err(message) => {
            state.notice = notice(NoticeLevel::Error, message);
            return Vec::new();
        }
    };
    let tab_id = open_document(workspace, name, text.clone());
    if run_after_open {
        request_run(state, tab_id, text)
    } else {
        Vec::new()
    }
}
