pub mod cell_editor;
pub mod completion;
pub mod confirm;
pub mod dump;
pub mod editor;
pub mod files;
pub mod header;
pub mod help;
pub mod job_log;
pub mod modal;
pub mod palette;
pub mod parameters;
pub mod profile_picker;
pub mod profile_rows;
pub mod profiles;
pub mod query;
pub mod reference_picker;
pub mod restore;
pub mod results;
pub mod row_detail;
pub mod sidebar;
pub mod status;
pub mod structure;
pub mod table;
pub mod tabs;
pub mod welcome;

use ducky_app::{AppState, Screen, TabContent, TabId, TableView, Workspace};
use ducky_core::StoragePaths;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::layout::{FrameAreas, is_too_small, sidebar_width};
use crate::theme::{Palette, palette};
use crate::tui_state::{CursorShape, TuiState};
use crate::widgets::loading::render_message;

pub struct ViewContext<'a> {
    pub workspace: &'a Workspace,
    pub storage: &'a StoragePaths,
    pub palette: &'a Palette,
}

impl ViewContext<'_> {
    pub fn active_table(&self) -> Option<(TabId, &TableView)> {
        let tab = self.workspace.active_tab()?;
        match &tab.content {
            TabContent::Table(table) => Some((tab.id, table)),
            _ => None,
        }
    }
}

pub fn draw(frame: &mut Frame, state: &AppState, tui: &mut TuiState) {
    tui.effects.begin_frame();
    tui.cursor_shape = CursorShape::Bar;
    tui.mouse_targets.clear();
    let screen = frame.area();
    let workspace_palette = palette(state.workspace().map(|workspace| workspace.profile.kind));
    if is_too_small(screen) {
        render_message(
            frame,
            screen,
            "Terminal too small (minimum 80x20)",
            &workspace_palette,
        );
        return;
    }
    let sidebar_width = state.workspace().map_or(0, |workspace| {
        sidebar_width(&workspace.sidebar, screen.width)
    });
    let areas = FrameAreas::new(screen, sidebar_width);
    header::render(frame, areas.header, state, &workspace_palette);
    match &state.screen {
        Screen::Profiles(picker) => profiles::render(frame, areas.body(), state, picker, tui),
        Screen::Workspace(workspace) => {
            let context = ViewContext {
                workspace,
                storage: &state.storage,
                palette: &workspace_palette,
            };
            if !areas.sidebar.is_empty() {
                sidebar::render(frame, areas.sidebar, &context, tui);
            }
            tabs::render(frame, areas.tab_bar, &context, tui);
            render_main(frame, areas.main, &context, tui);
        }
    }
    status::render(
        frame,
        areas.status,
        state,
        tui.pending_prefix,
        &workspace_palette,
    );
    if let Some(modal) = &state.modal {
        fade(frame.buffer_mut());
        modal::render(frame, areas.screen, state, modal, tui, &workspace_palette);
    }
    tui.effects
        .apply(state, areas, &workspace_palette, frame.buffer_mut());
}

fn fade(buffer: &mut Buffer) {
    let faded_style = Style::new().add_modifier(Modifier::DIM);
    for cell in &mut buffer.content {
        cell.set_style(faded_style);
    }
}

fn render_main(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let Some(tab) = context.workspace.active_tab() else {
        welcome::render(frame, area, context, tui);
        return;
    };
    match &tab.content {
        TabContent::Table(table) => table::render(frame, area, context, tab.id, table, tui),
        TabContent::Query(query) => query::render(frame, area, context, tab.id, query, tui),
        TabContent::Files => files::render(frame, area, context, tui),
        TabContent::Dump => dump::render(frame, area, context, tui),
        TabContent::Restore => restore::render(frame, area, context, tui),
    }
}
