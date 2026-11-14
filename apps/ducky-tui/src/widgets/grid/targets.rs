use ratatui::layout::Rect;

use super::COLUMN_GAP;
use super::cells::cell_area;
use super::columns::GridColumn;
use crate::keys::sequence::GridPane;
use crate::mouse::MouseTarget;

pub(super) fn header_targets(
    area: Rect,
    shown_columns: &[GridColumn],
    gutter_width: usize,
    first_column_index: usize,
    pane: GridPane,
) -> impl Iterator<Item = (Rect, MouseTarget)> {
    shown_columns
        .iter()
        .scan(gutter_width, |offset_x, column| {
            let column_offset_x = *offset_x + COLUMN_GAP.len();
            *offset_x = column_offset_x + column.width;
            Some((column_offset_x, column.width))
        })
        .enumerate()
        .map(move |(shown_offset, (column_offset_x, column_width))| {
            let body_cell_area = cell_area(area, column_offset_x, 0, column_width);
            let header_area = Rect {
                y: area.y,
                ..body_cell_area
            };
            let target = MouseTarget::GridHeader {
                pane,
                column_index: first_column_index + shown_offset,
            };
            (header_area, target)
        })
}
