#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

pub fn offset_at(sql: &str, position: TextPosition) -> usize {
    let mut line_start = 0;
    for _ in 0..position.line {
        match sql[line_start..].find('\n') {
            Some(newline_index) => line_start += newline_index + 1,
            None => return sql.len(),
        }
    }
    let line_end = sql[line_start..]
        .find('\n')
        .map_or(sql.len(), |newline_index| line_start + newline_index);
    sql[line_start..line_end]
        .char_indices()
        .nth(position.column)
        .map_or(line_end, |(char_offset, _)| line_start + char_offset)
}

pub fn position_at(sql: &str, offset: usize) -> TextPosition {
    let bounded_offset = floor_char_boundary(sql, offset);
    let text_before = &sql[..bounded_offset];
    let line_start = text_before
        .rfind('\n')
        .map_or(0, |newline_index| newline_index + 1);
    TextPosition {
        line: text_before.matches('\n').count(),
        column: text_before[line_start..].chars().count(),
    }
}

pub(crate) fn floor_char_boundary(sql: &str, offset: usize) -> usize {
    let mut bounded_offset = offset.min(sql.len());
    while !sql.is_char_boundary(bounded_offset) {
        bounded_offset -= 1;
    }
    bounded_offset
}

#[cfg(test)]
mod tests;
