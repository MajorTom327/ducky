use ducky_app::{NoticeLevel, PickerMode, ProfilePicker, ProfileRow};
use ducky_core::Profile;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::theme::{Palette, kind_badge, palette as profile_palette};
use crate::tui_state::TuiState;
use crate::widgets::loading::LoadSlot;

const INPUT_PREFIX: &str = " › ";
const FILTER_PLACEHOLDER: &str = "type to filter";
const CONNECTED_MARKER: &str = " ● ";
const ROW_INDENT: &str = "   ";
const BADGE_WIDTH: usize = 7;
const CHROME_ROWS: u16 = 3;

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
    profiles: &[Profile],
    picker: &ProfilePicker,
    connected_profile_index: Option<usize>,
    tui: &mut TuiState,
    palette: &Palette,
) {
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
        connected_profile_index,
        list_area,
        palette,
    );
    frame.render_widget(Paragraph::new(profile_lines), list_area);
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

fn row_lines(
    rows: &[ProfileRow],
    profiles: &[Profile],
    picker: &ProfilePicker,
    connected_profile_index: Option<usize>,
    area: Rect,
    palette: &Palette,
) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return vec![Line::styled(" No matching profile", palette.muted())];
    }
    let selected_profile_index = picker.selected_visible_profile_index(profiles);
    let selected_row_position = rows
        .iter()
        .position(|row| Some(row) == selected_profile_index.map(ProfileRow::Profile).as_ref())
        .unwrap_or(0);
    let visible_row_count = usize::from(area.height).max(1);
    let first_row_position = (selected_row_position + 1).saturating_sub(visible_row_count);
    let row_width = usize::from(area.width);
    rows.iter()
        .enumerate()
        .skip(first_row_position)
        .take(visible_row_count)
        .filter_map(|(row_position, row)| match row {
            ProfileRow::Project(project_name) => {
                let member_count = rows[row_position + 1..]
                    .iter()
                    .take_while(|next_row| matches!(next_row, ProfileRow::Profile(_)))
                    .count();
                Some(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(project_name.clone(), palette.title()),
                    Span::styled(format!(" {member_count}"), palette.muted()),
                ]))
            }
            ProfileRow::Profile(profile_index) => profiles.get(*profile_index).map(|profile| {
                profile_line(
                    profile,
                    Some(*profile_index) == selected_profile_index,
                    Some(*profile_index) == connected_profile_index,
                    row_width,
                    palette,
                )
            }),
        })
        .collect()
}

fn profile_line(
    profile: &Profile,
    is_selected: bool,
    is_connected: bool,
    row_width: usize,
    palette: &Palette,
) -> Line<'static> {
    let kind_palette = profile_palette(Some(profile.kind));
    let (badge_style, muted_style) = match is_selected {
        true => (Style::new(), Style::new()),
        false => (Style::new().fg(kind_palette.accent), palette.muted()),
    };
    let badge = kind_badge(profile.kind).trim_matches(['[', ']']);
    let mut left_spans = vec![
        Span::raw(match is_connected {
            true => CONNECTED_MARKER,
            false => ROW_INDENT,
        }),
        Span::styled(format!("{badge:<BADGE_WIDTH$}"), badge_style),
        Span::raw(" "),
        Span::raw(profile.label.clone()),
    ];
    if !profile.tags.is_empty() {
        let tags_text = profile
            .tags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" ");
        left_spans.push(Span::styled(format!("  {tags_text}"), muted_style));
    }
    let target_span = Span::styled(profile.target(), muted_style);
    let left_width: usize = left_spans.iter().map(Span::width).sum();
    let right_width = target_span.width();
    if left_width + right_width + 3 <= row_width {
        left_spans.push(Span::raw(
            " ".repeat(row_width - left_width - right_width - 1),
        ));
        left_spans.push(target_span);
        left_spans.push(Span::raw(" "));
    }
    let line = Line::from(left_spans);
    match is_selected {
        true => line.style(kind_palette.selected(true)),
        false => line,
    }
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
