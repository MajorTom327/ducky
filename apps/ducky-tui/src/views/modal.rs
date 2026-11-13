use ducky_app::{AppState, Confirm, Modal, Prompt};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::keys::help::HELP_ROWS;
use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::{palette as palette_view, parameters, row_detail};

const HELP_COLUMN_SPACING: u16 = 2;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    state: &AppState,
    modal: &Modal,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let area = match modal {
        Modal::RowDetail(detail) => row_detail::render(frame, screen, detail, palette),
        Modal::Parameters(form) => parameters::render(frame, screen, form, palette),
        Modal::Palette(command_palette) => match state.workspace() {
            Some(workspace) => palette_view::render(
                frame,
                screen,
                workspace,
                &state.profiles,
                command_palette,
                palette,
            ),
            None => return,
        },
        Modal::Prompt(prompt) => {
            let area = popup_area(screen, 60, Constraint::Length(3));
            frame.render_widget(Clear, area);
            render_prompt(frame, area, prompt, palette);
            area
        }
        Modal::Confirm(confirm) => {
            let area = popup_area(screen, 60, Constraint::Length(6));
            frame.render_widget(Clear, area);
            render_confirm(frame, area, confirm, palette);
            area
        }
        Modal::SchemaPicker {
            selected_schema_index,
        } => {
            let schema_names = state
                .workspace()
                .map(|workspace| workspace.schema_names())
                .unwrap_or_default();
            let height = u16::try_from(schema_names.len() + 2).unwrap_or(u16::MAX);
            let area = popup_area(screen, 60, Constraint::Length(height.min(screen.height)));
            frame.render_widget(Clear, area);
            render_schema_picker(frame, area, &schema_names, *selected_schema_index, palette);
            area
        }
        Modal::Help => {
            let area = popup_area(screen, 80, Constraint::Percentage(80));
            frame.render_widget(Clear, area);
            render_help(frame, area, tui, palette);
            area
        }
    };
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

fn render_help(frame: &mut Frame, area: Rect, tui: &mut TuiState, palette: &Palette) {
    let block = popup_block("Help · j/k scroll · Esc close", palette);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let help_groups = help_groups();
    let key_width = HELP_ROWS
        .iter()
        .map(|(keys, _)| keys.chars().count())
        .max()
        .unwrap_or(0);
    let row_width = HELP_ROWS
        .iter()
        .map(|(_, description)| key_width + 2 + description.chars().count())
        .max()
        .unwrap_or(0);
    let two_column_width = 2 * row_width + usize::from(HELP_COLUMN_SPACING);
    let column_count = match usize::from(inner_area.width) >= two_column_width {
        true => 2,
        false => 1,
    };
    let columns = split_groups(&help_groups, column_count);
    let column_areas = Layout::horizontal(vec![Constraint::Fill(1); columns.len()])
        .spacing(HELP_COLUMN_SPACING)
        .split(inner_area);
    let column_lines: Vec<Vec<Line>> = columns
        .iter()
        .map(|column_groups| help_lines(column_groups, key_width, palette))
        .collect();
    let tallest_column = column_lines.iter().map(Vec::len).max().unwrap_or(0);
    let max_scroll = tallest_column.saturating_sub(usize::from(inner_area.height));
    tui.help_scroll = tui
        .help_scroll
        .min(u16::try_from(max_scroll).unwrap_or(u16::MAX));
    for (lines, column_area) in column_lines.into_iter().zip(column_areas.iter()) {
        frame.render_widget(
            Paragraph::new(lines).scroll((tui.help_scroll, 0)),
            *column_area,
        );
    }
}

type HelpGroup<'a> = (&'a str, &'a [(&'a str, &'a str)]);

fn help_groups() -> Vec<HelpGroup<'static>> {
    let mut groups = Vec::new();
    let mut remaining_rows = HELP_ROWS;
    while let Some(((_, title), rest)) = remaining_rows.split_first() {
        let binding_count = rest
            .iter()
            .position(|(keys, _)| keys.is_empty())
            .unwrap_or(rest.len());
        let (bindings, next_rows) = rest.split_at(binding_count);
        groups.push((*title, bindings));
        remaining_rows = next_rows;
    }
    groups
}

fn split_groups<'a>(groups: &[HelpGroup<'a>], column_count: usize) -> Vec<Vec<HelpGroup<'a>>> {
    let total_height: usize = groups.iter().map(group_height).sum();
    let column_target = total_height.div_ceil(column_count.max(1));
    let mut columns: Vec<Vec<HelpGroup<'a>>> = vec![Vec::new()];
    let mut current_height = 0;
    for group in groups {
        let is_column_full = current_height >= column_target && columns.len() < column_count;
        if is_column_full {
            columns.push(Vec::new());
            current_height = 0;
        }
        current_height += group_height(group);
        if let Some(column) = columns.last_mut() {
            column.push(*group);
        }
    }
    columns
}

fn group_height(group: &HelpGroup) -> usize {
    group.1.len() + 2
}

fn help_lines<'a>(groups: &[HelpGroup<'a>], key_width: usize, palette: &Palette) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for (group_index, (title, bindings)) in groups.iter().enumerate() {
        if group_index > 0 {
            lines.push(Line::default());
        }
        lines.push(Line::styled(*title, palette.title()));
        lines.extend(bindings.iter().map(|(keys, description)| {
            Line::from(vec![
                Span::styled(
                    format!("{keys:<key_width$}"),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::raw(*description),
            ])
        }));
    }
    lines
}
