use ducky_app::{AppState, Modal, Prompt};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use tui_input::Input;

use crate::mouse::{MouseTarget, MouseTargets, list_row_targets};
use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::{
    cell_editor, confirm, help, palette as palette_view, parameters, profiles, reference_picker,
    row_detail,
};

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    state: &AppState,
    modal: &Modal,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let first_modal_target_index = tui.mouse_targets.len();
    let area = match modal {
        Modal::RowDetail(detail) => row_detail::render(frame, screen, detail, palette),
        Modal::Parameters(form) => parameters::render(frame, screen, form, palette),
        Modal::ReferencePicker {
            links,
            selected_link_index,
        } => reference_picker::render(
            frame,
            screen,
            links,
            *selected_link_index,
            palette,
            &mut tui.mouse_targets,
        ),
        Modal::Palette(command_palette) => match state.workspace() {
            Some(workspace) => palette_view::render(
                frame,
                screen,
                workspace,
                command_palette,
                palette,
                &mut tui.mouse_targets,
            ),
            None => return,
        },
        Modal::ProfilePicker(picker) => {
            profiles::render_switch_popup(frame, screen, state, picker, tui, palette)
        }
        Modal::Prompt(prompt) => {
            let area = popup_area(screen, 60, Constraint::Length(3));
            frame.render_widget(Clear, area);
            render_prompt(frame, area, prompt, palette);
            area
        }
        Modal::Confirm(confirm) => confirm::render(frame, screen, state, confirm, palette),
        Modal::CellEditor(editor) => {
            let anchor = state
                .workspace()
                .and_then(|workspace| workspace.active_tab())
                .and_then(|tab| tui.table_cursor_cells.get(&tab.id).copied());
            cell_editor::render(
                frame,
                screen,
                editor,
                anchor,
                palette,
                &mut tui.mouse_targets,
            )
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
            render_schema_picker(
                frame,
                area,
                &schema_names,
                *selected_schema_index,
                palette,
                &mut tui.mouse_targets,
            );
            area
        }
        Modal::Help => help::render(frame, screen, tui, palette),
    };
    tui.mouse_targets
        .insert(first_modal_target_index, (area, MouseTarget::Modal));
    tui.effects.mark_modal(area);
}

pub fn popup_area(screen: Rect, width_percent: u16, height: Constraint) -> Rect {
    screen.centered(Constraint::Percentage(width_percent), height)
}

pub fn popup_block<'a>(title: &str, palette: &Palette) -> Block<'a> {
    Block::bordered()
        .title(Span::styled(format!(" {title} "), palette.title()))
        .border_style(palette.border(true))
}

fn render_prompt(frame: &mut Frame, area: Rect, prompt: &Prompt, palette: &Palette) {
    let block = popup_block(&prompt.title, palette);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    render_input_line(frame, inner_area, &prompt.input, Style::new());
}

pub fn render_input_line(frame: &mut Frame, area: Rect, input: &Input, style: Style) {
    let width = usize::from(area.width.max(1));
    let scroll = input.visual_scroll(width);
    let paragraph = Paragraph::new(input.value().to_owned())
        .style(style)
        .scroll((0, u16::try_from(scroll).unwrap_or(0)));
    frame.render_widget(paragraph, area);
    let cursor_offset = input.visual_cursor().saturating_sub(scroll);
    let cursor_x = area
        .x
        .saturating_add(u16::try_from(cursor_offset).unwrap_or(u16::MAX));
    frame.set_cursor_position((cursor_x, area.y));
}

fn render_schema_picker(
    frame: &mut Frame,
    area: Rect,
    schema_names: &[String],
    selected_schema_index: usize,
    palette: &Palette,
    mouse_targets: &mut MouseTargets,
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
    mouse_targets.extend(list_row_targets(
        inner_area,
        scroll,
        schema_names.len(),
        MouseTarget::ModalItem,
    ));
}
