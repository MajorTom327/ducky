use ducky_app::{CellEditor, CellInput};
use ducky_core::Cell;
use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Position, Rect, Size};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use tui_input::Input;

use crate::mouse::{MouseTarget, MouseTargets, list_row_targets};
use crate::theme::Palette;
use crate::views::completion::anchored_popup_area;
use crate::views::modal::render_input_line;

const MIN_TEXT_WIDTH: u16 = 24;
const FALLBACK_TEXT_WIDTH_PERCENT: u16 = 40;
const MAX_VISIBLE_OPTIONS: usize = 8;
const NULL_OPTION_LABEL: &str = "NULL";
const BORDER_SIZE: usize = 2;
const OPTION_PADDING: usize = 2;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    editor: &CellEditor,
    anchor: Option<Rect>,
    palette: &Palette,
    mouse_targets: &mut MouseTargets,
) -> Rect {
    match &editor.input {
        CellInput::Text(input) => render_text(frame, screen, input, anchor, palette),
        CellInput::Choice {
            options,
            selected_option_index,
        } => {
            let area = render_choice(
                frame,
                screen,
                &editor.column_name,
                options,
                *selected_option_index,
                anchor,
                palette,
            );
            mouse_targets.extend(list_row_targets(
                area.inner(Margin::new(1, 1)),
                first_visible_option_index(options.len(), *selected_option_index),
                options.len(),
                MouseTarget::ModalItem,
            ));
            area
        }
    }
}

fn first_visible_option_index(option_count: usize, selected_option_index: usize) -> usize {
    (selected_option_index + 1).saturating_sub(option_count.min(MAX_VISIBLE_OPTIONS))
}

fn render_text(
    frame: &mut Frame,
    screen: Rect,
    input: &Input,
    anchor: Option<Rect>,
    palette: &Palette,
) -> Rect {
    let area = match anchor {
        Some(cell_area) => {
            let width = cell_area.width.max(MIN_TEXT_WIDTH);
            let x = cell_area.x.min(screen.right().saturating_sub(width));
            Rect::new(x, cell_area.y, width, 1).intersection(screen)
        }
        None => screen.centered(
            Constraint::Percentage(FALLBACK_TEXT_WIDTH_PERCENT),
            Constraint::Length(1),
        ),
    };
    frame.render_widget(Clear, area);
    render_input_line(frame, area, input, palette.selected(true));
    area
}

fn render_choice(
    frame: &mut Frame,
    screen: Rect,
    column_name: &str,
    options: &[Cell],
    selected_option_index: usize,
    anchor: Option<Rect>,
    palette: &Palette,
) -> Rect {
    let title = format!(" {column_name} ");
    let content_width = options
        .iter()
        .map(|option| option_label(option).chars().count() + OPTION_PADDING)
        .chain([title.chars().count()])
        .max()
        .unwrap_or_default();
    let visible_option_count = options.len().min(MAX_VISIBLE_OPTIONS);
    let popup_size = Size::new(
        u16::try_from(content_width + BORDER_SIZE)
            .unwrap_or(u16::MAX)
            .min(screen.width),
        u16::try_from(visible_option_count + BORDER_SIZE).unwrap_or(u16::MAX),
    );
    let area = match anchor {
        Some(cell_area) => anchored_popup_area(
            Position::new(cell_area.x, cell_area.y),
            popup_size,
            screen.right(),
            screen,
        ),
        None => screen.centered(
            Constraint::Length(popup_size.width),
            Constraint::Length(popup_size.height),
        ),
    };
    let first_option_index = first_visible_option_index(options.len(), selected_option_index);
    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .skip(first_option_index)
        .take(visible_option_count)
        .map(|(option_index, option)| {
            let line = Line::raw(format!(" {} ", option_label(option)));
            match (option_index == selected_option_index, option) {
                (true, _) => line.style(palette.selected(true)),
                (false, None) => line.style(palette.muted()),
                (false, Some(_)) => line,
            }
        })
        .collect();
    let block = Block::bordered()
        .title(Span::styled(title, palette.title()))
        .border_style(palette.border(true));
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
    area
}

fn option_label(option: &Cell) -> &str {
    option.as_deref().unwrap_or(NULL_OPTION_LABEL)
}
