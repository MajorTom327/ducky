use std::collections::BTreeMap;

use ducky_app::GridCursor;
use ducky_core::{Cell, ResultSet, Sort};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::keys::sequence::GridPane;
use crate::mouse::{MouseTarget, MouseTargets};
use crate::theme::Palette;

mod cells;
mod columns;
mod header;
mod targets;
mod viewport;

use cells::{cell_area, cell_style, cell_text, displayed_cell, fit};
use columns::{GridColumn, expand_truncated_columns, grid_column};
use header::{header_line, rule_line};
use targets::header_targets;
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
    pub draft_cells: &'a BTreeMap<GridCursor, Cell>,
    pub pane: GridPane,
}

pub fn render_grid(
    frame: &mut Frame,
    area: Rect,
    result_set: &ResultSet,
    viewport: &mut GridViewport,
    options: GridOptions,
    palette: &Palette,
    mouse_targets: &mut MouseTargets,
) -> Option<Rect> {
    if area.height == 0 || area.width == 0 {
        return None;
    }
    let cursor = options.cursor;
    let reserved_rows = HEADER_ROW_COUNT + usize::from(result_set.is_truncated);
    let visible_row_count = usize::from(area.height)
        .saturating_sub(reserved_rows)
        .max(1);
    scroll_rows(viewport, cursor.row_index, visible_row_count);
    let first_row_index = viewport.first_row_index;
    let visible_rows: Vec<&Vec<Cell>> = result_set
        .rows
        .iter()
        .skip(first_row_index)
        .take(visible_row_count)
        .collect();
    let columns: Vec<GridColumn> = result_set
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, name)| {
            let is_linked = options.linked_columns.contains(name);
            let visible_cells = visible_rows.iter().enumerate().map(|(offset, row)| {
                let cell_cursor = GridCursor {
                    row_index: first_row_index + offset,
                    column_index,
                };
                displayed_cell(options.draft_cells, row, cell_cursor)
            });
            grid_column(name, options.sort, is_linked, visible_cells)
        })
        .collect();
    let last_row_number =
        options.first_row_number + first_row_index + visible_rows.len().saturating_sub(1);
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
    mouse_targets.push((area, MouseTarget::Grid(options.pane)));
    mouse_targets.extend(header_targets(
        area,
        shown_columns,
        full_gutter_width,
        viewport.first_column_index,
        options.pane,
    ));

    let mut lines = vec![
        header_line(shown_columns, full_gutter_width, palette),
        rule_line(shown_columns, full_gutter_width, palette),
    ];
    if result_set.rows.is_empty() {
        lines.push(Line::styled("No rows", palette.muted()));
    }
    let mut cursor_cell_area = None;
    for (offset, row) in visible_rows.into_iter().enumerate() {
        let row_index = first_row_index + offset;
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
        let mut cell_offset_x = full_gutter_width;
        for (shown_offset, column) in shown_columns.iter().enumerate() {
            let column_index = viewport.first_column_index + shown_offset;
            let cell_cursor = GridCursor {
                row_index,
                column_index,
            };
            let is_draft = options.draft_cells.contains_key(&cell_cursor);
            let cell = displayed_cell(options.draft_cells, row, cell_cursor);
            let style = cell_style(
                cell,
                is_draft,
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
            cell_offset_x += COLUMN_GAP.len();
            let body_cell_area = cell_area(area, cell_offset_x, offset, column.width);
            let cell_target = MouseTarget::GridCell {
                pane: options.pane,
                row_index,
                column_index,
            };
            mouse_targets.push((body_cell_area.intersection(area), cell_target));
            if cursor_column_index == Some(column_index) {
                cursor_cell_area = Some(body_cell_area);
            }
            cell_offset_x += column.width;
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
    cursor_cell_area
}
