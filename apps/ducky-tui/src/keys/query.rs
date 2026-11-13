use crossterm::event::KeyEvent;
use ducky_app::{Msg, Workspace};

use crate::tui_state::TuiState;

pub fn handle_editor(_workspace: &Workspace, _tui: &mut TuiState, _key: KeyEvent) -> Vec<Msg> {
    Vec::new()
}

pub fn handle_results(_workspace: &Workspace, _tui: &mut TuiState, _key: KeyEvent) -> Vec<Msg> {
    Vec::new()
}

pub fn handle_paste(_workspace: &Workspace, _tui: &mut TuiState, _pasted_text: String) -> Vec<Msg> {
    Vec::new()
}
