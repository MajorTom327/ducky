pub mod files;
pub mod help;
pub mod hints;
pub mod jobs;
pub mod modal;
pub mod profiles;
pub mod query;
pub mod sequence;
pub mod sidebar;
pub mod table;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Focus, Msg, PaneDirection, QueryMsg, QueryPane, Screen, TabContent, TabId, TabsMsg,
    Workspace,
};
use edtui::EditorMode;

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
        Some(workspace) if state.modal.is_none() && focused_editor_tab(workspace).is_some() => {
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

pub fn focused_editor_tab(workspace: &Workspace) -> Option<TabId> {
    let tab = workspace.active_tab()?;
    match &tab.content {
        TabContent::Query(query)
            if workspace.focus == Focus::Main && query.pane == QueryPane::Editor =>
        {
            Some(tab.id)
        }
        _ => None,
    }
}

fn route_key(state: &AppState, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if is_ctrl(key, 'q') {
        return vec![Msg::Quit];
    }
    if let Some(modal) = &state.modal {
        tui.pending_prefix = None;
        return modal::handle(modal, tui, key);
    }
    let workspace = match &state.screen {
        Screen::Profiles(_) => return profiles::handle(key),
        Screen::Workspace(workspace) => workspace,
    };
    let editor_tab_id = focused_editor_tab(workspace);
    if let Some(direction) = pane_direction(key) {
        if let Some(editor) = editor_tab_id.and_then(|tab_id| tui.editors.get_mut(&tab_id)) {
            editor.completion = None;
        }
        return vec![Msg::FocusPane(direction)];
    }
    let is_completion_open = editor_tab_id
        .and_then(|tab_id| tui.editors.get(&tab_id))
        .is_some_and(|editor| editor.completion.is_some());
    if is_completion_open {
        return query::handle_editor(workspace, tui, key);
    }
    if let Some(global_messages) = global_messages(workspace, key) {
        return global_messages;
    }
    if let Some(tab_id) = editor_tab_id {
        let is_normal_mode = tui.editor(tab_id).state.mode == EditorMode::Normal;
        if is_normal_mode && let Some(sequence_messages) = sequence::handle(tui, key) {
            return sequence_messages;
        }
        return query::handle_editor(workspace, tui, key);
    }
    if let Some(sequence_messages) = sequence::handle(tui, key) {
        return sequence_messages;
    }
    if let Some(navigation_messages) = navigation_messages(key) {
        return navigation_messages;
    }
    if workspace.focus == Focus::Sidebar {
        return sidebar::handle(workspace, key);
    }
    match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Table(_)) => table::handle(workspace, tui, key),
        Some(TabContent::Query(_)) => query::handle_results(workspace, tui, key),
        Some(TabContent::Files) => files::handle(workspace, tui, key),
        Some(TabContent::Dump | TabContent::Restore) => jobs::handle(workspace, tui, key),
        None => Vec::new(),
    }
}

fn global_messages(workspace: &Workspace, key: KeyEvent) -> Option<Vec<Msg>> {
    let is_control = key.modifiers.contains(KeyModifiers::CONTROL);
    let message = match key.code {
        KeyCode::F(1) => Msg::OpenHelp,
        KeyCode::PageDown if is_control => Msg::Tabs(TabsMsg::Next),
        KeyCode::PageUp if is_control => Msg::Tabs(TabsMsg::Previous),
        _ if is_ctrl(key, 'p') => Msg::OpenPalette,
        _ if is_ctrl(key, 'n') => Msg::Tabs(TabsMsg::NewQuery),
        _ if is_ctrl(key, 'w') => Msg::Tabs(TabsMsg::Close),
        _ if is_ctrl(key, 'o') => Msg::Tabs(TabsMsg::OpenFiles),
        _ if is_ctrl(key, 'c') && workspace.running_script_tab_id.is_some() => {
            Msg::Query(QueryMsg::Cancel)
        }
        _ => return None,
    };
    Some(vec![message])
}

fn pane_direction(key: KeyEvent) -> Option<PaneDirection> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('h') => Some(PaneDirection::Left),
        KeyCode::Char('j') => Some(PaneDirection::Down),
        KeyCode::Char('k') => Some(PaneDirection::Up),
        KeyCode::Char('l') => Some(PaneDirection::Right),
        _ => None,
    }
}

fn navigation_messages(key: KeyEvent) -> Option<Vec<Msg>> {
    let message = match key.code {
        KeyCode::Tab | KeyCode::BackTab => Msg::ToggleFocus,
        KeyCode::Char('q') if is_plain_char(key, 'q') => Msg::Quit,
        KeyCode::Char('?') if is_plain_char(key, '?') => Msg::OpenHelp,
        KeyCode::Char('H') if is_plain_char(key, 'H') => Msg::Tabs(TabsMsg::Previous),
        KeyCode::Char('L') if is_plain_char(key, 'L') => Msg::Tabs(TabsMsg::Next),
        KeyCode::Char(digit @ '1'..='9') if is_plain_char(key, digit) => {
            let tab_index = digit.to_digit(10).map_or(0, |number| number as usize - 1);
            Msg::Tabs(TabsMsg::Select(tab_index))
        }
        _ => return None,
    };
    Some(vec![message])
}
