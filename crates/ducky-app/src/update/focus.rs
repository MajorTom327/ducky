use crate::command::Command;
use crate::msg::PaneDirection;
use crate::state::{AppState, Focus, QueryPane, TabContent};
use crate::update::workspace_mut;

pub(crate) fn toggle(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let current_focus = workspace.focus;
    let next_focus = match (workspace.active_content_mut(), current_focus) {
        (None, _) => Focus::Sidebar,
        (Some(TabContent::Query(query)), Focus::Sidebar) => {
            query.pane = QueryPane::Editor;
            Focus::Main
        }
        (Some(TabContent::Query(query)), Focus::Main) => match query.pane {
            QueryPane::Editor => {
                query.pane = QueryPane::Results;
                Focus::Main
            }
            QueryPane::Results => Focus::Sidebar,
        },
        (Some(_), Focus::Main) => Focus::Sidebar,
        (Some(_), Focus::Sidebar) => Focus::Main,
    };
    workspace.focus = next_focus;
    Vec::new()
}

pub(crate) fn move_towards(state: &mut AppState, direction: PaneDirection) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let is_main_focused = workspace.focus == Focus::Main;
    match (direction, workspace.active_content_mut()) {
        (PaneDirection::Left, _) => workspace.focus = Focus::Sidebar,
        (PaneDirection::Right, Some(_)) => workspace.focus = Focus::Main,
        (PaneDirection::Up, Some(TabContent::Query(query))) if is_main_focused => {
            query.pane = QueryPane::Editor;
        }
        (PaneDirection::Down, Some(TabContent::Query(query))) if is_main_focused => {
            query.pane = QueryPane::Results;
        }
        _ => {}
    }
    Vec::new()
}
