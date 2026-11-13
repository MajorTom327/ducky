use ducky_app::{Focus, QueryPane};
use ducky_core::TransactionMode;
use edtui::{EditorMode, EditorTheme, EditorView, LineNumbers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Block;

use crate::editor::highlight::refresh_highlights;
use crate::tui_state::TuiState;
use crate::views::{ViewContext, completion};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let is_focused = workspace.focus == Focus::Main && workspace.query.pane == QueryPane::Editor;
    let document_name = workspace
        .query
        .document
        .name
        .as_deref()
        .unwrap_or("untitled");
    let transaction_label = match workspace.query.transaction_mode {
        TransactionMode::Transaction => "ON",
        TransactionMode::AutoCommit => "OFF",
    };
    let title = format!(
        " SQL command — {document_name} · tx {transaction_label} · {} ",
        mode_label(tui.editor.state.mode)
    );
    let block = Block::bordered()
        .title(Span::styled(title, palette.title()))
        .border_style(palette.border(is_focused));
    let mut theme = EditorTheme::default()
        .hide_status_line()
        .block(block)
        .base(Style::new())
        .cursor_style(Style::new().add_modifier(Modifier::REVERSED))
        .selection_style(Style::new().fg(Color::Black).bg(palette.accent))
        .line_numbers_style(palette.muted());
    if !is_focused {
        theme = theme.hide_cursor();
    }
    refresh_highlights(&mut tui.editor);
    let editor_view = EditorView::new(&mut tui.editor.state)
        .theme(theme)
        .line_numbers(LineNumbers::Absolute);
    frame.render_widget(editor_view, area);
    completion::render(frame, area, tui, palette);
}

fn mode_label(mode: EditorMode) -> &'static str {
    match mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
        EditorMode::Search => "SEARCH",
    }
}
