use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{Modal, ModalMsg, Msg};
use tui_input::InputRequest;

use crate::keys::{is_ctrl, is_plain_char};

pub fn handle(modal: &Modal, key: KeyEvent) -> Vec<Msg> {
    let modal_message = match modal {
        Modal::Prompt(_) => prompt_message(key),
        Modal::Confirm(_) => confirm_message(key),
        Modal::RowDetail(_) => row_detail_message(key),
        Modal::SchemaPicker { .. } => schema_picker_message(key),
        Modal::Help => help_message(key),
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

fn schema_picker_message(key: KeyEvent) -> Option<ModalMsg> {
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

fn help_message(key: KeyEvent) -> Option<ModalMsg> {
    match key.code {
        KeyCode::Esc => Some(ModalMsg::Cancel),
        _ if is_plain_char(key, 'q') || is_plain_char(key, '?') => Some(ModalMsg::Cancel),
        _ => None,
    }
}
