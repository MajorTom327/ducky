use ducky_app::GridCursor;
use ducky_core::{Cell, ResultSet, Sort, SortDirection};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Palette;

mod columns;
mod viewport;

use columns::{GridColumn, expand_truncated_columns, grid_column};
pub use viewport::GridViewport;
use viewport::{scroll_columns, scroll_rows};

pub const MAX_COLUMN_WIDTH: usize = 40;
const COLUMN_GAP: &str = "  ";
const HEADER_ROW_COUNT: usize = 2;
const GUTTER_HEADER: &str = "#";
const NULL_CELL: Cell = None;
const LINK_SUFFIX: &str = " →";
const SELECTION_MARKER: &str = "▌";

#[derive(Debug, Clone, Copy)]
pub struct GridOptions<'a> {
    pub cursor: GridCursor,
    pub sort: Option<&'a Sort>,
    pub first_row_number: usize,
    pub is_focused: bool,
    pub linked_columns: &'a [String],
    pub selected_row_indexes: &'a [usize],
}

pub fn render_grid(
    frame: &mut Frame,
    area: Rect,
    result_set: &ResultSet,
    viewport: &mut GridViewport,
    options: GridOptions,
    palette: &Palette,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let cursor = options.cursor;
    let reserved_rows = HEADER_ROW_COUNT + usize::from(result_set.is_truncated);
    let visible_row_count = usize::from(area.height)
        .saturating_sub(reserved_rows)
        .max(1);
    scroll_rows(viewport, cursor.row_index, visible_row_count);
    let visible_rows: Vec<&Vec<Cell>> = result_set
        .rows
        .iter()
        .skip(viewport.first_row_index)
        .take(visible_row_count)
        .collect();
    let columns: Vec<GridColumn> = result_set
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, name)| {
            let is_linked = options.linked_columns.contains(name);
            grid_column(name, column_index, options.sort, is_linked, &visible_rows)
        })
        .collect();
    let last_row_number =
        options.first_row_number + viewport.first_row_index + visible_rows.len().saturating_sub(1);
    let gutter_width = last_row_number.to_string().len().max(GUTTER_HEADER.len());
    let marker_width = usize::from(!options.selected_row_indexes.is_empty());
    let full_gutter_width = gutter_width + marker_width;
    let column_widths: Vec<usize> = columns.iter().map(|column| column.width).collect();
    let grid_width = usize::from(area.width).saturating_sub(full_gutter_width);
    scroll_columns(
        viewport,
        cursor.column_index,
        &column_widths,
        grid_width,
        COLUMN_GAP.len(),
    );
    let mut shown_columns = columns[viewport.first_column_index.min(columns.len())..].to_vec();
    expand_truncated_columns(&mut shown_columns, grid_width);
    let shown_columns = shown_columns.as_slice();

    let mut lines = vec![
        header_line(shown_columns, full_gutter_width, palette),
        rule_line(shown_columns, full_gutter_width, palette),
    ];
    if result_set.rows.is_empty() {
        lines.push(Line::styled("No rows", palette.muted()));
    }
    for (offset, row) in visible_rows.into_iter().enumerate() {
        let row_index = viewport.first_row_index + offset;
        let row_number = options.first_row_number + row_index;
        let cursor_column_index = (row_index == cursor.row_index).then_some(cursor.column_index);
        let is_selected = options
            .selected_row_indexes
            .binary_search(&row_index)
            .is_ok();
        let mut spans = Vec::with_capacity(2 * shown_columns.len() + 2);
        if marker_width > 0 {
            spans.push(match is_selected {
                true => Span::styled(SELECTION_MARKER, palette.title()),
                false => Span::raw(" "),
            });
        }
        spans.push(Span::styled(
            format!("{row_number:>gutter_width$}"),
            match cursor_column_index {
                Some(_) => palette.cursor_row(),
                None => palette.muted(),
            },
        ));
        for (shown_offset, column) in shown_columns.iter().enumerate() {
            let column_index = viewport.first_column_index + shown_offset;
            let cell = row.get(column_index).unwrap_or(&NULL_CELL);
            let style = cell_style(
                cell,
                cursor_column_index,
                column_index,
                is_selected,
                options.is_focused,
                palette,
            );
            let gap_style = match cursor_column_index {
                Some(_) => palette.cursor_row(),
                None => Style::new(),
            };
            spans.push(Span::styled(COLUMN_GAP, gap_style));
            spans.push(Span::styled(
                fit(&cell_text(cell), column.width, column.is_numeric),
                style,
            ));
        }
        lines.push(Line::from(spans));
    }
    if result_set.is_truncated {
        lines.push(Line::styled(
            format!("Showing first {} rows", result_set.rows.len()),
            palette.muted(),
        ));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn header_line(columns: &[GridColumn], gutter_width: usize, palette: &Palette) -> Line<'static> {
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

fn rule_line(columns: &[GridColumn], gutter_width: usize, palette: &Palette) -> Line<'static> {
    let mut rule = "─".repeat(gutter_width);
    for column in columns {
        rule.push_str(COLUMN_GAP);
        rule.push_str(&"─".repeat(column.width));
    }
    Line::styled(rule, palette.muted())
}

fn cell_style(
    cell: &Cell,
    cursor_column_index: Option<usize>,
    column_index: usize,
    is_selected: bool,
    is_focused: bool,
    palette: &Palette,
) -> Style {
    let Some(cursor_column_index) = cursor_column_index else {
        let style = match cell {
            Some(_) => Style::new(),
            None => palette.null_cell(),
        };
        return match is_selected {
            true => style.patch(palette.muted()).add_modifier(Modifier::BOLD),
            false => style,
        };
    };
    let mut style = palette.cursor_row();
    if cell.is_none() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    match (column_index == cursor_column_index, is_focused) {
        (true, true) => style.patch(palette.selected(true)),
        (true, false) => style.add_modifier(Modifier::UNDERLINED),
        (false, _) => style,
    }
}

fn header_text(column: &str, sort: Option<&Sort>) -> String {
    match sort {
        Some(sort) if sort.column == column => match sort.direction {
            SortDirection::Ascending => format!("{column} ↑"),
            SortDirection::Descending => format!("{column} ↓"),
        },
        _ => column.to_owned(),
    }
}

fn cell_text(cell: &Cell) -> String {
    match cell {
        Some(value) => value.replace(['\n', '\r', '\t'], " "),
        None => "NULL".to_owned(),
    }
}

fn truncate(text: &str, width: usize) -> String {
    match text.chars().count() > width {
        true => {
            let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
            truncated.push('…');
            truncated
        }
        false => text.to_owned(),
    }
}

fn fit(text: &str, width: usize, is_right_aligned: bool) -> String {
    let fitted = truncate(text, width);
    match is_right_aligned {
        true => format!("{fitted:>width$}"),
        false => format!("{fitted:<width$}"),
    }
}
