use chrono::{DateTime, Local};
use ducky_core::FileEntry;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::loading::{LoadSlot, render_loadable};
use crate::widgets::sql_text::highlighted_sql;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let [list_area, preview_area] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).areas(area);
    let files = &context.workspace.files;
    let palette = context.palette;
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border(context.workspace.focus == ducky_app::Focus::Main))
        .title(Line::styled(
            format!(" SQL files — {} ", context.storage.sql_dir.display()),
            palette.title(),
        ));
    let list_inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);
    if let Some(entries) = render_loadable(
        frame,
        list_inner,
        &files.entries,
        "SQL files",
        LoadSlot::Main,
        tui,
        palette,
    ) {
        match entries.is_empty() {
            true => frame.render_widget(
                Paragraph::new("No .sql files yet. Save a query with Ctrl+s in SQL command.")
                    .style(palette.muted())
                    .wrap(Wrap { trim: true }),
                list_inner,
            ),
            false => render_entries(
                frame,
                list_inner,
                entries,
                files.selected_entry_index,
                palette,
            ),
        }
    }
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border(false))
        .title(Line::styled(" Preview ", palette.title()));
    let preview_inner = preview_block.inner(preview_area);
    frame.render_widget(preview_block, preview_area);
    if let Some(preview) = render_loadable(
        frame,
        preview_inner,
        &files.preview,
        "preview",
        LoadSlot::Main,
        tui,
        palette,
    ) {
        let paragraph = Paragraph::new(highlighted_sql(preview)).scroll((tui.preview_scroll, 0));
        frame.render_widget(paragraph, preview_inner);
    }
}

pub fn render_entries(
    frame: &mut Frame,
    area: Rect,
    entries: &[FileEntry],
    selected_entry_index: usize,
    palette: &Palette,
) {
    let visible_height = usize::from(area.height).max(1);
    let first_entry_index = selected_entry_index.saturating_sub(visible_height - 1);
    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .skip(first_entry_index)
        .take(visible_height)
        .map(|(entry_index, entry)| {
            let row_style = match entry_index == selected_entry_index {
                true => palette.selected(true),
                false => Style::new(),
            };
            entry_line(entry, palette).style(row_style)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn entry_line(entry: &FileEntry, palette: &Palette) -> Line<'static> {
    let modified_text = entry
        .modified_at
        .map(|modified_at| {
            DateTime::<Local>::from(modified_at)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default();
    Line::from(vec![
        Span::raw(entry.name.clone()),
        Span::styled(
            format!("  {}  {modified_text}", human_size(entry.size_bytes)),
            palette.muted(),
        ),
    ])
}

fn human_size(size_bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = size_bytes as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    match unit_index {
        0 => format!("{size_bytes} B"),
        _ => format!("{size:.1} {}", UNITS[unit_index]),
    }
}
