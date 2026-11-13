use ducky_app::{Loadable, NoticeLevel};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use crate::theme::Palette;
use crate::tui_state::TuiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadSlot {
    Sidebar,
    Main,
}

pub fn render_loading(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    slot: LoadSlot,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let line = Line::styled(format!("Loading {label}…"), palette.title());
    frame.render_widget(Paragraph::new(line).centered(), vertical_center(area));
    tui.effects.mark_loading(slot, area);
}

pub fn render_message(frame: &mut Frame, area: Rect, text: &str, palette: &Palette) {
    let paragraph = Paragraph::new(text.to_owned())
        .style(palette.muted())
        .centered()
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, vertical_center(area));
}

pub fn render_error(frame: &mut Frame, area: Rect, message: &str, palette: &Palette) {
    let paragraph = Paragraph::new(message.to_owned())
        .style(palette.notice(NoticeLevel::Error))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub fn render_loadable<'a, T>(
    frame: &mut Frame,
    area: Rect,
    loadable: &'a Loadable<T>,
    label: &str,
    slot: LoadSlot,
    tui: &mut TuiState,
    palette: &Palette,
) -> Option<&'a T> {
    match loadable {
        Loadable::Ready(value) => Some(value),
        Loadable::Loading => {
            render_loading(frame, area, label, slot, tui, palette);
            None
        }
        Loadable::Failed(message) => {
            render_error(frame, area, message, palette);
            None
        }
        Loadable::Idle => None,
    }
}

fn vertical_center(area: Rect) -> Rect {
    area.centered_vertically(Constraint::Length(area.height.min(1)))
}
