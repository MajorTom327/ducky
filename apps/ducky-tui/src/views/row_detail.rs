use std::ops::Range;

use ducky_app::RowDetail;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};

use crate::theme::Palette;

const WIDTH_PERCENT: u32 = 70;
const MAX_HEIGHT_PERCENT: u32 = 80;
const NAME_GAP: &str = "  ";
const BORDER_SIZE: u16 = 2;
const HORIZONTAL_PADDING: u16 = 1;

struct DetailLayout {
    name_width: usize,
    value_width: usize,
}

pub fn render(frame: &mut Frame, screen: Rect, row_detail: &RowDetail, palette: &Palette) -> Rect {
    let width = percent_of(screen.width, WIDTH_PERCENT);
    let inner_width = usize::from(width.saturating_sub(BORDER_SIZE + 2 * HORIZONTAL_PADDING));
    let longest_name = row_detail
        .columns
        .iter()
        .map(|column| column.chars().count())
        .max()
        .unwrap_or(0);
    let name_width = longest_name.min(inner_width / 3).max(1);
    let layout = DetailLayout {
        name_width,
        value_width: inner_width
            .saturating_sub(name_width + NAME_GAP.len())
            .max(1),
    };
    let mut lines = Vec::new();
    let mut selected_lines = 0..0;
    for (field_index, column) in row_detail.columns.iter().enumerate() {
        let is_selected = field_index == row_detail.selected_field_index;
        let value = row_detail
            .values
            .get(field_index)
            .and_then(Option::as_deref);
        let field_lines = field_lines(column, value, is_selected, &layout, palette);
        if is_selected {
            selected_lines = lines.len()..lines.len() + field_lines.len();
        }
        lines.extend(field_lines);
    }
    let max_height = percent_of(screen.height, MAX_HEIGHT_PERCENT);
    let height = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(BORDER_SIZE)
        .min(max_height);
    let area = screen.centered(Constraint::Length(width), Constraint::Length(height));
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .padding(Padding::horizontal(HORIZONTAL_PADDING))
        .title(Span::styled(" Row detail ", palette.title()))
        .border_style(palette.border(true));
    let scroll = scroll_offset(
        selected_lines,
        usize::from(height.saturating_sub(BORDER_SIZE)),
    );
    let paragraph = Paragraph::new(lines)
        .scroll((u16::try_from(scroll).unwrap_or(0), 0))
        .block(block);
    frame.render_widget(paragraph, area);
    area
}

fn percent_of(length: u16, percent: u32) -> u16 {
    u16::try_from(u32::from(length) * percent / 100).unwrap_or(length)
}

fn scroll_offset(selected_lines: Range<usize>, visible_height: usize) -> usize {
    selected_lines
        .end
        .saturating_sub(visible_height)
        .min(selected_lines.start)
}

fn field_lines(
    column: &str,
    value: Option<&str>,
    is_selected: bool,
    layout: &DetailLayout,
    palette: &Palette,
) -> Vec<Line<'static>> {
    let (name_style, value_style) = match (is_selected, value) {
        (true, _) => (palette.selected(true), palette.selected(true)),
        (false, Some(_)) => (palette.muted(), Style::new()),
        (false, None) => (palette.muted(), palette.null_cell()),
    };
    let chunks = match value {
        Some(text) => wrapped(text, layout.value_width),
        None => vec!["NULL".to_owned()],
    };
    let name_width = layout.name_width;
    let continuation_indent = " ".repeat(name_width + NAME_GAP.len());
    chunks
        .into_iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let prefix = match chunk_index {
                0 => Span::styled(
                    format!("{:>name_width$}{NAME_GAP}", truncated(column, name_width)),
                    name_style,
                ),
                _ => Span::styled(continuation_indent.clone(), name_style),
            };
            let line = Line::from(vec![prefix, Span::styled(chunk, value_style)]);
            match is_selected {
                true => line.style(palette.selected(true)),
                false => line,
            }
        })
        .collect()
}

fn truncated(text: &str, width: usize) -> String {
    match text.chars().count() > width {
        true => {
            let mut shortened: String = text.chars().take(width.saturating_sub(1)).collect();
            shortened.push('…');
            shortened
        }
        false => text.to_owned(),
    }
}

fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    for line in text.split('\n') {
        let characters: Vec<char> = line.trim_end_matches('\r').chars().collect();
        if characters.is_empty() {
            chunks.push(String::new());
        }
        chunks.extend(
            characters
                .chunks(width)
                .map(|chunk| chunk.iter().collect::<String>()),
        );
    }
    chunks
}
