use ducky_core::{Sort, SortDirection};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::cells::truncate;
use super::columns::GridColumn;
use super::{COLUMN_GAP, GUTTER_HEADER, LINK_SUFFIX};
use crate::theme::Palette;

pub(super) fn header_line(
    columns: &[GridColumn],
    gutter_width: usize,
    palette: &Palette,
) -> Line<'static> {
    let header_style = Style::new().add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        format!("{GUTTER_HEADER:>gutter_width$}"),
        palette.muted(),
    )];
    for column in columns {
        let suffix = if column.is_linked { LINK_SUFFIX } else { "" };
        let suffix_width = suffix.chars().count();
        let label = truncate(&column.header, column.width.saturating_sub(suffix_width));
        let padding = " ".repeat(
            column
                .width
                .saturating_sub(label.chars().count() + suffix_width),
        );
        let (leading_padding, trailing_padding) = match column.is_numeric {
            true => (padding, String::new()),
            false => (String::new(), padding),
        };
        spans.push(Span::raw(COLUMN_GAP));
        spans.push(Span::raw(leading_padding));
        spans.push(Span::styled(label, header_style));
        spans.push(Span::styled(suffix, palette.muted()));
        spans.push(Span::raw(trailing_padding));
    }
    Line::from(spans)
}

pub(super) fn rule_line(
    columns: &[GridColumn],
    gutter_width: usize,
    palette: &Palette,
) -> Line<'static> {
    let mut rule = "─".repeat(gutter_width);
    for column in columns {
        rule.push_str(COLUMN_GAP);
        rule.push_str(&"─".repeat(column.width));
    }
    Line::styled(rule, palette.muted())
}

pub(super) fn header_text(column: &str, sort: Option<&Sort>) -> String {
    match sort {
        Some(sort) if sort.column == column => match sort.direction {
            SortDirection::Ascending => format!("{column} ↑"),
            SortDirection::Descending => format!("{column} ↓"),
        },
        _ => column.to_owned(),
    }
}
