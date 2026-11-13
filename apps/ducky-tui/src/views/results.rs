use ducky_app::{Focus, NoticeLevel, QueryPane, QueryView, RunState};
use ducky_core::{ScriptFailure, ScriptReport, StatementOutcome, StatementResult, TransactionMode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::grid::render_grid;
use crate::widgets::loading::{LoadSlot, render_loading, render_message};

const SQL_PREVIEW_CHARS: usize = 24;
const STATEMENT_SEPARATOR: &str = "  ";

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let query = &workspace.query;
    let is_focused = workspace.focus == Focus::Main && query.pane == QueryPane::Results;
    let block = Block::bordered()
        .title(Span::styled(" Results ", palette.title()))
        .border_style(palette.border(is_focused));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    match &query.run {
        RunState::Idle => render_message(
            frame,
            inner_area,
            "F5 or Ctrl+e runs the editor content",
            palette,
        ),
        RunState::Running => render_loading(
            frame,
            inner_area,
            "script (Ctrl+c cancels)",
            LoadSlot::Main,
            tui,
            palette,
        ),
        RunState::Finished(report) => {
            render_report(frame, inner_area, query, report, is_focused, tui, palette)
        }
    }
}

fn render_report(
    frame: &mut Frame,
    area: Rect,
    query: &QueryView,
    report: &ScriptReport,
    is_focused: bool,
    tui: &mut TuiState,
    palette: &Palette,
) {
    let failure_lines = report
        .failure
        .as_ref()
        .map(|failure| failure_lines(failure, report.mode, palette))
        .unwrap_or_default();
    let failure_height = u16::try_from(failure_lines.len())
        .unwrap_or(u16::MAX)
        .min(area.height / 2);
    let [statements_area, summary_area, failure_area, body_area] =
        area.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(failure_height),
            Constraint::Min(0),
        ]));
    let statements_line = statements_line(
        &report.results,
        query.selected_result_index,
        usize::from(statements_area.width),
        palette,
    );
    frame.render_widget(Paragraph::new(statements_line), statements_area);
    let mode_label = match report.mode {
        TransactionMode::Transaction => "transaction",
        TransactionMode::AutoCommit => "autocommit",
    };
    let summary = format!("{} ms · {mode_label}", report.elapsed.as_millis());
    frame.render_widget(
        Paragraph::new(Line::styled(summary, palette.muted())),
        summary_area,
    );
    frame.render_widget(
        Paragraph::new(failure_lines).wrap(Wrap { trim: false }),
        failure_area,
    );
    match query.selected_result().map(|result| &result.outcome) {
        Some(StatementOutcome::Rows(result_set)) => render_grid(
            frame,
            body_area,
            result_set,
            query.cursor,
            &mut tui.results_viewport,
            None,
            is_focused,
            palette,
        ),
        Some(StatementOutcome::Affected(affected_count)) => render_message(
            frame,
            body_area,
            &format!("{affected_count} rows affected"),
            palette,
        ),
        None => {}
    }
}

fn failure_lines(
    failure: &ScriptFailure,
    mode: TransactionMode,
    palette: &Palette,
) -> Vec<Line<'static>> {
    let error_style = palette.notice(NoticeLevel::Error);
    let outcome = match mode {
        TransactionMode::Transaction => "Transaction rolled back",
        TransactionMode::AutoCommit => "Earlier statements were committed",
    };
    let mut lines = vec![Line::styled(
        format!("Statement {} failed", failure.statement_index + 1),
        error_style,
    )];
    lines.extend(
        failure
            .message
            .lines()
            .map(|message_line| Line::styled(message_line.to_owned(), error_style)),
    );
    lines.push(Line::styled(outcome, palette.muted()));
    lines
}

fn statements_line(
    results: &[StatementResult],
    selected_result_index: usize,
    available_width: usize,
    palette: &Palette,
) -> Line<'static> {
    let labels: Vec<String> = results
        .iter()
        .enumerate()
        .map(|(result_index, result)| statement_label(result_index, result))
        .collect();
    let first_shown_index = first_shown_index(&labels, selected_result_index, available_width);
    let mut spans = Vec::new();
    for (result_index, label) in labels.into_iter().enumerate().skip(first_shown_index) {
        if result_index > first_shown_index {
            spans.push(Span::raw(STATEMENT_SEPARATOR));
        }
        match result_index == selected_result_index {
            true => spans.push(Span::styled(label, palette.title())),
            false => spans.push(Span::styled(label, palette.muted())),
        }
    }
    Line::from(spans)
}

fn first_shown_index(
    labels: &[String],
    selected_result_index: usize,
    available_width: usize,
) -> usize {
    let mut used_width = 0;
    let mut first_index = selected_result_index.min(labels.len().saturating_sub(1));
    for (label_index, label) in labels.iter().enumerate().take(first_index + 1).rev() {
        used_width += label.chars().count() + STATEMENT_SEPARATOR.len();
        if used_width > available_width && label_index < first_index {
            break;
        }
        first_index = label_index;
    }
    first_index
}

fn statement_label(result_index: usize, result: &StatementResult) -> String {
    let sql_preview: String = result
        .sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(SQL_PREVIEW_CHARS)
        .collect();
    let outcome = match &result.outcome {
        StatementOutcome::Rows(result_set) => format!("{} rows", result_set.rows.len()),
        StatementOutcome::Affected(affected_count) => format!("{affected_count} affected"),
    };
    format!("{}: {sql_preview} ({outcome})", result_index + 1)
}
