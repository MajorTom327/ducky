use ducky_app::{AppState, Confirm, ConfirmKind};
use ducky_core::TransactionMode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::theme::{Palette, palette};

const WIDTH_PERCENT: u16 = 60;
const SQL_PREVIEW_LINE_LIMIT: usize = 3;
const BORDER_SIZE: u16 = 2;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    state: &AppState,
    confirm: &Confirm,
    workspace_palette: &Palette,
) -> Rect {
    let popup_palette = match &confirm.kind {
        ConfirmKind::RunScript { .. } => {
            palette(state.workspace().map(|workspace| workspace.profile.kind))
        }
        ConfirmKind::OverwriteFile | ConfirmKind::Restore { .. } => *workspace_palette,
    };
    let lines = confirm_lines(state, confirm, &popup_palette);
    let popup_width = screen.width.saturating_mul(WIDTH_PERCENT) / 100;
    let inner_width = usize::from(popup_width.saturating_sub(BORDER_SIZE).max(1));
    let wrapped_row_count: usize = lines
        .iter()
        .map(|line| line.width().max(1).div_ceil(inner_width))
        .sum();
    let height = u16::try_from(wrapped_row_count)
        .unwrap_or(u16::MAX)
        .saturating_add(BORDER_SIZE)
        .min(screen.height);
    let area = screen.centered(
        Constraint::Percentage(WIDTH_PERCENT),
        Constraint::Length(height),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(Span::styled(" Confirm ", popup_palette.title()))
        .border_style(popup_palette.border(true));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
    area
}

fn confirm_lines<'a>(state: &AppState, confirm: &'a Confirm, palette: &Palette) -> Vec<Line<'a>> {
    let mut lines = vec![Line::raw(confirm.message.as_str())];
    if let ConfirmKind::RunScript { tab_id, sql } = &confirm.kind {
        lines.push(Line::default());
        lines.extend(
            sql_preview(sql)
                .into_iter()
                .map(|preview_line| Line::styled(preview_line, palette.muted())),
        );
        let transaction_mode = state
            .workspace()
            .and_then(|workspace| workspace.query(*tab_id))
            .map_or(TransactionMode::Transaction, |query| query.transaction_mode);
        let mode_label = match transaction_mode {
            TransactionMode::Transaction => "one transaction",
            TransactionMode::AutoCommit => "autocommit",
        };
        lines.push(Line::styled(format!("Mode: {mode_label}"), palette.muted()));
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        "y / Enter confirm · n / Esc cancel",
        palette.muted(),
    ));
    lines
}

fn sql_preview(sql: &str) -> Vec<String> {
    let sql_lines: Vec<&str> = sql.trim().lines().collect();
    let mut preview_lines: Vec<String> = sql_lines
        .iter()
        .take(SQL_PREVIEW_LINE_LIMIT)
        .map(|sql_line| (*sql_line).to_owned())
        .collect();
    if sql_lines.len() > SQL_PREVIEW_LINE_LIMIT {
        preview_lines.push("…".to_owned());
    }
    preview_lines
}
