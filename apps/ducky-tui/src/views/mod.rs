pub mod completion;
pub mod dump;
pub mod editor;
pub mod files;
pub mod header;
pub mod job_log;
pub mod modal;
pub mod profiles;
pub mod query;
pub mod restore;
pub mod results;
pub mod sidebar;
pub mod status;
pub mod structure;
pub mod table;
pub mod welcome;

use ducky_app::{AppState, MainView, Screen, Workspace};
use ducky_core::StoragePaths;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::layout::{FrameAreas, is_too_small};
use crate::theme::{Palette, palette};
use crate::tui_state::TuiState;
use crate::widgets::loading::render_message;

pub struct ViewContext<'a> {
    pub workspace: &'a Workspace,
    pub storage: &'a StoragePaths,
    pub palette: &'a Palette,
}

pub fn draw(frame: &mut Frame, state: &AppState, tui: &mut TuiState) {
    tui.effects.begin_frame();
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
    let areas = FrameAreas::new(screen);
    header::render(frame, areas.header, state, &workspace_palette);
    match &state.screen {
        Screen::Profiles(picker) => profiles::render(frame, areas.body(), state, picker, tui),
        Screen::Workspace(workspace) => {
            let context = ViewContext {
                workspace,
                storage: &state.storage,
                palette: &workspace_palette,
            };
            sidebar::render(frame, areas.sidebar, &context, tui);
            render_main(frame, areas.main, &context, tui);
        }
    }
    status::render(frame, areas.status, state, &workspace_palette);
    if let Some(modal) = &state.modal {
        modal::render(frame, areas.screen, state, modal, tui, &workspace_palette);
    }
    tui.effects
        .apply(state, areas, &workspace_palette, frame.buffer_mut());
}

fn render_main(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    match context.workspace.view {
        MainView::Welcome => welcome::render(frame, area, context, tui),
        MainView::Table => table::render(frame, area, context, tui),
        MainView::Query => query::render(frame, area, context, tui),
        MainView::Files => files::render(frame, area, context, tui),
        MainView::Dump => dump::render(frame, area, context, tui),
        MainView::Restore => restore::render(frame, area, context, tui),
    }
}
