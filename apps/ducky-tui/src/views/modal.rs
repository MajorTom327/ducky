use ducky_app::{AppState, Confirm, Modal, Prompt, RowDetail};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::keys::help::HELP_ROWS;
use crate::theme::Palette;
use crate::tui_state::TuiState;

const HELP_KEY_WIDTH: usize = 16;
const HELP_COLUMN_WIDTH: u16 = 48;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    state: &AppState,
    modal: &Modal,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let area = match modal {
        Modal::Prompt(_) => popup_area(screen, 60, Constraint::Length(3)),
        Modal::Confirm(_) => popup_area(screen, 60, Constraint::Length(6)),
        Modal::RowDetail(_) => popup_area(screen, 60, Constraint::Percentage(70)),
        Modal::SchemaPicker { .. } => {
            let schema_count = state
                .workspace()
                .map_or(0, |workspace| workspace.schema_names().len());
            let height = u16::try_from(schema_count + 2).unwrap_or(u16::MAX);
            popup_area(screen, 60, Constraint::Length(height.min(screen.height)))
        }
        Modal::Help => popup_area(screen, 90, Constraint::Percentage(90)),
    };
    frame.render_widget(Clear, area);
    match modal {
        Modal::Prompt(prompt) => render_prompt(frame, area, prompt, palette),
        Modal::Confirm(confirm) => render_confirm(frame, area, confirm, palette),
        Modal::RowDetail(row_detail) => render_row_detail(frame, area, row_detail, palette),
        Modal::SchemaPicker {
            selected_schema_index,
        } => {
            let schema_names = state
                .workspace()
                .map(|workspace| workspace.schema_names())
                .unwrap_or_default();
            render_schema_picker(frame, area, &schema_names, *selected_schema_index, palette);
        }
        Modal::Help => render_help(frame, area, palette),
    }
    tui.effects.mark_modal(area);
}

fn popup_area(screen: Rect, width_percent: u16, height: Constraint) -> Rect {
    screen.centered(Constraint::Percentage(width_percent), height)
}

fn popup_block<'a>(title: &str, palette: &Palette) -> Block<'a> {
    Block::bordered()
        .title(Span::styled(format!(" {title} "), palette.title()))
        .border_style(palette.border(true))
}

fn render_prompt(frame: &mut Frame, area: Rect, prompt: &Prompt, palette: &Palette) {
    let block = popup_block(&prompt.title, palette);
    let inner_area = block.inner(area);
    let width = usize::from(inner_area.width.max(1));
    let scroll = prompt.input.visual_scroll(width);
    let paragraph = Paragraph::new(prompt.input.value().to_owned())
        .scroll((0, u16::try_from(scroll).unwrap_or(0)))
        .block(block);
    frame.render_widget(paragraph, area);
    let cursor_offset = prompt.input.visual_cursor().saturating_sub(scroll);
    let cursor_x = inner_area
        .x
        .saturating_add(u16::try_from(cursor_offset).unwrap_or(u16::MAX));
    frame.set_cursor_position((cursor_x, inner_area.y));
}

fn render_confirm(frame: &mut Frame, area: Rect, confirm: &Confirm, palette: &Palette) {
    let block = popup_block("Confirm", palette);
    let lines = vec![
        Line::raw(confirm.message.clone()),
        Line::default(),
        Line::styled("y / Enter confirm · n / Esc cancel", palette.muted()),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}

fn render_row_detail(frame: &mut Frame, area: Rect, row_detail: &RowDetail, palette: &Palette) {
    let block = popup_block("Row detail", palette);
    let inner_area = block.inner(area);
    let name_style = Style::new().add_modifier(Modifier::BOLD);
    let mut lines = Vec::new();
    let mut selected_line_index = 0;
    for (field_index, column) in row_detail.columns.iter().enumerate() {
        let is_selected = field_index == row_detail.selected_field_index;
        if is_selected {
            selected_line_index = lines.len();
        }
        let value = row_detail.values.get(field_index).cloned().flatten();
        let mut field_lines = match value {
            Some(text) => {
                let mut value_lines = text.split('\n').map(str::to_owned);
                let first_line = value_lines.next().unwrap_or_default();
                let mut field_lines = vec![Line::from(vec![
                    Span::styled(format!("{column}: "), name_style),
                    Span::raw(first_line),
                ])];
                field_lines.extend(value_lines.map(Line::raw));
                field_lines
            }
            None => vec![Line::from(vec![
                Span::styled(format!("{column}: "), name_style),
                Span::styled("NULL", palette.null_cell()),
            ])],
        };
        if is_selected {
            field_lines = field_lines
                .into_iter()
                .map(|line| line.style(palette.selected(true)))
                .collect();
        }
        lines.extend(field_lines);
    }
    let scroll = selected_line_index.saturating_sub(usize::from(inner_area.height) / 2);
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(scroll).unwrap_or(0), 0))
        .block(block);
    frame.render_widget(paragraph, area);
}

fn render_schema_picker(
    frame: &mut Frame,
    area: Rect,
    schema_names: &[String],
    selected_schema_index: usize,
    palette: &Palette,
) {
    let block = popup_block("Schemas", palette);
    let inner_area = block.inner(area);
    let lines: Vec<Line> = schema_names
        .iter()
        .enumerate()
        .map(|(schema_index, schema_name)| {
            let line = Line::raw(format!(" {schema_name}"));
            match schema_index == selected_schema_index {
                true => line.style(palette.selected(true)),
                false => line,
            }
        })
        .collect();
    let scroll = (selected_schema_index + 1).saturating_sub(usize::from(inner_area.height));
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((u16::try_from(scroll).unwrap_or(0), 0))
            .block(block),
        area,
    );
}

fn render_help(frame: &mut Frame, area: Rect, palette: &Palette) {
    let block = popup_block("Help · Esc closes", palette);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let rows_per_column = usize::from(inner_area.height.max(1));
    let column_count = HELP_ROWS.len().div_ceil(rows_per_column).max(1);
    let constraints = vec![Constraint::Length(HELP_COLUMN_WIDTH); column_count];
    let column_areas = Layout::horizontal(constraints).split(inner_area);
    for (column_rows, column_area) in HELP_ROWS.chunks(rows_per_column).zip(column_areas.iter()) {
        let lines: Vec<Line> = column_rows
            .iter()
            .map(|(keys, description)| match keys.is_empty() {
                true => Line::styled(*description, palette.title()),
                false => Line::from(vec![
                    Span::styled(format!("{keys:<HELP_KEY_WIDTH$}"), palette.muted()),
                    Span::raw(*description),
                ]),
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), *column_area);
    }
}
