mod modal;
mod targets;
mod workspace;

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ducky_app::{AppState, Msg, ProfilesMsg, Screen, SidebarMsg};
use ratatui::layout::Position;

pub use targets::{
    MouseTarget, MouseTargets, ScrollArea, is_inside_modal, list_row_targets, target_area,
    topmost_target,
};

use crate::tui_state::TuiState;

pub const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(400);
pub const WHEEL_LINE_COUNT: u16 = 3;

pub fn handle_mouse(state: &AppState, tui: &mut TuiState, mouse_event: MouseEvent) -> Vec<Msg> {
    let mut messages = route_mouse(state, tui, mouse_event);
    if state.notice.is_some() && matches!(mouse_event.kind, MouseEventKind::Down(_)) {
        messages.insert(0, Msg::DismissNotice);
    }
    messages
}

fn route_mouse(state: &AppState, tui: &mut TuiState, mouse_event: MouseEvent) -> Vec<Msg> {
    let position = Position::new(mouse_event.column, mouse_event.row);
    let target = topmost_target(&tui.mouse_targets, position);
    match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => press(state, tui, target, mouse_event),
        MouseEventKind::Drag(MouseButton::Left) => drag(state, tui, target, mouse_event),
        MouseEventKind::Up(MouseButton::Left) => release(state, tui, target, mouse_event),
        MouseEventKind::ScrollDown => wheel(state, tui, target, mouse_event, true),
        MouseEventKind::ScrollUp => wheel(state, tui, target, mouse_event, false),
        _ => Vec::new(),
    }
}

fn press(
    state: &AppState,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    mouse_event: MouseEvent,
) -> Vec<Msg> {
    let is_double_click = register_click(tui, target);
    if let Some(open_modal) = &state.modal {
        let position = Position::new(mouse_event.column, mouse_event.row);
        let is_inside = is_inside_modal(&tui.mouse_targets, position);
        return modal::click(open_modal, tui, target, is_inside);
    }
    let Some(target) = target else {
        return Vec::new();
    };
    match &state.screen {
        Screen::Profiles(_) => match target {
            MouseTarget::ProfileRow(profile_index) => vec![
                Msg::Profiles(ProfilesMsg::Select(profile_index)),
                Msg::Profiles(ProfilesMsg::Connect),
            ],
            _ => Vec::new(),
        },
        Screen::Workspace(open_workspace) => {
            workspace::click(open_workspace, tui, target, mouse_event, is_double_click)
        }
    }
}

fn register_click(tui: &mut TuiState, target: Option<MouseTarget>) -> bool {
    let clicked_at = Instant::now();
    let is_double_click = match (tui.last_click, target) {
        (Some((last_target, last_clicked_at)), Some(target)) => {
            last_target == target
                && clicked_at.duration_since(last_clicked_at) < DOUBLE_CLICK_INTERVAL
        }
        _ => false,
    };
    tui.last_click = match is_double_click {
        true => None,
        false => target.map(|target| (target, clicked_at)),
    };
    is_double_click
}

fn drag(
    state: &AppState,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    mouse_event: MouseEvent,
) -> Vec<Msg> {
    if tui.is_dragging_sidebar {
        let sidebar_x = target_area(&tui.mouse_targets, MouseTarget::SidebarList)
            .map_or(0, |sidebar_area| sidebar_area.x);
        let width = mouse_event
            .column
            .saturating_sub(sidebar_x)
            .saturating_add(1);
        return vec![Msg::Sidebar(SidebarMsg::SetWidth(width))];
    }
    forward_to_editor(state, tui, target, mouse_event);
    Vec::new()
}

fn release(
    state: &AppState,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    mouse_event: MouseEvent,
) -> Vec<Msg> {
    if tui.is_dragging_sidebar {
        tui.is_dragging_sidebar = false;
        return Vec::new();
    }
    forward_to_editor(state, tui, target, mouse_event);
    Vec::new()
}

fn forward_to_editor(
    state: &AppState,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    mouse_event: MouseEvent,
) {
    if state.modal.is_none()
        && let Some(MouseTarget::Editor(tab_id)) = target
    {
        workspace::forward_to_editor(tui, tab_id, mouse_event);
    }
}

fn wheel(
    state: &AppState,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    mouse_event: MouseEvent,
    is_down: bool,
) -> Vec<Msg> {
    if state.modal.is_some() {
        let position = Position::new(mouse_event.column, mouse_event.row);
        let is_inside = is_inside_modal(&tui.mouse_targets, position);
        return modal::wheel(tui, target, is_inside, is_down);
    }
    let Some(target) = target else {
        return Vec::new();
    };
    match &state.screen {
        Screen::Profiles(_) => match target {
            MouseTarget::ProfileRow(_) => vec![Msg::Profiles(match is_down {
                true => ProfilesMsg::Next,
                false => ProfilesMsg::Previous,
            })],
            _ => Vec::new(),
        },
        Screen::Workspace(open_workspace) => {
            workspace::wheel(open_workspace, tui, target, mouse_event, is_down)
        }
    }
}

pub fn scroll_lines(scroll: &mut u16, is_down: bool) {
    *scroll = match is_down {
        true => scroll.saturating_add(WHEEL_LINE_COUNT),
        false => scroll.saturating_sub(WHEEL_LINE_COUNT),
    };
}
