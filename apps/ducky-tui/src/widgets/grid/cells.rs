use std::collections::BTreeMap;

use ducky_app::GridCursor;
use ducky_core::Cell;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::{HEADER_ROW_COUNT, NULL_CELL};
use crate::theme::Palette;

pub(super) fn displayed_cell<'a>(
    draft_cells: &'a BTreeMap<GridCursor, Cell>,
    row: &'a [Cell],
    cell_cursor: GridCursor,
) -> &'a Cell {
    draft_cells
        .get(&cell_cursor)
        .unwrap_or_else(|| row.get(cell_cursor.column_index).unwrap_or(&NULL_CELL))
}

pub(super) fn cell_area(
    area: Rect,
    cell_offset_x: usize,
    row_offset: usize,
    cell_width: usize,
) -> Rect {
    let x = area
        .x
        .saturating_add(u16::try_from(cell_offset_x).unwrap_or(u16::MAX))
        .min(area.right());
    let y = area
        .y
        .saturating_add(u16::try_from(HEADER_ROW_COUNT + row_offset).unwrap_or(u16::MAX));
    let width = u16::try_from(cell_width)
        .unwrap_or(u16::MAX)
        .min(area.right() - x);
    Rect::new(x, y, width, 1)
}

pub(super) fn cell_style(
    cell: &Cell,
    is_draft: bool,
    cursor_column_index: Option<usize>,
    column_index: usize,
    is_selected: bool,
    is_focused: bool,
    palette: &Palette,
) -> Style {
    let Some(cursor_column_index) = cursor_column_index else {
        let style = match (is_draft, cell) {
            (true, _) => return palette.draft_cell(),
            (false, Some(_)) => Style::new(),
            (false, None) => palette.null_cell(),
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
        (false, _) if is_draft => style.patch(palette.draft_cell()),
        (false, _) => style,
    }
}

pub(super) fn cell_text(cell: &Cell) -> String {
    match cell {
        Some(value) => value.replace(['\n', '\r', '\t'], " "),
        None => "NULL".to_owned(),
    }
}

pub(super) fn truncate(text: &str, width: usize) -> String {
    match text.chars().count() > width {
        true => {
            let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
            truncated.push('…');
            truncated
        }
        false => text.to_owned(),
    }
}

pub(super) fn fit(text: &str, width: usize, is_right_aligned: bool) -> String {
    let fitted = truncate(text, width);
    match is_right_aligned {
        true => format!("{fitted:>width$}"),
        false => format!("{fitted:<width$}"),
    }
}
