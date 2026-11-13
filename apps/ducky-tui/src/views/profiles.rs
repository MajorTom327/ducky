use ducky_app::{AppState, NoticeLevel, ProfilePicker};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::theme::{kind_badge, palette};
use crate::tui_state::TuiState;
use crate::widgets::loading::{LoadSlot, render_message};

const LIST_WIDTH: u16 = 60;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    picker: &ProfilePicker,
    tui: &mut TuiState,
) {
    let neutral_palette = palette(None);
    let list_height = u16::try_from(state.profiles.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2);
    let column = area.centered_horizontally(Constraint::Length(LIST_WIDTH.min(area.width)));
    let [list_area, info_area] = column.layout(&Layout::vertical([
        Constraint::Length(list_height.min(column.height.saturating_sub(3))),
        Constraint::Min(0),
    ]));
    let rows: Vec<Line> = state
        .profiles
        .iter()
        .enumerate()
        .map(|(profile_index, profile)| {
            let badge_style = Style::new().fg(palette(Some(profile.kind)).accent);
            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(kind_badge(profile.kind), badge_style),
                Span::raw(" "),
                Span::raw(profile.label.clone()),
            ]);
            match profile_index == picker.selected_profile_index {
                true => line.style(neutral_palette.selected(true)),
                false => line,
            }
        })
        .collect();
    let block = Block::bordered()
        .title(Span::styled(" Profiles ", neutral_palette.title()))
        .border_style(neutral_palette.border(true));
    let scroll = u16::try_from(
        picker
            .selected_profile_index
            .saturating_sub(usize::from(list_area.height.saturating_sub(3))),
    )
    .unwrap_or(0);
    frame.render_widget(
        Paragraph::new(rows).block(block).scroll((scroll, 0)),
        list_area,
    );
    let [message_area, _] = info_area.layout(&Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
    ]));
    if picker.connecting_profile_index.is_some() {
        render_message(frame, message_area, "Connecting…", &neutral_palette);
        tui.effects.mark_loading(LoadSlot::Main, message_area);
    } else if let Some(connection_error) = &picker.connection_error {
        let paragraph = Paragraph::new(connection_error.clone())
            .style(neutral_palette.notice(NoticeLevel::Error))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, info_area);
    }
}
