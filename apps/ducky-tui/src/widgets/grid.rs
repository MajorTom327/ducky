use ducky_app::GridCursor;
use ducky_core::{Cell, ResultSet, Sort, SortDirection};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Palette;

mod viewport;

pub use viewport::GridViewport;
use viewport::{scroll_columns, scroll_rows};

pub const MAX_COLUMN_WIDTH: usize = 40;
const COLUMN_GAP: &str = "  ";
const HEADER_ROW_COUNT: usize = 2;
const GUTTER_HEADER: &str = "#";
const NULL_CELL: Cell = None;

#[derive(Debug, Clone, Copy)]
pub struct GridOptions<'a> {
    pub cursor: GridCursor,
    pub sort: Option<&'a Sort>,
    pub first_row_number: usize,
    pub is_focused: bool,
}

struct GridColumn {
    header: String,
    width: usize,
    is_numeric: bool,
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
        .map(|(column_index, name)| grid_column(name, column_index, options.sort, &visible_rows))
        .collect();
    let last_row_number =
        options.first_row_number + viewport.first_row_index + visible_rows.len().saturating_sub(1);
    let gutter_width = last_row_number.to_string().len().max(GUTTER_HEADER.len());
    let column_widths: Vec<usize> = columns.iter().map(|column| column.width).collect();
    let grid_width = usize::from(area.width).saturating_sub(gutter_width);
    scroll_columns(
        viewport,
        cursor.column_index,
        &column_widths,
        grid_width,
        COLUMN_GAP.len(),
    );
    let shown_columns = &columns[viewport.first_column_index.min(columns.len())..];

    let mut lines = vec![
        header_line(shown_columns, gutter_width, palette),
        rule_line(shown_columns, gutter_width, palette),
    ];
    if result_set.rows.is_empty() {
        lines.push(Line::styled("No rows", palette.muted()));
    }
    for (offset, row) in visible_rows.into_iter().enumerate() {
        let row_index = viewport.first_row_index + offset;
        let row_number = options.first_row_number + row_index;
        let cursor_column_index = (row_index == cursor.row_index).then_some(cursor.column_index);
        let mut spans = vec![Span::styled(
            format!("{row_number:>gutter_width$}"),
            match cursor_column_index {
                Some(_) => palette.cursor_row(),
                None => palette.muted(),
            },
        )];
        for (shown_offset, column) in shown_columns.iter().enumerate() {
            let column_index = viewport.first_column_index + shown_offset;
            let cell = row.get(column_index).unwrap_or(&NULL_CELL);
            let style = cell_style(
                cell,
                cursor_column_index,
                column_index,
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

fn grid_column(
    name: &str,
    column_index: usize,
    sort: Option<&Sort>,
    visible_rows: &[&Vec<Cell>],
) -> GridColumn {
    let header = header_text(name, sort);
    let visible_cells = visible_rows
        .iter()
        .map(|row| row.get(column_index).unwrap_or(&NULL_CELL));
    let widest_cell = visible_cells
        .clone()
        .map(|cell| cell_text(cell).chars().count())
        .max()
        .unwrap_or(0);
    let mut values = visible_cells.flatten().peekable();
    let has_value = values.peek().is_some();
    let is_numeric = has_value && values.all(|value| value.trim().parse::<f64>().is_ok());
    GridColumn {
        width: header
            .chars()
            .count()
            .max(widest_cell)
            .min(MAX_COLUMN_WIDTH),
        header,
        is_numeric,
    }
}

fn header_line(columns: &[GridColumn], gutter_width: usize, palette: &Palette) -> Line<'static> {
    let header_style = Style::new().add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        format!("{GUTTER_HEADER:>gutter_width$}"),
        palette.muted(),
    )];
    for column in columns {
        spans.push(Span::raw(COLUMN_GAP));
        spans.push(Span::styled(
            fit(&column.header, column.width, column.is_numeric),
            header_style,
        ));
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
    is_focused: bool,
    palette: &Palette,
) -> Style {
    let Some(cursor_column_index) = cursor_column_index else {
        return match cell {
            Some(_) => Style::new(),
            None => palette.null_cell(),
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

fn fit(text: &str, width: usize, is_right_aligned: bool) -> String {
    let fitted = match text.chars().count() > width {
        true => {
            let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
            truncated.push('…');
            truncated
        }
        false => text.to_owned(),
    };
    match is_right_aligned {
        true => format!("{fitted:>width$}"),
        false => format!("{fitted:<width$}"),
    }
}
