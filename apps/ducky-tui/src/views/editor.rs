use ducky_app::{Focus, QueryPane, QueryView};
use ducky_core::TransactionMode;
use edtui::{EditorMode, EditorTheme, EditorView, LineNumbers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use crate::editor::highlight::refresh_highlights;
use crate::theme::Palette;
use crate::tui_state::{CursorShape, TuiState};
use crate::views::{ViewContext, completion};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let Some((tab_id, query)) = context.active_query() else {
        return;
    };
    let palette = context.palette;
    let is_focused = context.workspace.focus == Focus::Main && query.pane == QueryPane::Editor;
    let editor = tui.editor(tab_id);
    let mut title_spans = vec![Span::styled(
        format!(" {} ", document_title(query)),
        palette.title(),
    )];
    if editor.is_modified(&query.document) {
        title_spans.push(Span::styled("• ", palette.title()));
    }
    let mode = editor.state.mode;
    let transaction_style = match query.transaction_mode {
        TransactionMode::Transaction => palette.muted(),
        TransactionMode::AutoCommit => palette.warning(),
    };
    let transaction_title = format!(" {} · Ctrl+t ", transaction_label(query.transaction_mode));
    let block = Block::bordered()
        .title(Line::from(title_spans))
        .title_bottom(Line::from(mode_badge(mode, is_focused, palette)))
        .title_bottom(Line::styled(transaction_title, transaction_style).right_aligned())
        .border_style(palette.border(is_focused));
    let theme = EditorTheme::default()
        .hide_status_line()
        .block(block)
        .base(Style::new())
        .selection_style(Style::new().fg(Color::Black).bg(palette.accent))
        .line_numbers_style(palette.muted())
        .hide_cursor();
    refresh_highlights(editor);
    let editor_view = EditorView::new(&mut editor.state)
        .theme(theme)
        .line_numbers(LineNumbers::Absolute);
    frame.render_widget(editor_view, area);
    if is_focused && let Some(cursor_position) = editor.state.cursor_screen_position() {
        frame.set_cursor_position(cursor_position);
        tui.cursor_shape = match mode {
            EditorMode::Insert | EditorMode::Search => CursorShape::Bar,
            EditorMode::Normal | EditorMode::Visual => CursorShape::Block,
        };
    }
    completion::render(frame, area, tui.editor(tab_id), palette);
}

pub fn transaction_label(mode: TransactionMode) -> &'static str {
    match mode {
        TransactionMode::Transaction => "one transaction",
        TransactionMode::AutoCommit => "autocommit",
    }
}

fn mode_badge(mode: EditorMode, is_focused: bool, palette: &Palette) -> Span<'static> {
    let label = format!(" {} ", mode_label(mode));
    match (mode, is_focused) {
        (_, false) => Span::styled(label, palette.muted()),
        (EditorMode::Normal, true) => Span::styled(label, palette.title()),
        (EditorMode::Insert | EditorMode::Visual | EditorMode::Search, true) => {
            Span::styled(label, palette.selected(true).add_modifier(Modifier::BOLD))
        }
    }
}

fn document_title(query: &QueryView) -> String {
    match &query.document.name {
        Some(name) => name.clone(),
        None => format!("Query {}", query.number),
    }
}

fn mode_label(mode: EditorMode) -> &'static str {
    match mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
        EditorMode::Search => "SEARCH",
    }
}
