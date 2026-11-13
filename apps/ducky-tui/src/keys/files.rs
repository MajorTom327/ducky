use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{FilesMsg, Msg, Workspace};

use crate::keys::{is_ctrl, is_plain_char};
use crate::tui_state::TuiState;

const PREVIEW_SCROLL_STEP: u16 = 10;

pub fn handle(_workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if is_ctrl(key, 'd') {
        tui.preview_scroll = tui.preview_scroll.saturating_add(PREVIEW_SCROLL_STEP);
        return Vec::new();
    }
    if is_ctrl(key, 'u') {
        tui.preview_scroll = tui.preview_scroll.saturating_sub(PREVIEW_SCROLL_STEP);
        return Vec::new();
    }
    let message = match key.code {
        KeyCode::Down => FilesMsg::Next,
        KeyCode::Up => FilesMsg::Previous,
        KeyCode::Enter => FilesMsg::Open,
        KeyCode::Esc => return vec![Msg::ToggleFocus],
        _ if is_plain_char(key, 'j') => FilesMsg::Next,
        _ if is_plain_char(key, 'k') => FilesMsg::Previous,
        _ if is_plain_char(key, 'x') => FilesMsg::Execute,
        _ if is_plain_char(key, 'r') => FilesMsg::Reload,
        _ => return Vec::new(),
    };
    if matches!(message, FilesMsg::Next | FilesMsg::Previous) {
        tui.preview_scroll = 0;
    }
    vec![Msg::Files(message)]
}
