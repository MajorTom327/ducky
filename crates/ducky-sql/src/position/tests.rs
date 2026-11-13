use super::*;

const MULTI_LINE_SQL: &str = "select 'é'\nfrom t\n\nwhere x";

#[test]
fn offset_at_counts_columns_in_chars() {
    assert_eq!(
        offset_at(MULTI_LINE_SQL, TextPosition { line: 0, column: 9 }),
        10
    );
    assert_eq!(
        offset_at(
            MULTI_LINE_SQL,
            TextPosition {
                line: 0,
                column: 10
            }
        ),
        11
    );
    assert_eq!(
        offset_at(MULTI_LINE_SQL, TextPosition { line: 1, column: 0 }),
        12
    );
}

#[test]
fn offset_at_clamps() {
    assert_eq!(
        offset_at(
            MULTI_LINE_SQL,
            TextPosition {
                line: 0,
                column: 99
            }
        ),
        11
    );
    assert_eq!(
        offset_at(MULTI_LINE_SQL, TextPosition { line: 9, column: 0 }),
        MULTI_LINE_SQL.len()
    );
}

#[test]
fn position_and_offset_round_trip() {
    for (offset, _) in MULTI_LINE_SQL.char_indices() {
        let position = position_at(MULTI_LINE_SQL, offset);
        assert_eq!(offset_at(MULTI_LINE_SQL, position), offset);
    }
    let end_position = position_at(MULTI_LINE_SQL, MULTI_LINE_SQL.len());
    assert_eq!(end_position, TextPosition { line: 3, column: 7 });
    assert_eq!(
        offset_at(MULTI_LINE_SQL, end_position),
        MULTI_LINE_SQL.len()
    );
}

#[test]
fn position_at_inside_multi_byte_char_floors() {
    assert_eq!(
        position_at(MULTI_LINE_SQL, 9),
        TextPosition { line: 0, column: 8 }
    );
    assert_eq!(
        position_at(MULTI_LINE_SQL, 10),
        TextPosition { line: 0, column: 9 }
    );
    assert_eq!(
        position_at(MULTI_LINE_SQL, 12),
        TextPosition { line: 1, column: 0 }
    );
}
