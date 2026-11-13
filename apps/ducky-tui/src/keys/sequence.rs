use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{ModalMsg, Msg, TabsMsg};
use tui_input::InputRequest;

use crate::keys::help::KeyHint;
use crate::tui_state::TuiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPrefix {
    Previous,
    Next,
    Leader,
    LeaderBuffer,
    LeaderQuit,
}

pub fn handle(tui: &mut TuiState, key: KeyEvent) -> Option<Vec<Msg>> {
    let character = match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(character)
        }
        _ => None,
    };
    let Some(prefix) = tui.pending_prefix.take() else {
        tui.pending_prefix = match character? {
            '[' => Some(KeyPrefix::Previous),
            ']' => Some(KeyPrefix::Next),
            ' ' => Some(KeyPrefix::Leader),
            ':' => {
                return Some(vec![
                    Msg::OpenPalette,
                    Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(':'))),
                ]);
            }
            _ => return None,
        };
        return Some(Vec::new());
    };
    let message = match (prefix, character) {
        (KeyPrefix::Previous, Some('b')) => Msg::Tabs(TabsMsg::Previous),
        (KeyPrefix::Next, Some('b')) => Msg::Tabs(TabsMsg::Next),
        (KeyPrefix::Leader, Some('b')) => {
            tui.pending_prefix = Some(KeyPrefix::LeaderBuffer);
            return Some(Vec::new());
        }
        (KeyPrefix::Leader, Some('q')) => {
            tui.pending_prefix = Some(KeyPrefix::LeaderQuit);
            return Some(Vec::new());
        }
        (KeyPrefix::LeaderBuffer, Some('d')) => Msg::Tabs(TabsMsg::Close),
        (KeyPrefix::LeaderQuit, Some('q')) => Msg::Quit,
        _ => return Some(Vec::new()),
    };
    Some(vec![message])
}

pub fn pending_hints(prefix: KeyPrefix) -> &'static [KeyHint] {
    match prefix {
        KeyPrefix::Previous => &[("[", "…"), ("b", "previous tab"), ("Esc", "cancel")],
        KeyPrefix::Next => &[("]", "…"), ("b", "next tab"), ("Esc", "cancel")],
        KeyPrefix::Leader => &[
            ("Space", "…"),
            ("b", "tabs"),
            ("q", "quit"),
            ("Esc", "cancel"),
        ],
        KeyPrefix::LeaderBuffer => &[("Space b", "…"), ("d", "close tab"), ("Esc", "cancel")],
        KeyPrefix::LeaderQuit => &[("Space q", "…"), ("q", "quit"), ("Esc", "cancel")],
    }
}
