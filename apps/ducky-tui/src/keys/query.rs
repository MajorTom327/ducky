use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{GridMove, Msg, QueryMsg, Workspace};
use edtui::EditorMode;

use crate::editor::completion::{accept_completion, refresh_completion};
use crate::editor::highlight::refresh_highlights;
use crate::keys::{is_ctrl, is_plain_char};
use crate::tui_state::TuiState;

pub fn handle_editor(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if key.code == KeyCode::F(5) || is_ctrl(key, 'e') {
        return vec![Msg::Query(QueryMsg::Run {
            sql: tui.editor.text(),
        })];
    }
    if is_ctrl(key, 's') {
        return vec![Msg::Query(QueryMsg::Save {
            content: tui.editor.text(),
        })];
    }
    if key.code == KeyCode::F(2) {
        return vec![Msg::Query(QueryMsg::SaveAs {
            content: tui.editor.text(),
        })];
    }
    if is_ctrl(key, 't') {
        return vec![Msg::Query(QueryMsg::ToggleTransactionMode)];
    }
    if is_ctrl(key, ' ') {
        refresh_completion(&mut tui.editor, workspace, true);
        return Vec::new();
    }
    if tui.editor.completion.is_some() && handle_completion_key(tui, key) {
        return Vec::new();
    }
    if tui.editor.state.mode == EditorMode::Normal
        && matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
    {
        return vec![Msg::ToggleFocus];
    }
    let text_before = tui.editor.text();
    tui.editor
        .event_handler
        .on_key_event(key, &mut tui.editor.state);
    let has_text_changed = tui.editor.text() != text_before;
    refresh_highlights(&mut tui.editor);
    match has_text_changed && tui.editor.is_insert_mode() {
        true => refresh_completion(&mut tui.editor, workspace, false),
        false => tui.editor.completion = None,
    }
    Vec::new()
}

pub fn handle_paste(_workspace: &Workspace, tui: &mut TuiState, pasted_text: String) -> Vec<Msg> {
    tui.editor
        .event_handler
        .on_paste_event(pasted_text, &mut tui.editor.state);
    tui.editor.completion = None;
    refresh_highlights(&mut tui.editor);
    Vec::new()
}

pub fn handle_results(_workspace: &Workspace, _tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if let Some(grid_move) = grid_move(key) {
        return vec![Msg::Query(QueryMsg::Move(grid_move))];
    }
    let query_msg = match key.code {
        KeyCode::Esc => return vec![Msg::ToggleFocus],
        KeyCode::Enter => QueryMsg::OpenRowDetail,
        _ if is_plain_char(key, '[') => QueryMsg::PreviousResult,
        _ if is_plain_char(key, ']') => QueryMsg::NextResult,
        _ if is_plain_char(key, 'y') => QueryMsg::CopyCell,
        _ => return Vec::new(),
    };
    vec![Msg::Query(query_msg)]
}

fn handle_completion_key(tui: &mut TuiState, key: KeyEvent) -> bool {
    let Some(popup) = tui.editor.completion.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Tab => {
            accept_completion(&mut tui.editor);
            refresh_highlights(&mut tui.editor);
        }
        KeyCode::Down => popup.select_next(),
        KeyCode::Up => popup.select_previous(),
        KeyCode::Esc => tui.editor.completion = None,
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
        _ if is_plain_char(key, 'g') => GridMove::FirstRow,
        _ if is_plain_char(key, 'G') => GridMove::LastRow,
        _ if is_plain_char(key, '0') => GridMove::FirstColumn,
        _ if is_plain_char(key, '$') => GridMove::LastColumn,
        _ => return None,
    };
    Some(grid_move)
}
