use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::tui_state::TuiState;
use crate::views::{ViewContext, editor, results};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let [editor_area, results_area] = area.layout(&Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ]));
    editor::render(frame, editor_area, context, tui);
    results::render(frame, results_area, context, tui);
}
