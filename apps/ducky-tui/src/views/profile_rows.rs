use ducky_app::{ProfilePicker, ProfileRow};
use ducky_core::Profile;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::mouse::MouseTarget;
use crate::theme::{Palette, kind_badge, palette as profile_palette};

const CONNECTED_MARKER: &str = " ● ";
const ROW_INDENT: &str = "   ";
const BADGE_WIDTH: usize = 7;

pub fn first_visible_row_position(
    rows: &[ProfileRow],
    profiles: &[Profile],
    picker: &ProfilePicker,
    visible_row_count: usize,
) -> usize {
    let selected_profile_index = picker.selected_visible_profile_index(profiles);
    let selected_row_position = rows
        .iter()
        .position(|row| Some(row) == selected_profile_index.map(ProfileRow::Profile).as_ref())
        .unwrap_or(0);
    (selected_row_position + 1).saturating_sub(visible_row_count.max(1))
}

pub fn row_targets(
    area: Rect,
    rows: &[ProfileRow],
    first_row_position: usize,
    row_target: fn(usize) -> MouseTarget,
) -> impl Iterator<Item = (Rect, MouseTarget)> {
    area.rows()
        .zip(rows.iter().skip(first_row_position))
        .filter_map(move |(row_area, row)| match row {
            ProfileRow::Profile(profile_index) => Some((row_area, row_target(*profile_index))),
            ProfileRow::Project(_) => None,
        })
}

pub fn row_lines(
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
    let visible_row_count = usize::from(area.height).max(1);
    let first_row_position = first_visible_row_position(rows, profiles, picker, visible_row_count);
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
