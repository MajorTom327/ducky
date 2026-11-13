use ducky_sql::highlight;
use edtui::{Highlight, Index2};

use crate::editor::panel::EditorPanel;
use crate::theme::token_style;

pub fn refresh_highlights(panel: &mut EditorPanel) {
    let text = panel.text();
    if text == panel.highlighted_text {
        return;
    }
    let highlights = editor_highlights(&text);
    panel.state.set_highlights(highlights);
    panel.highlighted_text = text;
}

pub fn editor_highlights(sql: &str) -> Vec<Highlight> {
    let line_lengths: Vec<usize> = sql.split('\n').map(|line| line.chars().count()).collect();
    let mut highlights = Vec::new();
    for span in highlight(sql) {
        let style = token_style(span.kind);
        for row in span.start.line..=span.end.line {
            let Some(&line_length) = line_lengths.get(row) else {
                break;
            };
            let start_column = match row == span.start.line {
                true => span.start.column,
                false => 0,
            };
            let end_column = match row == span.end.line {
                true => span.end.column.min(line_length),
                false => line_length,
            };
            if start_column < end_column {
                highlights.push(Highlight::new(
                    Index2::new(row, start_column),
                    Index2::new(row, end_column - 1),
                    style,
                ));
            }
        }
    }
    highlights
}
