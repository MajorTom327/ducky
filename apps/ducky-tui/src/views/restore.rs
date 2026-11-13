use ducky_core::ToolStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::views::files::render_entries;
use crate::views::job_log::{self, disabled_lines, render_job_state};
use crate::widgets::loading::{LoadSlot, render_loadable};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let restore = &workspace.restore;
    let palette = context.palette;
    let dump_dir = context.storage.dump_dir.display().to_string();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border(true))
        .title(Line::styled(
            format!("Restore dump — {dump_dir}"),
            palette.title(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let ToolStatus::Missing(reason) = &workspace.tools.pg_restore {
        let paragraph =
            Paragraph::new(disabled_lines("Restore", reason)).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
        return;
    }
    let [list_area, options_area, state_area, log_area] = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(3),
    ])
    .areas(inner);
    if let Some(entries) = render_loadable(
        frame,
        list_area,
        &restore.entries,
        "dumps",
        LoadSlot::Main,
        tui,
        palette,
    ) {
        match entries.is_empty() {
            true => frame.render_widget(
                Paragraph::new(format!(
                    "No dumps in {dump_dir}. Create one from Dump database."
                ))
                .style(palette.muted())
                .wrap(Wrap { trim: true }),
                list_area,
            ),
            false => render_entries(
                frame,
                list_area,
                entries,
                restore.selected_entry_index,
                palette,
            ),
        }
    }
    let options = vec![
        Line::raw(""),
        Line::raw(format!(
            "{} clean before restore (c)",
            checkbox(restore.options.clean_before_restore)
        )),
        Line::raw(format!(
            "{} skip ownership (o)",
            checkbox(restore.options.skip_ownership)
        )),
    ];
    frame.render_widget(Paragraph::new(options), options_area);
    render_job_state(
        frame,
        state_area,
        &restore.job,
        "Enter restores the selected dump",
        "restore",
        tui,
        palette,
    );
    job_log::render(
        frame,
        log_area,
        &restore.log_lines,
        tui.job_log_scroll,
        palette,
    );
}

fn checkbox(is_checked: bool) -> &'static str {
    match is_checked {
        true => "[x]",
        false => "[ ]",
    }
}
