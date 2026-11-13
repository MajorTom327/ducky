use ducky_app::TableTab;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui_state::TuiState;
use crate::views::{ViewContext, structure};
use crate::widgets::loading::render_message;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let Some(table) = &context.workspace.table else {
        return;
    };
    match table.tab {
        TableTab::Data => render_message(frame, area, "Table", context.palette),
        TableTab::Structure => structure::render(frame, area, context, tui),
    }
}
