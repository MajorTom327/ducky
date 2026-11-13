use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{ModalMsg, Msg, PickerMode, ProfilePicker, ProfilesMsg};
use tui_input::InputRequest;

use crate::keys::{is_ctrl, is_plain_char};

pub fn handle(picker: &ProfilePicker, key: KeyEvent, is_modal: bool) -> Vec<Msg> {
    let message = match picker.mode {
        PickerMode::Filter => filter_message(key).map(Msg::Profiles),
        PickerMode::Navigate => navigate_message(picker, key, is_modal),
    };
    message.into_iter().collect()
}

fn filter_message(key: KeyEvent) -> Option<ProfilesMsg> {
    if is_ctrl(key, 'j') || is_ctrl(key, 'n') {
        return Some(ProfilesMsg::Next);
    }
    if is_ctrl(key, 'k') || is_ctrl(key, 'p') {
        return Some(ProfilesMsg::Previous);
    }
    if is_ctrl(key, 'w') {
        return Some(ProfilesMsg::Filter(InputRequest::DeletePrevWord));
    }
    if is_ctrl(key, 'u') {
        return Some(ProfilesMsg::Filter(InputRequest::DeleteLine));
    }
    match key.code {
        KeyCode::Down | KeyCode::Tab => Some(ProfilesMsg::Next),
        KeyCode::Up | KeyCode::BackTab => Some(ProfilesMsg::Previous),
        KeyCode::Enter => Some(ProfilesMsg::Connect),
        KeyCode::Esc => Some(ProfilesMsg::FocusList),
        KeyCode::Backspace => Some(ProfilesMsg::Filter(InputRequest::DeletePrevChar)),
        KeyCode::Char(character) if is_plain_char(key, character) => {
            Some(ProfilesMsg::Filter(InputRequest::InsertChar(character)))
        }
        _ => None,
    }
}

fn navigate_message(picker: &ProfilePicker, key: KeyEvent, is_modal: bool) -> Option<Msg> {
    let close_message = is_modal.then_some(Msg::Modal(ModalMsg::Cancel));
    let profiles_message = match key.code {
        KeyCode::Down => ProfilesMsg::Next,
        KeyCode::Up => ProfilesMsg::Previous,
        KeyCode::Enter => ProfilesMsg::Connect,
        KeyCode::Esc if !picker.filter.value().is_empty() => ProfilesMsg::ClearFilter,
        KeyCode::Esc => return close_message,
        _ if is_plain_char(key, 'j') => ProfilesMsg::Next,
        _ if is_plain_char(key, 'k') => ProfilesMsg::Previous,
        _ if is_plain_char(key, 'g') => ProfilesMsg::First,
        _ if is_plain_char(key, 'G') => ProfilesMsg::Last,
        _ if is_plain_char(key, '/') || is_plain_char(key, 'i') => ProfilesMsg::FocusFilter,
        _ if is_plain_char(key, 'q') => return close_message.or(Some(Msg::Quit)),
        _ => return None,
    };
    Some(Msg::Profiles(profiles_message))
}
