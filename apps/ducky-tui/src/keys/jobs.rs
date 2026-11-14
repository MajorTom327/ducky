use crossterm::event::{KeyCode, KeyEvent};
use ducky_app::{DumpMsg, Msg, RestoreMsg, TabContent, Workspace};

use crate::keys::{is_ctrl, is_plain_char};
use crate::tui_state::TuiState;

const LOG_SCROLL_STEP: u16 = 10;

pub fn handle(workspace: &Workspace, tui: &mut TuiState, key: KeyEvent) -> Vec<Msg> {
    if is_ctrl(key, 'u') || is_ctrl(key, 'd') {
        scroll_job_log(workspace, tui, is_ctrl(key, 'd'), LOG_SCROLL_STEP);
        return Vec::new();
    }
    if key.code == KeyCode::Esc {
        return vec![Msg::ToggleFocus];
    }
    let message = match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Dump) => dump_message(key),
        Some(TabContent::Restore) => restore_message(key).map(Msg::Restore),
        _ => None,
    };
    message.into_iter().collect()
}

pub fn scroll_job_log(workspace: &Workspace, tui: &mut TuiState, is_down: bool, line_count: u16) {
    let log_line_count = match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Restore) => workspace.restore.log_lines.len(),
        _ => workspace.dump.log_lines.len(),
    };
    let log_end = u16::try_from(log_line_count).unwrap_or(u16::MAX);
    tui.job_log_scroll = match is_down {
        true => tui
            .job_log_scroll
            .map(|offset| offset.saturating_add(line_count))
            .filter(|offset| *offset < log_end),
        false => Some(
            tui.job_log_scroll
                .unwrap_or(log_end)
                .saturating_sub(line_count),
        ),
    };
}

fn dump_message(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Enter => Some(Msg::Dump(DumpMsg::Start)),
        _ if is_plain_char(key, 'e') => Some(Msg::Dump(DumpMsg::EditFileName)),
        _ => None,
    }
}

fn restore_message(key: KeyEvent) -> Option<RestoreMsg> {
    let bindings = [
        ('j', RestoreMsg::Next),
        ('k', RestoreMsg::Previous),
        ('c', RestoreMsg::ToggleClean),
        ('o', RestoreMsg::ToggleSkipOwnership),
        ('r', RestoreMsg::Reload),
    ];
    match key.code {
        KeyCode::Enter => Some(RestoreMsg::Start),
        KeyCode::Down => Some(RestoreMsg::Next),
        KeyCode::Up => Some(RestoreMsg::Previous),
        _ => bindings
            .into_iter()
            .find(|(character, _)| is_plain_char(key, *character))
            .map(|(_, message)| message),
    }
}
