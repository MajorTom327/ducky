use ducky_app::{Focus, Loadable, TableTab, TableView};
use ducky_core::SortDirection;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::tui_state::TuiState;
use crate::views::{ViewContext, structure};
use crate::widgets::grid::{GridViewport, render_grid};
use crate::widgets::loading::{LoadSlot, render_error, render_loadable};

const FAILED_PAGE_HINT: &str = "/ edit filter · Esc clear filter · r reload";

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let Some(table) = &context.workspace.table else {
        return;
    };
    if tui.shown_relation.as_ref() != Some(&table.relation) {
        tui.table_viewport = GridViewport::default();
        tui.structure_scroll = 0;
        tui.shown_relation = Some(table.relation.clone());
    }
    let is_focused = context.workspace.focus == Focus::Main;
    let block = Block::bordered()
        .title(Span::styled(
            format!(" {}.{} ", table.relation.schema, table.relation.name),
            context.palette.title(),
        ))
        .border_style(context.palette.border(is_focused));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let [tabs_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner_area);
    frame.render_widget(Paragraph::new(tabs_line(table, context)), tabs_area);
    match table.tab {
        TableTab::Data => render_data(frame, content_area, table, context, tui, is_focused),
        TableTab::Structure => structure::render(frame, content_area, context, tui),
    }
}

fn tabs_line(table: &TableView, context: &ViewContext) -> Line<'static> {
    let tab_style = |tab: TableTab| match table.tab == tab {
        true => context.palette.title(),
        false => context.palette.muted(),
    };
    Line::from(vec![
        Span::styled("Data", tab_style(TableTab::Data)),
        Span::raw(" │ "),
        Span::styled("Structure", tab_style(TableTab::Structure)),
    ])
}

fn summary_text(table: &TableView, context: &ViewContext) -> String {
    let estimated_row_count = context
        .workspace
        .catalog
        .ready()
        .and_then(|catalog| catalog.relation(&table.relation))
        .and_then(|relation| relation.estimated_row_count);
    let mut parts = Vec::new();
    if let Some(row_count) = estimated_row_count {
        parts.push(format!("~{row_count} rows"));
    }
    parts.push(format!("page {}", table.request.page_index + 1));
    if let Some(sort) = &table.request.sort {
        let arrow = match sort.direction {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        };
        parts.push(format!("sort: {} {arrow}", sort.column));
    }
    if let Some(filter) = &table.request.filter {
        parts.push(format!("where: {filter}"));
    }
    parts.join(" · ")
}

fn render_data(
    frame: &mut Frame,
    area: Rect,
    table: &TableView,
    context: &ViewContext,
    tui: &mut TuiState,
    is_focused: bool,
) {
    let [summary_area, body_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(summary_text(table, context)).style(context.palette.muted()),
        summary_area,
    );
    if let Loadable::Failed(message) = &table.page {
        let [error_area, hint_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(body_area);
        render_error(frame, error_area, message, context.palette);
        frame.render_widget(
            Paragraph::new(FAILED_PAGE_HINT).style(context.palette.muted()),
            hint_area,
        );
        return;
    }
    let Some(page) = render_loadable(
        frame,
        body_area,
        &table.page,
        "rows",
        LoadSlot::Main,
        tui,
        context.palette,
    ) else {
        return;
    };
    render_grid(
        frame,
        body_area,
        &page.result_set,
        table.cursor,
        &mut tui.table_viewport,
        table.request.sort.as_ref(),
        is_focused,
        context.palette,
    );
}
