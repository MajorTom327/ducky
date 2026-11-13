use ducky_core::ResultSet;

use crate::msg::GridMove;
use crate::state::GridCursor;

pub(crate) fn move_cursor(cursor: &mut GridCursor, result_set: &ResultSet, grid_move: GridMove) {
    let last_row_index = result_set.rows.len().saturating_sub(1);
    let last_column_index = result_set.columns.len().saturating_sub(1);
    match grid_move {
        GridMove::Left => cursor.column_index = cursor.column_index.saturating_sub(1),
        GridMove::Right => cursor.column_index += 1,
        GridMove::Up => cursor.row_index = cursor.row_index.saturating_sub(1),
        GridMove::Down => cursor.row_index += 1,
        GridMove::FirstRow => cursor.row_index = 0,
        GridMove::LastRow => cursor.row_index = last_row_index,
        GridMove::FirstColumn => cursor.column_index = 0,
        GridMove::LastColumn => cursor.column_index = last_column_index,
    }
    clamp_cursor(cursor, result_set);
}

pub(crate) fn clamp_cursor(cursor: &mut GridCursor, result_set: &ResultSet) {
    cursor.row_index = cursor
        .row_index
        .min(result_set.rows.len().saturating_sub(1));
    cursor.column_index = cursor
        .column_index
        .min(result_set.columns.len().saturating_sub(1));
}

pub(crate) fn cell_text(result_set: &ResultSet, cursor: GridCursor) -> Option<String> {
    let cell = result_set
        .rows
        .get(cursor.row_index)?
        .get(cursor.column_index)?;
    Some(cell.clone().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_set() -> ResultSet {
        ResultSet {
            columns: vec!["a".to_owned(), "b".to_owned()],
            rows: vec![
                vec![Some("1".to_owned()), None],
                vec![Some("2".to_owned()), Some("x".to_owned())],
            ],
            is_truncated: false,
        }
    }

    #[test]
    fn moves_clamp_inside_result_set() {
        let mut cursor = GridCursor::default();
        move_cursor(&mut cursor, &result_set(), GridMove::Up);
        move_cursor(&mut cursor, &result_set(), GridMove::Left);
        assert_eq!(cursor, GridCursor::default());
        for _ in 0..5 {
            move_cursor(&mut cursor, &result_set(), GridMove::Down);
            move_cursor(&mut cursor, &result_set(), GridMove::Right);
        }
        assert_eq!(
            cursor,
            GridCursor {
                row_index: 1,
                column_index: 1
            }
        );
        move_cursor(&mut cursor, &result_set(), GridMove::FirstRow);
        move_cursor(&mut cursor, &result_set(), GridMove::FirstColumn);
        assert_eq!(cursor, GridCursor::default());
        move_cursor(&mut cursor, &result_set(), GridMove::LastRow);
        move_cursor(&mut cursor, &result_set(), GridMove::LastColumn);
        assert_eq!(
            cursor,
            GridCursor {
                row_index: 1,
                column_index: 1
            }
        );
    }

    #[test]
    fn cell_text_maps_null_to_empty() {
        let cursor = GridCursor {
            row_index: 0,
            column_index: 1,
        };
        assert_eq!(cell_text(&result_set(), cursor), Some(String::new()));
    }
}
