use ducky_core::ToolStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::views::job_log::{self, disabled_lines, render_job_state};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border(context.workspace.focus == ducky_app::Focus::Main))
        .title(Line::styled(" Dump database ", palette.title()));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let tool_path = match &workspace.tools.pg_dump {
        ToolStatus::Missing(reason) => {
            let paragraph =
                Paragraph::new(disabled_lines("Dump", reason)).wrap(Wrap { trim: false });
            frame.render_widget(paragraph, inner);
            return;
        }
        ToolStatus::Available(path) => path,
    };
    let target_name = match &workspace.dump.custom_file_name {
        Some(name) => name.clone(),
        None => format!(
            "{}-<timestamp>.dump (automatic)",
            workspace.session.database
        ),
    };
    let fact = |label: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("{label:<8}"), palette.muted()),
            Span::raw(value),
        ])
    };
    let facts = vec![
        fact("Tool", tool_path.display().to_string()),
        fact(
            "Target",
            format!("{}/{target_name}", context.storage.dump_dir.display()),
        ),
        fact("Format", "custom (pg_dump -Fc)".to_owned()),
    ];
    let [facts_area, _, state_area, log_area] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(3),
    ])
    .areas(inner);
    frame.render_widget(Paragraph::new(facts).wrap(Wrap { trim: false }), facts_area);
    render_job_state(
        frame,
        state_area,
        &workspace.dump.job,
        "Enter starts the dump · e renames the file",
        "dump",
        tui,
        palette,
    );
    job_log::render(
        frame,
        log_area,
        &workspace.dump.log_lines,
        tui.job_log_scroll,
        palette,
    );
}
