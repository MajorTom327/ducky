use crossterm::event::KeyEvent;
use ducky_app::{Msg, Workspace};

use crate::tui_state::TuiState;

pub fn handle(_workspace: &Workspace, _tui: &mut TuiState, _key: KeyEvent) -> Vec<Msg> {
    Vec::new()
}
