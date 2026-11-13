use ducky_app::{AppState, ProfilePicker};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::{Block, Clear};

use crate::theme::{Palette, palette};
use crate::tui_state::TuiState;
use crate::views::profile_picker;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    picker: &ProfilePicker,
    tui: &mut TuiState,
) {
    let neutral_palette = palette(None);
    let container = Rect {
        height: area.height.saturating_sub(2),
        y: area.y.saturating_add(1),
        ..area
    };
    let picker_area = profile_picker::fitted_area(container, &state.profiles, picker);
    let block = Block::bordered()
        .title(Span::styled(" Connect ", neutral_palette.title()))
        .border_style(neutral_palette.border(true));
    let inner_area = block.inner(picker_area);
    frame.render_widget(block, picker_area);
    profile_picker::render(
        frame,
        inner_area,
        &state.profiles,
        picker,
        None,
        tui,
        &neutral_palette,
    );
}

pub fn render_switch_popup(
    frame: &mut Frame,
    screen: Rect,
    state: &AppState,
    picker: &ProfilePicker,
    tui: &mut TuiState,
    workspace_palette: &Palette,
) -> Rect {
    let area = profile_picker::fitted_area(screen, &state.profiles, picker);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(Span::styled(" Switch profile ", workspace_palette.title()))
        .border_style(workspace_palette.border(true));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    profile_picker::render(
        frame,
        inner_area,
        &state.profiles,
        picker,
        state.workspace().map(|workspace| workspace.profile_index),
        tui,
        &palette(None),
    );
    area
}
