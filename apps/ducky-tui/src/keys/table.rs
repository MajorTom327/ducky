use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{GridMove, Msg, TabContent, TableMsg, TableTab, TableView, Workspace};

use crate::keys::{is_ctrl, is_plain_char};
use crate::tui_state::TuiState;

pub fn handle(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    let Some(tab) = workspace.active_tab() else {
        return Vec::new();
    };
    let TabContent::Table(table) = &tab.content else {
        return Vec::new();
    };
    let message = match table.tab {
        TableTab::Data => data_message(table, key),
        TableTab::Structure => {
            structure_message(tui.structure_scrolls.entry(tab.id).or_default(), key)
        }
    };
    message.into_iter().collect()
}

fn data_message(table: &TableView, key: KeyEvent) -> Option<Msg> {
    let table_message = match key.code {
        KeyCode::Left => TableMsg::Move(GridMove::Left),
        KeyCode::Right => TableMsg::Move(GridMove::Right),
        KeyCode::Down => TableMsg::Move(GridMove::Down),
        KeyCode::Up => TableMsg::Move(GridMove::Up),
        KeyCode::Enter => TableMsg::OpenRowDetail,
        KeyCode::Esc if table.has_selection() => TableMsg::ClearSelection,
        KeyCode::Esc if table.request.filter.is_some() => TableMsg::ClearFilter,
        KeyCode::Esc => return Some(Msg::ToggleFocus),
        _ if is_ctrl(key, 's') => TableMsg::SaveDrafts,
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
        ('G', TableMsg::Move(GridMove::LastRow)),
        ('0', TableMsg::Move(GridMove::FirstColumn)),
        ('$', TableMsg::Move(GridMove::LastColumn)),
        ('n', TableMsg::NextPage),
        ('p', TableMsg::PreviousPage),
        ('s', TableMsg::CycleSort),
        ('/', TableMsg::StartFilter),
        ('y', TableMsg::CopyCell),
        ('r', TableMsg::Reload),
        ('t', TableMsg::ToggleTab),
        ('v', TableMsg::ToggleRowMark),
        ('V', TableMsg::ToggleVisualSelection),
        ('e', TableMsg::EditSelectedRows),
        ('d', TableMsg::DeleteSelectedRows),
        ('c', TableMsg::ChangeCell),
        ('x', TableMsg::SetCellNull),
        ('u', TableMsg::RevertCell),
        ('U', TableMsg::DiscardDrafts),
    ];
    bindings
        .into_iter()
        .find(|(character, _)| is_plain_char(key, *character))
        .map(|(_, message)| message)
}

fn structure_message(structure_scroll: &mut u16, key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => return Some(Msg::ToggleFocus),
        KeyCode::Down => *structure_scroll = structure_scroll.saturating_add(1),
        KeyCode::Up => *structure_scroll = structure_scroll.saturating_sub(1),
        _ if is_plain_char(key, 'j') => *structure_scroll = structure_scroll.saturating_add(1),
        _ if is_plain_char(key, 'k') => *structure_scroll = structure_scroll.saturating_sub(1),
        _ if is_plain_char(key, 'g') => *structure_scroll = 0,
        _ if is_plain_char(key, 't') => return Some(Msg::Table(TableMsg::ToggleTab)),
        _ if is_plain_char(key, 'r') => return Some(Msg::Table(TableMsg::Reload)),
        _ => {}
    }
    None
}
