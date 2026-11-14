use ducky_app::{Focus, Loadable, TabId, TableTab, TableView};
use ducky_core::SortDirection;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

use crate::keys::sequence::GridPane;
use crate::mouse::MouseTarget;
use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::{ViewContext, structure};
use crate::widgets::grid::{GridOptions, render_grid};
use crate::widgets::loading::{LoadSlot, render_error, render_loadable};

const FAILED_PAGE_HINT: &str = "/ edit filter · Esc clear filter · r reload";
const DATA_LABEL_WIDTH: u16 = 4;
const TAB_SEPARATOR_WIDTH: u16 = 3;
const STRUCTURE_LABEL_WIDTH: u16 = 9;
const TABS_LINE_WIDTH: u16 = DATA_LABEL_WIDTH + TAB_SEPARATOR_WIDTH + STRUCTURE_LABEL_WIDTH + 2;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    context: &ViewContext,
    tab_id: TabId,
    table: &TableView,
    tui: &mut TuiState,
) {
    let palette = context.palette;
    let is_focused = context.workspace.focus == Focus::Main;
    let mut block = Block::bordered()
        .padding(Padding::horizontal(1))
        .border_style(palette.border(is_focused))
        .title_top(Line::styled(
            format!(" {}.{} ", table.relation.schema, table.relation.name),
            palette.title(),
        ))
        .title_top(tabs_line(table, palette).right_aligned());
    if table.tab == TableTab::Data {
        if let Some(summary) = summary_text(table) {
            block = block.title_bottom(Line::styled(summary, palette.muted()));
        }
        block = block.title_bottom(
            Line::styled(
                format!(" page {} ", table.request.page_index + 1),
                palette.muted(),
            )
            .right_aligned(),
        );
    }
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    push_table_tab_targets(area, tui);
    match table.tab {
        TableTab::Data => render_data(frame, inner_area, tab_id, table, context, tui),
        TableTab::Structure => structure::render(frame, inner_area, tab_id, table, context, tui),
    }
}

fn tabs_line(table: &TableView, palette: &Palette) -> Line<'static> {
    let tab_style = |tab: TableTab| match table.tab == tab {
        true => palette.title(),
        false => palette.muted(),
    };
    Line::from(vec![
        Span::raw(" "),
        Span::styled("Data", tab_style(TableTab::Data)),
        Span::styled(" · ", palette.muted()),
        Span::styled("Structure", tab_style(TableTab::Structure)),
        Span::raw(" "),
    ])
}

fn push_table_tab_targets(area: Rect, tui: &mut TuiState) {
    let data_label_x = area.right().saturating_sub(TABS_LINE_WIDTH);
    let structure_label_x = data_label_x + DATA_LABEL_WIDTH + TAB_SEPARATOR_WIDTH;
    tui.mouse_targets.push((
        Rect::new(data_label_x, area.y, DATA_LABEL_WIDTH, 1),
        MouseTarget::TableTab(TableTab::Data),
    ));
    tui.mouse_targets.push((
        Rect::new(structure_label_x, area.y, STRUCTURE_LABEL_WIDTH, 1),
        MouseTarget::TableTab(TableTab::Structure),
    ));
}

fn summary_text(table: &TableView) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(page) = table.page.ready() {
        let first_row_number = first_row_number(table);
        let row_count = page.result_set.rows.len();
        parts.push(match row_count {
            0 => "no rows".to_owned(),
            _ => format!(
                "rows {first_row_number}–{}",
                first_row_number + row_count - 1
            ),
        });
    }
    if let Some(sort) = &table.request.sort {
        let arrow = match sort.direction {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        };
        parts.push(format!("sort {} {arrow}", sort.column));
    }
    if let Some(filter) = &table.request.filter {
        parts.push(format!("where {filter}"));
    }
    let selected_row_count = table.selected_row_indexes().len();
    if selected_row_count > 0 {
        parts.push(format!("{selected_row_count} selected"));
    }
    if table.is_saving_drafts {
        parts.push("saving…".to_owned());
    } else if !table.cell_drafts.is_empty() {
        parts.push(format!("{} unsaved", table.cell_drafts.len()));
    }
    match parts.is_empty() {
        true => None,
        false => Some(format!(" {} ", parts.join(" · "))),
    }
}

fn first_row_number(table: &TableView) -> usize {
    table.request.page_index * table.request.page_size + 1
}

fn render_data(
    frame: &mut Frame,
    area: Rect,
    tab_id: TabId,
    table: &TableView,
    context: &ViewContext,
    tui: &mut TuiState,
) {
    if let Loadable::Failed(message) = &table.page {
        let [error_area, hint_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        render_error(frame, error_area, message, context.palette);
        frame.render_widget(
            Paragraph::new(FAILED_PAGE_HINT).style(context.palette.muted()),
            hint_area,
        );
        return;
    }
    let Some(page) = render_loadable(
        frame,
        area,
        &table.page,
        "rows",
        LoadSlot::Main,
        tui,
        context.palette,
    ) else {
        return;
    };
    let linked_columns: Vec<String> = context
        .workspace
        .catalog
        .ready()
        .and_then(|catalog| catalog.relation(&table.relation))
        .map(|relation| {
            relation
                .foreign_keys
                .iter()
                .flat_map(|link| link.columns.iter().cloned())
                .collect()
        })
        .unwrap_or_default();
    let selected_row_indexes = table.selected_row_indexes();
    let options = GridOptions {
        cursor: table.cursor,
        sort: table.request.sort.as_ref(),
        first_row_number: first_row_number(table),
        is_focused: context.workspace.focus == Focus::Main,
        linked_columns: &linked_columns,
        selected_row_indexes: &selected_row_indexes,
        draft_cells: &table.cell_drafts,
        pane: GridPane::TableData,
    };
    let cursor_cell_area = render_grid(
        frame,
        area,
        &page.result_set,
        tui.table_viewports.entry(tab_id).or_default(),
        options,
        context.palette,
        &mut tui.mouse_targets,
    );
    match cursor_cell_area {
        Some(cell_area) => tui.table_cursor_cells.insert(tab_id, cell_area),
        None => tui.table_cursor_cells.remove(&tab_id),
    };
}
