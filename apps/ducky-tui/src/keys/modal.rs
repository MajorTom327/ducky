use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{Modal, ModalMsg, Msg};
use tui_input::InputRequest;

use crate::keys::{is_ctrl, is_plain_char, profiles};
use crate::tui_state::TuiState;

pub const HELP_HALF_PAGE: u16 = 10;

pub fn handle(modal: &Modal, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let modal_message = match modal {
        Modal::ProfilePicker(picker) => return profiles::handle(picker, key, true),
        Modal::Prompt(_) => prompt_message(key),
        Modal::Confirm(_) => confirm_message(key),
        Modal::RowDetail(_) => row_detail_message(key),
        Modal::SchemaPicker { .. } | Modal::ReferencePicker { .. } => picker_message(key),
        Modal::Help => help_message(tui, key),
        Modal::Palette(_) => palette_message(key),
        Modal::Parameters(_) => parameters_message(key),
    };
    modal_message.map(Msg::Modal).into_iter().collect()
}

fn prompt_message(key: KeyEvent) -> Option<ModalMsg> {
    if is_ctrl(key, 'w') {
        return Some(ModalMsg::Input(InputRequest::DeletePrevWord));
    }
    if is_ctrl(key, 'u') {
        return Some(ModalMsg::Input(InputRequest::DeleteLine));
    }
    let input_request = match key.code {
        KeyCode::Enter => return Some(ModalMsg::Submit),
        KeyCode::Esc => return Some(ModalMsg::Cancel),
        KeyCode::Backspace => InputRequest::DeletePrevChar,
        KeyCode::Delete => InputRequest::DeleteNextChar,
        KeyCode::Left => InputRequest::GoToPrevChar,
        KeyCode::Right => InputRequest::GoToNextChar,
        KeyCode::Home => InputRequest::GoToStart,
        KeyCode::End => InputRequest::GoToEnd,
        KeyCode::Char(character) if is_plain_char(key, character) => {
            InputRequest::InsertChar(character)
        }
        _ => return None,
    };
    Some(ModalMsg::Input(input_request))
}

fn palette_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Down | KeyCode::Tab => Some(ModalMsg::Next),
        KeyCode::Up | KeyCode::BackTab => Some(ModalMsg::Previous),
        _ if is_ctrl(key, 'n') => Some(ModalMsg::Next),
        _ if is_ctrl(key, 'p') => Some(ModalMsg::Previous),
        _ => prompt_message(key),
    }
}

fn parameters_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Down | KeyCode::Tab => Some(ModalMsg::Next),
        KeyCode::Up | KeyCode::BackTab => Some(ModalMsg::Previous),
        _ if is_ctrl(key, 'r') => Some(ModalMsg::ToggleRaw),
        _ => prompt_message(key),
    }
}

fn confirm_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Enter => Some(ModalMsg::Submit),
        KeyCode::Esc => Some(ModalMsg::Cancel),
        _ if is_plain_char(key, 'y') => Some(ModalMsg::Submit),
        _ if is_plain_char(key, 'n') => Some(ModalMsg::Cancel),
        _ => None,
    }
}

fn row_detail_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Down => Some(ModalMsg::Next),
        KeyCode::Up => Some(ModalMsg::Previous),
        KeyCode::Esc => Some(ModalMsg::Cancel),
        _ if is_plain_char(key, 'j') => Some(ModalMsg::Next),
        _ if is_plain_char(key, 'k') => Some(ModalMsg::Previous),
        _ if is_plain_char(key, 'y') => Some(ModalMsg::CopyValue),
        _ if is_plain_char(key, 'q') => Some(ModalMsg::Cancel),
        _ => None,
    }
}

fn picker_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Down => Some(ModalMsg::Next),
        KeyCode::Up => Some(ModalMsg::Previous),
        KeyCode::Enter => Some(ModalMsg::Submit),
        KeyCode::Esc => Some(ModalMsg::Cancel),
        _ if is_plain_char(key, 'j') => Some(ModalMsg::Next),
        _ if is_plain_char(key, 'k') => Some(ModalMsg::Previous),
        _ => None,
    }
}

fn help_message(tui: &mut TuiState, key: KeyEvent) -> Option<ModalMsg> {
    let is_close_key =
        key.code == KeyCode::Esc || is_plain_char(key, 'q') || is_plain_char(key, '?');
    if is_close_key {
        tui.help_scroll = 0;
        return Some(ModalMsg::Cancel);
    }
    match key.code {
        KeyCode::Down => tui.help_scroll = tui.help_scroll.saturating_add(1),
        KeyCode::Up => tui.help_scroll = tui.help_scroll.saturating_sub(1),
        _ if is_plain_char(key, 'j') => tui.help_scroll = tui.help_scroll.saturating_add(1),
        _ if is_plain_char(key, 'k') => tui.help_scroll = tui.help_scroll.saturating_sub(1),
        _ if is_plain_char(key, 'g') => tui.help_scroll = 0,
        _ if is_ctrl(key, 'd') => tui.help_scroll = tui.help_scroll.saturating_add(HELP_HALF_PAGE),
        _ if is_ctrl(key, 'u') => tui.help_scroll = tui.help_scroll.saturating_sub(HELP_HALF_PAGE),
        _ => {}
    }
    None
}
