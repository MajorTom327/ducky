use ducky_app::{Focus, SidebarItem};
use ducky_core::RelationKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::loading::{LoadSlot, render_loadable};

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let is_focused = workspace.focus == Focus::Sidebar;
    let bottom_title = match workspace.sidebar.filter.is_empty() {
        true => Line::styled(" s schema ", palette.muted()),
        false => Line::styled(format!(" /{} ", workspace.sidebar.filter), palette.title()),
    };
    let block = Block::bordered()
        .title(Span::styled(
            format!(" {} ", workspace.schema_name),
            palette.title(),
        ))
        .title_bottom(bottom_title)
        .border_style(palette.border(is_focused));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    if render_loadable(
        frame,
        inner_area,
        &workspace.catalog,
        "tables",
        LoadSlot::Sidebar,
        tui,
        palette,
    )
    .is_none()
    {
        return;
    }
    let sidebar_items = workspace.sidebar_items();
    if sidebar_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(" No tables", palette.muted())),
            inner_area,
        );
        return;
    }
    let active_relation = context.active_table().map(|(_, table)| &table.relation);
    let row_width = usize::from(inner_area.width);
    let selected_item_index = workspace.sidebar.selected_item_index;
    let visible_row_count = usize::from(inner_area.height).max(1);
    let first_item_index = (selected_item_index + 1).saturating_sub(visible_row_count);
    let rows: Vec<Line> = sidebar_items
        .iter()
        .enumerate()
        .skip(first_item_index)
        .take(visible_row_count)
        .map(|(item_index, item)| {
            let is_selected = item_index == selected_item_index;
            let name_style = match active_relation == Some(&item.relation) {
                true => Style::new().add_modifier(Modifier::BOLD),
                false => Style::new(),
            };
            let meta_style = match is_selected {
                true => Style::new(),
                false => palette.muted(),
            };
            let line = spread_line(
                Span::styled(item.relation.name.clone(), name_style),
                vec![Span::styled(relation_meta(item), meta_style)],
                row_width,
            );
            match is_selected {
                true => line.style(palette.selected(is_focused)),
                false => line,
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(rows), inner_area);
}

pub fn relation_meta(item: &SidebarItem) -> String {
    match item.kind {
        RelationKind::Table => item
            .estimated_row_count
            .map(compact_count)
            .unwrap_or_default(),
        RelationKind::View => "view".to_owned(),
        RelationKind::MaterializedView => "mview".to_owned(),
        RelationKind::ForeignTable => "foreign".to_owned(),
    }
}

pub fn compact_count(count: i64) -> String {
    let count = count.max(0);
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => scaled_count(count, 1_000, "k"),
        1_000_000..1_000_000_000 => scaled_count(count, 1_000_000, "M"),
        _ => scaled_count(count, 1_000_000_000, "G"),
    }
}

fn scaled_count(count: i64, unit: i64, suffix: &str) -> String {
    let tenths = count * 10 / unit;
    match tenths < 100 {
        true => format!("{}.{}{suffix}", tenths / 10, tenths % 10),
        false => format!("{}{suffix}", count / unit),
    }
}

pub fn spread_line<'a>(left: Span<'a>, right: Vec<Span<'a>>, width: usize) -> Line<'a> {
    let right_width: usize = right.iter().map(Span::width).sum();
    let left_budget = width.saturating_sub(right_width + 3);
    let left = fit_span(left, left_budget);
    let padding = width.saturating_sub(left.width() + right_width + 2);
    let mut spans = vec![Span::raw(" "), left, Span::raw(" ".repeat(padding))];
    spans.extend(right);
    spans.push(Span::raw(" "));
    Line::from(spans)
}

fn fit_span(span: Span<'_>, max_width: usize) -> Span<'_> {
    if span.width() <= max_width {
        return span;
    }
    let kept: String = span
        .content
        .chars()
        .take(max_width.saturating_sub(1))
        .collect();
    Span::styled(format!("{kept}…"), span.style)
}
