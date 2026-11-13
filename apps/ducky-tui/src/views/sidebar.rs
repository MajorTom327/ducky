use ducky_app::{Focus, SidebarAction, SidebarItem, Workspace};
use ducky_core::{RelationKind, ToolStatus};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::loading::{LoadSlot, render_loadable};

const ACTION_COUNT: usize = 4;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let is_focused = workspace.focus == Focus::Sidebar;
    let block = Block::bordered()
        .title(Span::styled(
            format!(" Schema: {} ", workspace.schema_name),
            palette.title(),
        ))
        .border_style(palette.border(is_focused));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let has_filter = !workspace.sidebar.filter.is_empty();
    let [rows_area, filter_area] = inner_area.layout(&Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(u16::from(has_filter)),
    ]));
    if has_filter {
        frame.render_widget(
            Paragraph::new(Line::styled(
                format!("/{}", workspace.sidebar.filter),
                palette.title(),
            )),
            filter_area,
        );
    }
    let sidebar_items = workspace.sidebar_items();
    let mut display_rows: Vec<Line> = sidebar_items
        .iter()
        .enumerate()
        .map(|(item_index, item)| {
            let line = item_line(item, workspace, palette);
            match item_index == workspace.sidebar.selected_item_index {
                true => line.style(palette.selected(is_focused)),
                false => line,
            }
        })
        .collect();
    let separator = Line::styled("── Tables & views ──", palette.muted());
    display_rows.insert(ACTION_COUNT.min(display_rows.len()), separator);
    let selected_row_index = match workspace.sidebar.selected_item_index < ACTION_COUNT {
        true => workspace.sidebar.selected_item_index,
        false => workspace.sidebar.selected_item_index + 1,
    };
    if workspace.catalog.ready().is_none() {
        let [actions_area, catalog_area] = rows_area.layout(&Layout::vertical([
            Constraint::Length(u16::try_from(display_rows.len()).unwrap_or(u16::MAX)),
            Constraint::Min(0),
        ]));
        frame.render_widget(Paragraph::new(display_rows), actions_area);
        render_loadable(
            frame,
            catalog_area,
            &workspace.catalog,
            "tables",
            LoadSlot::Sidebar,
            tui,
            palette,
        );
        return;
    }
    let visible_row_count = usize::from(rows_area.height).max(1);
    let first_row_index = (selected_row_index + 1).saturating_sub(visible_row_count);
    let visible_rows: Vec<Line> = display_rows
        .into_iter()
        .skip(first_row_index)
        .take(visible_row_count)
        .collect();
    frame.render_widget(Paragraph::new(visible_rows), rows_area);
}

fn item_line(item: &SidebarItem, workspace: &Workspace, palette: &Palette) -> Line<'static> {
    match item {
        SidebarItem::Action(action) => {
            let label = action_label(*action);
            match action_tool(*action, workspace) {
                Some(ToolStatus::Missing(_)) => {
                    Line::styled(format!(" {label} (unavailable)"), palette.muted())
                }
                Some(ToolStatus::Available(_)) | None => Line::raw(format!(" {label}")),
            }
        }
        SidebarItem::Relation { relation, kind } => {
            let style = match kind {
                RelationKind::View | RelationKind::MaterializedView => {
                    Style::new().add_modifier(Modifier::ITALIC)
                }
                RelationKind::Table | RelationKind::ForeignTable => Style::new(),
            };
            Line::styled(format!(" {}", relation.name), style)
        }
    }
}

pub fn action_label(action: SidebarAction) -> &'static str {
    match action {
        SidebarAction::SqlCommand => "SQL command",
        SidebarAction::SqlFiles => "SQL files",
        SidebarAction::Dump => "Dump database",
        SidebarAction::Restore => "Restore dump",
    }
}

fn action_tool(action: SidebarAction, workspace: &Workspace) -> Option<&ToolStatus> {
    match action {
        SidebarAction::Dump => Some(&workspace.tools.pg_dump),
        SidebarAction::Restore => Some(&workspace.tools.pg_restore),
        SidebarAction::SqlCommand | SidebarAction::SqlFiles => None,
    }
}
