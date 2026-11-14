use std::iter::repeat_n;

use crossterm::event::MouseEvent;
use ducky_app::{
    FilesMsg, GridCursor, GridMove, Msg, PaneDirection, QueryMsg, RestoreMsg, SidebarMsg,
    TabContent, TabId, TableMsg, TabsMsg, Workspace,
};

use crate::keys::focused_editor_tab;
use crate::keys::jobs::scroll_job_log;
use crate::keys::sequence::GridPane;
use crate::mouse::{MouseTarget, ScrollArea, WHEEL_LINE_COUNT, scroll_lines};
use crate::tui_state::TuiState;

pub(super) fn click(
    workspace: &Workspace,
    tui: &mut TuiState,
    target: MouseTarget,
    mouse_event: MouseEvent,
    is_double_click: bool,
) -> Vec<Msg> {
    match target {
        MouseTarget::SidebarItem(item_index) => vec![
            Msg::Sidebar(SidebarMsg::Select(item_index)),
            Msg::Sidebar(SidebarMsg::Activate),
        ],
        MouseTarget::SidebarList => vec![Msg::FocusPane(PaneDirection::Left)],
        MouseTarget::SidebarBorder => {
            tui.is_dragging_sidebar = true;
            Vec::new()
        }
        MouseTarget::Tab(tab_index) => vec![Msg::Tabs(TabsMsg::Select(tab_index))],
        MouseTarget::GridCell {
            pane,
            row_index,
            column_index,
        } => cell_click(
            pane,
            GridCursor {
                row_index,
                column_index,
            },
            is_double_click,
        ),
        MouseTarget::GridHeader {
            pane: GridPane::TableData,
            column_index,
        } => header_click(workspace, column_index),
        MouseTarget::GridHeader { pane, .. } | MouseTarget::Grid(pane) => focus_grid(pane),
        MouseTarget::Editor(tab_id) => {
            forward_to_editor(tui, tab_id, mouse_event);
            match focused_editor_tab(workspace) == Some(tab_id) {
                true => Vec::new(),
                false => vec![
                    Msg::FocusPane(PaneDirection::Right),
                    Msg::FocusPane(PaneDirection::Up),
                ],
            }
        }
        MouseTarget::Scroll(_) => vec![Msg::FocusPane(PaneDirection::Right)],
        MouseTarget::FileItem(entry_index) => {
            file_click(workspace, tui, entry_index, is_double_click)
        }
        MouseTarget::ModalItem(_) | MouseTarget::Modal | MouseTarget::ProfileRow(_) => Vec::new(),
    }
}

pub(super) fn wheel(
    workspace: &Workspace,
    tui: &mut TuiState,
    target: MouseTarget,
    mouse_event: MouseEvent,
    is_down: bool,
) -> Vec<Msg> {
    match target {
        MouseTarget::SidebarItem(_) | MouseTarget::SidebarList => {
            let sidebar_msg = match is_down {
                true => SidebarMsg::Next,
                false => SidebarMsg::Previous,
            };
            repeated(Msg::Sidebar(sidebar_msg))
        }
        MouseTarget::GridCell { pane, .. }
        | MouseTarget::GridHeader { pane, .. }
        | MouseTarget::Grid(pane) => {
            let grid_move = match is_down {
                true => GridMove::Down,
                false => GridMove::Up,
            };
            repeated(match pane {
                GridPane::TableData => Msg::Table(TableMsg::Move(grid_move)),
                GridPane::Results => Msg::Query(QueryMsg::Move(grid_move)),
            })
        }
        MouseTarget::Editor(tab_id) => {
            forward_to_editor(tui, tab_id, mouse_event);
            Vec::new()
        }
        MouseTarget::Scroll(scroll_area) => {
            scroll(workspace, tui, scroll_area, is_down);
            Vec::new()
        }
        MouseTarget::FileItem(_) => file_wheel(workspace, tui, is_down),
        MouseTarget::SidebarBorder
        | MouseTarget::Tab(_)
        | MouseTarget::ModalItem(_)
        | MouseTarget::Modal
        | MouseTarget::ProfileRow(_) => Vec::new(),
    }
}

