use ducky_app::TabId;
use ratatui::layout::{Position, Rect};

use crate::keys::sequence::GridPane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollArea {
    Structure(TabId),
    Help,
    FilePreview,
    JobLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseTarget {
    SidebarItem(usize),
    SidebarList,
    SidebarBorder,
    Tab(usize),
    GridCell {
        pane: GridPane,
        row_index: usize,
        column_index: usize,
    },
    GridHeader {
        pane: GridPane,
        column_index: usize,
    },
    Grid(GridPane),
    Editor(TabId),
    Scroll(ScrollArea),
    ModalItem(usize),
    Modal,
    ProfileRow(usize),
    FileItem(usize),
}

pub type MouseTargets = Vec<(Rect, MouseTarget)>;

pub fn list_row_targets(
    area: Rect,
    first_index: usize,
    item_count: usize,
    target: fn(usize) -> MouseTarget,
) -> impl Iterator<Item = (Rect, MouseTarget)> {
    area.rows()
        .zip(first_index..item_count)
        .map(move |(row_area, item_index)| (row_area, target(item_index)))
}

pub fn topmost_target(
    mouse_targets: &[(Rect, MouseTarget)],
    position: Position,
) -> Option<MouseTarget> {
    mouse_targets
        .iter()
        .rev()
        .find(|(area, _)| area.contains(position))
        .map(|(_, target)| *target)
}

pub fn is_inside_modal(mouse_targets: &[(Rect, MouseTarget)], position: Position) -> bool {
    mouse_targets
        .iter()
        .any(|(area, target)| *target == MouseTarget::Modal && area.contains(position))
}

pub fn target_area(mouse_targets: &[(Rect, MouseTarget)], wanted: MouseTarget) -> Option<Rect> {
    mouse_targets
        .iter()
        .find(|(_, target)| *target == wanted)
        .map(|(area, _)| *area)
}
