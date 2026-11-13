use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{Msg, SidebarMsg, Workspace};

use crate::keys::is_plain_char;

pub fn handle(workspace: &Workspace, key: KeyEvent) -> Vec<Msg> {
    let message = match key.code {
        KeyCode::Down => Msg::Sidebar(SidebarMsg::Next),
        KeyCode::Up => Msg::Sidebar(SidebarMsg::Previous),
        KeyCode::Enter | KeyCode::Right => Msg::Sidebar(SidebarMsg::Activate),
        KeyCode::Esc if !workspace.sidebar.filter.is_empty() => {
            Msg::Sidebar(SidebarMsg::ClearFilter)
        }
        _ => match char_message(key) {
            Some(message) => message,
            None => return Vec::new(),
        },
    };
    vec![message]
}

fn char_message(key: KeyEvent) -> Option<Msg> {
    let bindings = [
        ('j', Msg::Sidebar(SidebarMsg::Next)),
        ('k', Msg::Sidebar(SidebarMsg::Previous)),
        ('g', Msg::Sidebar(SidebarMsg::First)),
        ('G', Msg::Sidebar(SidebarMsg::Last)),
        ('l', Msg::Sidebar(SidebarMsg::Activate)),
        ('/', Msg::Sidebar(SidebarMsg::StartFilter)),
        ('s', Msg::OpenSchemaPicker),
        ('r', Msg::RefreshCatalog),
        ('R', Msg::Reconnect),
        ('D', Msg::Disconnect),
    ];
    bindings
        .into_iter()
        .find(|(character, _)| is_plain_char(key, *character))
        .map(|(_, message)| message)
}
