use ducky_app::{CommandPalette, PaletteAction, PaletteItem, Workspace};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::theme::Palette;
use crate::views::sidebar::spread_line;

const INPUT_PREFIX: &str = " › ";
const CHROME_ROWS: u16 = 4;
const MIN_LIST_ROWS: u16 = 3;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    workspace: &Workspace,
    command_palette: &CommandPalette,
    palette: &Palette,
) -> Rect {
    let query = command_palette.input.value();
    let palette_items = workspace.palette_items(query);
    let area = popup_area(screen, palette_items.len());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(Span::styled(" Go to ", palette.title()))
        .border_style(palette.border(true));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let [input_area, rule_area, list_area, footer_area] = inner_area.layout(&Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ]));
    render_input(frame, input_area, command_palette, palette);
    frame.render_widget(
        Paragraph::new(Line::styled(
            "─".repeat(usize::from(rule_area.width)),
            palette.muted(),
        )),
        rule_area,
    );
    render_items(
        frame,
        list_area,
        &ItemList {
            palette_items: &palette_items,
            selected_item_index: command_palette.selected_item_index,
        },
        palette,
    );
    frame.render_widget(
        Paragraph::new(Line::styled(
            " Enter open · ↑↓ move · Esc close",
            palette.muted(),
        )),
        footer_area,
    );
    area
}

fn popup_area(screen: Rect, item_count: usize) -> Rect {
    let max_list_rows = (screen.height / 3 * 2)
        .saturating_sub(CHROME_ROWS + 2)
        .max(MIN_LIST_ROWS);
    let list_rows = u16::try_from(item_count)
        .unwrap_or(u16::MAX)
        .clamp(1, max_list_rows);
    let height = (list_rows + CHROME_ROWS + 2).min(screen.height);
    let column = screen.centered_horizontally(Constraint::Percentage(60));
    Rect {
        y: screen.y + screen.height / 8,
        height: height.min(screen.height.saturating_sub(screen.height / 8)),
        ..column
    }
}

fn render_input(
    frame: &mut Frame,
    area: Rect,
    command_palette: &CommandPalette,
    palette: &Palette,
) {
    let prefix_width = u16::try_from(INPUT_PREFIX.chars().count()).unwrap_or(0);
    let input_width = usize::from(area.width.saturating_sub(prefix_width).max(1));
    let scroll = command_palette.input.visual_scroll(input_width);
    let visible_query: String = command_palette.input.value().chars().skip(scroll).collect();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(INPUT_PREFIX, palette.title()),
            Span::raw(visible_query),
        ])),
        area,
    );
    let cursor_offset = command_palette.input.visual_cursor().saturating_sub(scroll);
    let cursor_x = area
        .x
        .saturating_add(prefix_width)
        .saturating_add(u16::try_from(cursor_offset).unwrap_or(u16::MAX));
    frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(1)), area.y));
}

struct ItemList<'a> {
    palette_items: &'a [PaletteItem],
    selected_item_index: usize,
}

fn render_items(frame: &mut Frame, area: Rect, item_list: &ItemList, palette: &Palette) {
    let palette_items = item_list.palette_items;
    if palette_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(" No matches", palette.muted())),
            area,
        );
        return;
    }
    let selected_item_index = item_list.selected_item_index.min(palette_items.len() - 1);
    let visible_row_count = usize::from(area.height).max(1);
    let first_item_index = (selected_item_index + 1).saturating_sub(visible_row_count);
    let row_width = usize::from(area.width);
    let rows: Vec<Line> = palette_items
        .iter()
        .enumerate()
        .skip(first_item_index)
        .take(visible_row_count)
        .map(|(item_index, item)| {
            let is_selected = item_index == selected_item_index;
            let line = spread_line(
                Span::raw(item.label.clone()),
                item_trailer(item, is_selected, palette),
                row_width,
            );
            match is_selected {
                true => line.style(palette.selected(true)),
                false => line,
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(rows), area);
}

fn item_trailer(item: &PaletteItem, is_selected: bool, palette: &Palette) -> Vec<Span<'static>> {
    let (detail_style, shortcut_style) = match is_selected {
        true => (Style::new(), Style::new()),
        false => (palette.muted(), palette.title()),
    };
    let mut spans = Vec::new();
    if let Some(detail) = &item.detail {
        spans.push(Span::styled(detail.clone(), detail_style));
    }
    if let Some(shortcut) = item.command_alias.or_else(|| action_shortcut(&item.action)) {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(shortcut, shortcut_style));
    }
    spans
}

pub fn action_shortcut(action: &PaletteAction) -> Option<&'static str> {
    match action {
        PaletteAction::NewQuery => Some("Ctrl+n"),
        PaletteAction::SqlFiles => Some("Ctrl+o"),
        PaletteAction::ChooseSchema => Some("s"),
        PaletteAction::RefreshCatalog => Some("r"),
        PaletteAction::Reconnect => Some("R"),
        PaletteAction::Disconnect => Some("D"),
        PaletteAction::Help => Some("?"),
        PaletteAction::SwitchProfile => Some("Space p"),
        PaletteAction::SwitchTab(_)
        | PaletteAction::Dump
        | PaletteAction::Restore
        | PaletteAction::Quit
        | PaletteAction::OpenRelation(_) => None,
    }
}
