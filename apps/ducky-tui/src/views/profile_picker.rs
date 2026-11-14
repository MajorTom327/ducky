use ducky_app::{NoticeLevel, PickerMode, ProfilePicker};
use ducky_core::Profile;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::mouse::MouseTarget;
use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::profile_rows::{first_visible_row_position, row_lines, row_targets};
use crate::widgets::loading::LoadSlot;

const INPUT_PREFIX: &str = " › ";
const FILTER_PLACEHOLDER: &str = "type to filter";
const CHROME_ROWS: u16 = 3;

pub struct PickerView<'a> {
    pub profiles: &'a [Profile],
    pub picker: &'a ProfilePicker,
    pub connected_profile_index: Option<usize>,
    pub row_target: fn(usize) -> MouseTarget,
}

pub fn content_height(profiles: &[Profile], picker: &ProfilePicker, width: u16) -> u16 {
    let row_count = u16::try_from(picker.rows(profiles).len().max(1)).unwrap_or(u16::MAX);
    row_count
        .saturating_add(CHROME_ROWS)
        .saturating_add(error_height(picker, width))
}

pub fn fitted_area(container: Rect, profiles: &[Profile], picker: &ProfilePicker) -> Rect {
    let column = container.centered_horizontally(Constraint::Percentage(70));
    let inner_width = column.width.saturating_sub(2);
    let height = content_height(profiles, picker, inner_width)
        .saturating_add(2)
        .min(container.height);
    column.centered_vertically(Constraint::Length(height))
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    view: &PickerView,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let PickerView {
        profiles, picker, ..
    } = *view;
    let [input_area, rule_area, list_area, footer_area, error_area] =
        area.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(error_height(picker, area.width)),
        ]));
    render_input(frame, input_area, picker, palette);
    frame.render_widget(
        Paragraph::new(Line::styled(
            "─".repeat(usize::from(rule_area.width)),
            palette.muted(),
        )),
        rule_area,
    );
    let rows = picker.rows(profiles);
    let profile_lines = row_lines(
        &rows,
        profiles,
        picker,
        view.connected_profile_index,
        list_area,
        palette,
    );
    frame.render_widget(Paragraph::new(profile_lines), list_area);
    let first_row_position =
        first_visible_row_position(&rows, profiles, picker, usize::from(list_area.height));
    tui.mouse_targets.extend(row_targets(
        list_area,
        &rows,
        first_row_position,
        view.row_target,
    ));
    render_footer(frame, footer_area, profiles, picker, tui, palette);
    if let Some(connection_error) = &picker.connection_error {
        frame.render_widget(
            Paragraph::new(connection_error.clone())
                .style(palette.notice(NoticeLevel::Error))
                .wrap(Wrap { trim: false }),
            error_area.inner(Margin::new(1, 0)),
        );
    }
}

fn error_height(picker: &ProfilePicker, width: u16) -> u16 {
    let Some(connection_error) = &picker.connection_error else {
        return 0;
    };
    let line_width = usize::from(width.saturating_sub(2).max(1));
    let wrapped_line_count: usize = connection_error
        .lines()
        .map(|line| line.chars().count().div_ceil(line_width).max(1))
        .sum();
    u16::try_from(wrapped_line_count.max(1)).unwrap_or(u16::MAX)
}

fn render_input(frame: &mut Frame, area: Rect, picker: &ProfilePicker, palette: &Palette) {
    let prefix_width = u16::try_from(INPUT_PREFIX.chars().count()).unwrap_or(0);
    let input_width = usize::from(area.width.saturating_sub(prefix_width).max(1));
    let scroll = picker.filter.visual_scroll(input_width);
    let filter_span = match picker.filter.value().is_empty() {
        true => Span::styled(FILTER_PLACEHOLDER, palette.muted()),
        false => Span::raw(
            picker
                .filter
                .value()
                .chars()
                .skip(scroll)
                .collect::<String>(),
        ),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(INPUT_PREFIX, palette.title()),
            filter_span,
        ])),
        area,
    );
    if picker.mode != PickerMode::Filter {
        return;
    }
    let cursor_offset = picker.filter.visual_cursor().saturating_sub(scroll);
    let cursor_x = area
        .x
        .saturating_add(prefix_width)
        .saturating_add(u16::try_from(cursor_offset).unwrap_or(u16::MAX));
    frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(1)), area.y));
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    profiles: &[Profile],
    picker: &ProfilePicker,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let count_text = format!(
        " {} of {}",
        picker.visible_profile_indexes(profiles).len(),
        profiles.len()
    );
    let mut spans = vec![Span::styled(count_text, palette.muted())];
    if picker.connecting_profile_index.is_some() {
        spans.push(Span::styled(" · Connecting…", palette.title()));
        tui.effects.mark_loading(LoadSlot::Main, area);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
