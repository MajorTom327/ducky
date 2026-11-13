use ducky_app::GridCursor;
use ducky_core::{Cell, ResultSet, Sort, SortDirection};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Palette;

pub const MAX_COLUMN_WIDTH: usize = 40;
const COLUMN_GAP: usize = 1;
const NULL_CELL: Cell = None;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridViewport {
    pub first_row_index: usize,
    pub first_column_index: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn render_grid(
    frame: &mut Frame,
    area: Rect,
    result_set: &ResultSet,
    cursor: GridCursor,
    viewport: &mut GridViewport,
    sort: Option<&Sort>,
    is_focused: bool,
    palette: &Palette,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let reserved_rows = 1 + usize::from(result_set.is_truncated);
    let visible_row_count = usize::from(area.height)
        .saturating_sub(reserved_rows)
        .max(1);
    scroll_rows(viewport, cursor.row_index, visible_row_count);
    let visible_rows = result_set
        .rows
        .iter()
        .skip(viewport.first_row_index)
        .take(visible_row_count);
    let headers: Vec<String> = result_set
        .columns
        .iter()
        .map(|column| header_text(column, sort))
        .collect();
    let column_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(column_index, header)| {
            let widest_cell = visible_rows
                .clone()
                .map(|row| {
                    cell_text(row.get(column_index).unwrap_or(&NULL_CELL))
                        .chars()
                        .count()
                })
                .max()
                .unwrap_or(0);
            header
                .chars()
                .count()
                .max(widest_cell)
                .min(MAX_COLUMN_WIDTH)
        })
        .collect();
    scroll_columns(
        viewport,
        cursor.column_index,
        &column_widths,
        usize::from(area.width),
    );
    let shown_columns: Vec<usize> = (viewport.first_column_index..column_widths.len()).collect();

    let header_style = Style::new().add_modifier(Modifier::BOLD);
    let mut lines = vec![Line::from(
        shown_columns
            .iter()
            .map(|&column_index| {
                Span::styled(
                    fit(&headers[column_index], column_widths[column_index]),
                    header_style,
                )
            })
            .collect::<Vec<_>>(),
    )];
    if result_set.rows.is_empty() {
        lines.push(Line::styled("No rows", palette.muted()));
    }
    for (offset, row) in visible_rows.enumerate() {
        let row_index = viewport.first_row_index + offset;
        let is_cursor_row = row_index == cursor.row_index;
        let spans: Vec<Span> = shown_columns
            .iter()
            .map(|&column_index| {
                let cell = row.get(column_index).unwrap_or(&NULL_CELL);
                let mut style = match cell {
                    Some(_) => Style::new(),
                    None => palette.null_cell(),
                };
                if is_cursor_row {
                    style = style.patch(palette.selected(is_focused));
                }
                if is_cursor_row && column_index == cursor.column_index {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                Span::styled(fit(&cell_text(cell), column_widths[column_index]), style)
            })
            .collect();
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

fn fit(text: &str, width: usize) -> String {
    let character_count = text.chars().count();
    let fitted = match character_count > width {
        true => {
            let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
            truncated.push('…');
            truncated
        }
        false => text.to_owned(),
    };
    let padding = width.saturating_sub(fitted.chars().count()) + COLUMN_GAP;
    format!("{fitted}{}", " ".repeat(padding))
}

fn scroll_rows(viewport: &mut GridViewport, cursor_row_index: usize, visible_row_count: usize) {
    if cursor_row_index < viewport.first_row_index {
        viewport.first_row_index = cursor_row_index;
    }
    if cursor_row_index >= viewport.first_row_index + visible_row_count {
        viewport.first_row_index = cursor_row_index + 1 - visible_row_count;
    }
}

fn scroll_columns(
    viewport: &mut GridViewport,
    cursor_column_index: usize,
    column_widths: &[usize],
    available_width: usize,
) {
    let cursor_column_index = cursor_column_index.min(column_widths.len().saturating_sub(1));
    if cursor_column_index < viewport.first_column_index {
        viewport.first_column_index = cursor_column_index;
    }
    while viewport.first_column_index < cursor_column_index {
        let used_width: usize = column_widths[viewport.first_column_index..=cursor_column_index]
            .iter()
            .map(|width| width + COLUMN_GAP)
            .sum();
        if used_width <= available_width {
            break;
        }
        viewport.first_column_index += 1;
    }
}
