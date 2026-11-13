use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{Msg, ProfilesMsg};

use crate::keys::is_plain_char;

pub fn handle(key: KeyEvent) -> Vec<Msg> {
    let message = match key.code {
        KeyCode::Down => Msg::Profiles(ProfilesMsg::Next),
        KeyCode::Up => Msg::Profiles(ProfilesMsg::Previous),
        KeyCode::Enter => Msg::Profiles(ProfilesMsg::Connect),
        _ if is_plain_char(key, 'j') => Msg::Profiles(ProfilesMsg::Next),
        _ if is_plain_char(key, 'k') => Msg::Profiles(ProfilesMsg::Previous),
        _ if is_plain_char(key, 'q') => Msg::Quit,
        _ => return Vec::new(),
    };
    vec![message]
}
