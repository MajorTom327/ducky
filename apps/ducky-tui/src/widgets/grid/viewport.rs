#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridViewport {
    pub first_row_index: usize,
    pub first_column_index: usize,
}

pub(super) fn scroll_rows(
    viewport: &mut GridViewport,
    cursor_row_index: usize,
    visible_row_count: usize,
) {
    if cursor_row_index < viewport.first_row_index {
        viewport.first_row_index = cursor_row_index;
    }
    if cursor_row_index >= viewport.first_row_index + visible_row_count {
        viewport.first_row_index = cursor_row_index + 1 - visible_row_count;
    }
}

pub(super) fn scroll_columns(
    viewport: &mut GridViewport,
    cursor_column_index: usize,
    column_widths: &[usize],
    available_width: usize,
    column_gap_width: usize,
) {
    let cursor_column_index = cursor_column_index.min(column_widths.len().saturating_sub(1));
    if cursor_column_index < viewport.first_column_index {
        viewport.first_column_index = cursor_column_index;
    }
    while viewport.first_column_index < cursor_column_index {
        let used_width: usize = column_widths[viewport.first_column_index..=cursor_column_index]
            .iter()
            .map(|width| width + column_gap_width)
            .sum();
        if used_width <= available_width {
            break;
        }
        viewport.first_column_index += 1;
    }
}
