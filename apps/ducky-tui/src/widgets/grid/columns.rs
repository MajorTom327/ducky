use ducky_core::{Cell, Sort};

use super::{COLUMN_GAP, MAX_COLUMN_WIDTH, NULL_CELL, cell_text, header_text};

#[derive(Clone)]
pub(super) struct GridColumn {
    pub(super) header: String,
    pub(super) width: usize,
    natural_width: usize,
    pub(super) is_numeric: bool,
}

pub(super) fn grid_column(
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
    let natural_width = header.chars().count().max(widest_cell);
    GridColumn {
        width: natural_width.min(MAX_COLUMN_WIDTH),
        natural_width,
        header,
        is_numeric,
    }
}

pub(super) fn expand_truncated_columns(columns: &mut [GridColumn], grid_width: usize) {
    let used_width: usize = columns
        .iter()
        .map(|column| COLUMN_GAP.len() + column.width)
        .sum();
    let mut spare_width = grid_width.saturating_sub(used_width);
    for column in columns.iter_mut() {
        let extra_width = column
            .natural_width
            .saturating_sub(column.width)
            .min(spare_width);
        column.width += extra_width;
        spare_width -= extra_width;
    }
}