pub(super) fn forward_to_editor(tui: &mut TuiState, tab_id: TabId, mouse_event: MouseEvent) {
    let editor = tui.editor(tab_id);
    editor.completion = None;
    editor
        .event_handler
        .on_mouse_event(mouse_event, &mut editor.state);
}

fn repeated(message: Msg) -> Vec<Msg> {
    repeat_n(message, usize::from(WHEEL_LINE_COUNT)).collect()
}

fn cell_click(pane: GridPane, cell: GridCursor, is_double_click: bool) -> Vec<Msg> {
    let (select_message, double_click_message) = match pane {
        GridPane::TableData => (
            Msg::Table(TableMsg::SelectCell(cell)),
            Msg::Table(TableMsg::ChangeCell),
        ),
        GridPane::Results => (
            Msg::Query(QueryMsg::SelectCell(cell)),
            Msg::Query(QueryMsg::OpenRowDetail),
        ),
    };
    let mut messages = vec![select_message];
    if is_double_click {
        messages.push(double_click_message);
    }
    messages
}

fn header_click(workspace: &Workspace, column_index: usize) -> Vec<Msg> {
    let cursor_row_index = match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Table(table)) => table.cursor.row_index,
        _ => return Vec::new(),
    };
    let header_cell = GridCursor {
        row_index: cursor_row_index,
        column_index,
    };
    vec![
        Msg::Table(TableMsg::SelectCell(header_cell)),
        Msg::Table(TableMsg::CycleSort),
    ]
}

fn focus_grid(pane: GridPane) -> Vec<Msg> {
    let mut messages = vec![Msg::FocusPane(PaneDirection::Right)];
    if pane == GridPane::Results {
        messages.push(Msg::FocusPane(PaneDirection::Down));
    }
    messages
}

fn file_click(
    workspace: &Workspace,
    tui: &mut TuiState,
    entry_index: usize,
    is_double_click: bool,
) -> Vec<Msg> {
    let mut messages = vec![Msg::FocusPane(PaneDirection::Right)];
    match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Files) => {
            if entry_index != workspace.files.selected_entry_index {
                tui.preview_scroll = 0;
            }
            messages.push(Msg::Files(FilesMsg::Select(entry_index)));
            if is_double_click {
                messages.push(Msg::Files(FilesMsg::Open));
            }
        }
        Some(TabContent::Restore) => {
            messages.push(Msg::Restore(RestoreMsg::Select(entry_index)));
            if is_double_click {
                messages.push(Msg::Restore(RestoreMsg::Start));
            }
        }
        _ => {}
    }
    messages
}

fn file_wheel(workspace: &Workspace, tui: &mut TuiState, is_down: bool) -> Vec<Msg> {
    match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Files) => {
            tui.preview_scroll = 0;
            vec![Msg::Files(match is_down {
                true => FilesMsg::Next,
                false => FilesMsg::Previous,
            })]
        }
        Some(TabContent::Restore) => vec![Msg::Restore(match is_down {
            true => RestoreMsg::Next,
            false => RestoreMsg::Previous,
        })],
        _ => Vec::new(),
    }
}

fn scroll(workspace: &Workspace, tui: &mut TuiState, scroll_area: ScrollArea, is_down: bool) {
    match scroll_area {
        ScrollArea::Structure(tab_id) => {
            scroll_lines(tui.structure_scrolls.entry(tab_id).or_default(), is_down);
        }
        ScrollArea::Help => scroll_lines(&mut tui.help_scroll, is_down),
        ScrollArea::FilePreview => scroll_lines(&mut tui.preview_scroll, is_down),
        ScrollArea::JobLog => scroll_job_log(workspace, tui, is_down, WHEEL_LINE_COUNT),
    }
}
