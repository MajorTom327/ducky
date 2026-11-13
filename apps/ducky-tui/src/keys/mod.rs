pub mod files;
pub mod help;
pub mod jobs;
pub mod modal;
pub mod profiles;
pub mod query;
pub mod sidebar;
pub mod table;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{AppState, Focus, MainView, Msg, QueryMsg, QueryPane, RunState, Screen, Workspace};

use crate::tui_state::TuiState;

pub fn handle_key(state: &AppState, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let mut messages = route_key(state, tui, key);
    if state.notice.is_some() {
        messages.insert(0, Msg::DismissNotice);
    }
    messages
}

pub fn handle_paste(state: &AppState, tui: &mut TuiState, pasted_text: String) -> Vec<Msg> {
    match state.workspace() {
        Some(workspace) if state.modal.is_none() && is_editor_focused(workspace) => {
            query::handle_paste(workspace, tui, pasted_text)
        }
        _ => Vec::new(),
    }
}

pub fn is_ctrl(key: KeyEvent, character: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(character)
}

pub fn is_plain_char(key: KeyEvent, character: char) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && key.code == KeyCode::Char(character)
}

fn is_editor_focused(workspace: &Workspace) -> bool {
    workspace.focus == Focus::Main
        && workspace.view == MainView::Query
        && workspace.query.pane == QueryPane::Editor
}

fn route_key(state: &AppState, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if is_ctrl(key, 'q') {
        return vec![Msg::Quit];
    }
    if let Some(modal) = &state.modal {
        return modal::handle(modal, key);
    }
    let workspace = match &state.screen {
        Screen::Profiles(_) => return profiles::handle(key),
        Screen::Workspace(workspace) => workspace,
    };
    if key.code == KeyCode::F(1) {
        return vec![Msg::OpenHelp];
    }
    if is_ctrl(key, 'c') && matches!(workspace.query.run, RunState::Running) {
        return vec![Msg::Query(QueryMsg::Cancel)];
    }
    if is_editor_focused(workspace) {
        return query::handle_editor(workspace, tui, key);
    }
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        return vec![Msg::ToggleFocus];
    }
    if is_plain_char(key, 'q') {
        return vec![Msg::Quit];
    }
    if is_plain_char(key, '?') {
        return vec![Msg::OpenHelp];
    }
    match (workspace.focus, workspace.view) {
        (Focus::Sidebar, _) => sidebar::handle(workspace, key),
        (Focus::Main, MainView::Table) => table::handle(workspace, tui, key),
        (Focus::Main, MainView::Query) => query::handle_results(workspace, tui, key),
        (Focus::Main, MainView::Files) => files::handle(workspace, tui, key),
        (Focus::Main, MainView::Dump | MainView::Restore) => jobs::handle(workspace, tui, key),
        (Focus::Main, MainView::Welcome) => Vec::new(),
    }
}
