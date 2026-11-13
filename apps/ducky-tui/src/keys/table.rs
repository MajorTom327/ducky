use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{GridMove, Msg, TableMsg, TableTab, Workspace};

use crate::keys::is_plain_char;
use crate::tui_state::TuiState;

pub fn handle(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let Some(table) = &workspace.table else {
        return Vec::new();
    };
    let message = match table.tab {
        TableTab::Data => data_message(table.request.filter.is_some(), key),
        TableTab::Structure => structure_message(tui, key),
    };
    message.into_iter().collect()
}

fn data_message(has_filter: bool, key: KeyEvent) -> Option<Msg> {
    let table_message = match key.code {
        KeyCode::Left => TableMsg::Move(GridMove::Left),
        KeyCode::Right => TableMsg::Move(GridMove::Right),
        KeyCode::Down => TableMsg::Move(GridMove::Down),
        KeyCode::Up => TableMsg::Move(GridMove::Up),
        KeyCode::Enter => TableMsg::OpenRowDetail,
        KeyCode::Esc if has_filter => TableMsg::ClearFilter,
        KeyCode::Esc => return Some(Msg::ToggleFocus),
        _ => data_char_message(key)?,
    };
    Some(Msg::Table(table_message))
}

fn data_char_message(key: KeyEvent) -> Option<TableMsg> {
    let bindings = [
        ('h', TableMsg::Move(GridMove::Left)),
        ('l', TableMsg::Move(GridMove::Right)),
        ('j', TableMsg::Move(GridMove::Down)),
        ('k', TableMsg::Move(GridMove::Up)),
        ('g', TableMsg::Move(GridMove::FirstRow)),
        ('G', TableMsg::Move(GridMove::LastRow)),
        ('0', TableMsg::Move(GridMove::FirstColumn)),
        ('$', TableMsg::Move(GridMove::LastColumn)),
        ('n', TableMsg::NextPage),
        (']', TableMsg::NextPage),
        ('p', TableMsg::PreviousPage),
        ('[', TableMsg::PreviousPage),
        ('o', TableMsg::CycleSort),
        ('/', TableMsg::StartFilter),
        ('y', TableMsg::CopyCell),
        ('r', TableMsg::Reload),
        ('t', TableMsg::ToggleTab),
    ];
    bindings
        .into_iter()
        .find(|(character, _)| is_plain_char(key, *character))
        .map(|(_, message)| message)
}

fn structure_message(tui: &mut TuiState, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => return Some(Msg::ToggleFocus),
        KeyCode::Down => tui.structure_scroll = tui.structure_scroll.saturating_add(1),
        KeyCode::Up => tui.structure_scroll = tui.structure_scroll.saturating_sub(1),
        _ if is_plain_char(key, 'j') => {
            tui.structure_scroll = tui.structure_scroll.saturating_add(1)
        }
        _ if is_plain_char(key, 'k') => {
            tui.structure_scroll = tui.structure_scroll.saturating_sub(1)
        }
        _ if is_plain_char(key, 'g') => tui.structure_scroll = 0,
        _ if is_plain_char(key, 't') => return Some(Msg::Table(TableMsg::ToggleTab)),
        _ if is_plain_char(key, 'r') => return Some(Msg::Table(TableMsg::Reload)),
        _ => {}
    }
    None
}
