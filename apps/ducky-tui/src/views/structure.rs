use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::loading::render_message;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, _tui: &mut TuiState) {
    render_message(frame, area, "Structure", context.palette);
}
