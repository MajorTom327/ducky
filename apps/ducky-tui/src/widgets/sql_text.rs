use ducky_sql::highlight;
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};

use crate::theme::token_style;

struct Segment {
    start_column: usize,
    end_column: usize,
    style: Style,
}

pub fn highlighted_sql(sql: &str) -> Text<'static> {
    let source_lines: Vec<&str> = sql.split('\n').collect();
    let mut segments_by_line: Vec<Vec<Segment>> = source_lines.iter().map(|_| Vec::new()).collect();
    for span in highlight(sql) {
        let style = token_style(span.kind);
        let spanned_lines = source_lines
            .iter()
            .zip(segments_by_line.iter_mut())
            .enumerate()
            .skip(span.start.line)
            .take(span.end.line + 1 - span.start.line);
        for (line_index, (source_line, line_segments)) in spanned_lines {
            let line_length = source_line.chars().count();
            let start_column = match line_index == span.start.line {
                true => span.start.column,
                false => 0,
            };
            let end_column = match line_index == span.end.line {
                true => span.end.column.min(line_length),
                false => line_length,
            };
            if start_column < end_column {
                line_segments.push(Segment {
                    start_column,
                    end_column,
                    style,
                });
            }
        }
    }
    let lines: Vec<Line<'static>> = source_lines
        .iter()
        .zip(segments_by_line)
        .map(|(source_line, line_segments)| styled_line(source_line, line_segments))
        .collect();
    Text::from(lines)
}

fn styled_line(source_line: &str, mut line_segments: Vec<Segment>) -> Line<'static> {
    line_segments.sort_by_key(|segment| segment.start_column);
    let characters: Vec<char> = source_line.chars().collect();
    let mut spans = Vec::new();
    let mut current_column = 0;
    for segment in line_segments {
        if segment.start_column < current_column {
            continue;
        }
        if segment.start_column > current_column {
            spans.push(Span::raw(collect(
                &characters,
                current_column,
                segment.start_column,
            )));
        }
        spans.push(Span::styled(
            collect(&characters, segment.start_column, segment.end_column),
            segment.style,
        ));
        current_column = segment.end_column;
    }
    if current_column < characters.len() {
        spans.push(Span::raw(collect(
            &characters,
            current_column,
            characters.len(),
        )));
    }
    Line::from(spans)
}

fn collect(characters: &[char], start_column: usize, end_column: usize) -> String {
    characters[start_column..end_column].iter().collect()
}
