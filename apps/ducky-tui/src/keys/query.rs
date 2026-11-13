use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{GridMove, Msg, QueryMsg, TabContent, TabId, Workspace};
use edtui::EditorMode;

use crate::editor::completion::{accept_completion, refresh_completion};
use crate::editor::highlight::refresh_highlights;
use crate::editor::panel::EditorPanel;
use crate::keys::{focused_editor_tab, is_ctrl, is_plain_char};
use crate::tui_state::TuiState;

pub fn handle_editor(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let Some(tab_id) = focused_editor_tab(workspace) else {
        return Vec::new();
    };
    let editor = tui.editor(tab_id);
    if let Some(messages) = script_messages(editor, key) {
        return messages;
    }
    if is_ctrl(key, ' ') {
        refresh_completion(editor, workspace, true);
        return Vec::new();
    }
    if editor.completion.is_some() && handle_completion_key(editor, key) {
        return Vec::new();
    }
    if editor.state.mode == EditorMode::Normal
        && matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
    {
        return vec![Msg::ToggleFocus];
    }
    let text_before = editor.text();
    editor.event_handler.on_key_event(key, &mut editor.state);
    let has_text_changed = editor.text() != text_before;
    refresh_highlights(editor);
    match has_text_changed && editor.is_insert_mode() {
        true => refresh_completion(editor, workspace, false),
        false => editor.completion = None,
    }
    Vec::new()
}

pub fn handle_paste(workspace: &Workspace, tui: &mut TuiState, pasted_text: String) -> Vec<Msg> {
    let Some(tab_id) = focused_editor_tab(workspace) else {
        return Vec::new();
    };
    let editor = tui.editor(tab_id);
    editor
        .event_handler
        .on_paste_event(pasted_text, &mut editor.state);
    editor.completion = None;
    refresh_highlights(editor);
    Vec::new()
}

pub fn handle_results(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let Some(tab_id) = active_query_tab(workspace) else {
        return Vec::new();
    };
    if let Some(messages) = script_messages(tui.editor(tab_id), key) {
        return messages;
    }
    if let Some(grid_move) = grid_move(key) {
        return vec![Msg::Query(QueryMsg::Move(grid_move))];
    }
    let query_msg = match key.code {
        KeyCode::Esc => return vec![Msg::ToggleFocus],
        KeyCode::Enter => QueryMsg::OpenRowDetail,
        _ if is_plain_char(key, 'p') => QueryMsg::PreviousResult,
        _ if is_plain_char(key, 'n') => QueryMsg::NextResult,
        _ if is_plain_char(key, 'y') => QueryMsg::CopyCell,
        _ => return Vec::new(),
    };
    vec![Msg::Query(query_msg)]
}

fn active_query_tab(workspace: &Workspace) -> Option<TabId> {
    let tab = workspace.active_tab()?;
    match tab.content {
        TabContent::Query(_) => Some(tab.id),
        _ => None,
    }
}

fn script_messages(editor: &EditorPanel, key: KeyEvent) -> Option<Vec<Msg>> {
    let query_msg = match key.code {
        KeyCode::F(5) => QueryMsg::Run { sql: editor.text() },
        KeyCode::F(2) => QueryMsg::SaveAs {
            content: editor.text(),
        },
        _ if is_ctrl(key, 'e') => QueryMsg::Run { sql: editor.text() },
        _ if is_ctrl(key, 's') => QueryMsg::Save {
            content: editor.text(),
        },
        _ if is_ctrl(key, 't') => QueryMsg::ToggleTransactionMode,
        _ => return None,
    };
    Some(vec![Msg::Query(query_msg)])
}

fn handle_completion_key(editor: &mut EditorPanel, key: KeyEvent) -> bool {
    let Some(popup) = editor.completion.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Tab => {
            accept_completion(editor);
            refresh_highlights(editor);
        }
        KeyCode::Down => popup.select_next(),
        KeyCode::Up => popup.select_previous(),
        KeyCode::Esc => editor.completion = None,
        _ if is_ctrl(key, 'n') => popup.select_next(),
        _ if is_ctrl(key, 'p') => popup.select_previous(),
        _ => return false,
    }
    true
}

fn grid_move(key: KeyEvent) -> Option<GridMove> {
    let grid_move = match key.code {
        KeyCode::Left => GridMove::Left,
        KeyCode::Right => GridMove::Right,
        KeyCode::Up => GridMove::Up,
        KeyCode::Down => GridMove::Down,
        _ if is_plain_char(key, 'h') => GridMove::Left,
        _ if is_plain_char(key, 'l') => GridMove::Right,
        _ if is_plain_char(key, 'k') => GridMove::Up,
        _ if is_plain_char(key, 'j') => GridMove::Down,
        _ if is_plain_char(key, 'G') => GridMove::LastRow,
        _ if is_plain_char(key, '0') => GridMove::FirstColumn,
        _ if is_plain_char(key, '$') => GridMove::LastColumn,
        _ => return None,
    };
    Some(grid_move)
}
