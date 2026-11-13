use ducky_app::{JobState, NoticeLevel};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::widgets::loading::{LoadSlot, render_error, render_loading};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    log_lines: &[String],
    scroll: Option<u16>,
    palette: &Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border(false))
        .title(Line::styled("Log", palette.title()));
    let visible_height = usize::from(block.inner(area).height);
    let max_offset = log_lines.len().saturating_sub(visible_height);
    let first_line_index = match scroll {
        Some(offset) => usize::from(offset).min(max_offset),
        None => max_offset,
    };
    let lines: Vec<Line> = log_lines
        .iter()
        .skip(first_line_index)
        .take(visible_height)
        .map(|line| Line::raw(line.clone()))
        .collect();
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

pub fn render_job_state(
    frame: &mut Frame,
    area: Rect,
    job: &JobState,
    idle_hint: &str,
    running_label: &str,
    tui: &mut TuiState,
    palette: &Palette,
) {
    match job {
        JobState::Idle => frame.render_widget(
            Paragraph::new(idle_hint.to_owned()).style(palette.muted()),
            area,
        ),
        JobState::Running => {
            render_loading(frame, area, running_label, LoadSlot::Main, tui, palette)
        }
        JobState::Succeeded(summary) => frame.render_widget(
            Paragraph::new(summary.clone()).style(palette.notice(NoticeLevel::Success)),
            area,
        ),
        JobState::Failed(message) => render_error(frame, area, message, palette),
    }
}

pub fn disabled_lines(action: &str, reason: &str) -> Vec<Line<'static>> {
    vec![
        Line::raw(format!("{action} is disabled: {reason}.")),
        Line::raw(""),
        Line::raw(
            "Install the PostgreSQL client tools (macOS: brew install libpq, Debian/Ubuntu: apt install postgresql-client) and restart ducky.",
        ),
    ]
}
