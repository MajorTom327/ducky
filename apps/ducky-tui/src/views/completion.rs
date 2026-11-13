use ducky_sql::CompletionItem;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::theme::Palette;
use crate::tui_state::TuiState;

const MAX_VISIBLE_ITEMS: usize = 8;
const LABEL_DETAIL_GAP: &str = "  ";

pub fn render(frame: &mut Frame, editor_area: Rect, tui: &TuiState, palette: &Palette) {
    let Some(popup) = &tui.editor.completion else {
        return;
    };
    let Some(cursor_position) = tui.editor.state.cursor_screen_position() else {
        return;
    };
    let bounds = frame.area();
    let items = &popup.completion.items;
    let visible_item_count = items.len().min(MAX_VISIBLE_ITEMS);
    if visible_item_count == 0 {
        return;
    }
    let content_width = items.iter().map(item_width).max().unwrap_or_default();
    let popup_width = u16::try_from(content_width + 2)
        .unwrap_or(u16::MAX)
        .min(bounds.width);
    let popup_height = u16::try_from(visible_item_count + 2).unwrap_or(u16::MAX);
    let room_below = bounds.bottom().saturating_sub(cursor_position.y + 1);
    let popup_y = match room_below >= popup_height {
        true => cursor_position.y + 1,
        false => cursor_position.y.saturating_sub(popup_height),
    };
    let popup_x = cursor_position
        .x
        .min(editor_area.right().saturating_sub(popup_width));
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height).intersection(bounds);
    let first_item_index = (popup.selected_item_index + 1).saturating_sub(visible_item_count);
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(first_item_index)
        .take(visible_item_count)
        .map(|(item_index, item)| {
            item_line(
                item,
                content_width,
                item_index == popup.selected_item_index,
                palette,
            )
        })
        .collect();
    let block = Block::bordered().border_style(palette.border(true));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn item_width(item: &CompletionItem) -> usize {
    let label_width = item.label.chars().count();
    match &item.detail {
        Some(detail) => label_width + LABEL_DETAIL_GAP.len() + detail.chars().count(),
        None => label_width,
    }
}

fn item_line<'a>(
    item: &'a CompletionItem,
    content_width: usize,
    is_selected: bool,
    palette: &Palette,
) -> Line<'a> {
    let detail = item.detail.as_deref().unwrap_or_default();
    let padding_width = content_width
        .saturating_sub(item.label.chars().count())
        .saturating_sub(detail.chars().count());
    let spans = vec![
        Span::raw(item.label.as_str()),
        Span::raw(" ".repeat(padding_width)),
        Span::styled(detail, palette.muted()),
    ];
    match is_selected {
        true => Line::from(spans).style(palette.selected(true)),
        false => Line::from(spans),
    }
}
