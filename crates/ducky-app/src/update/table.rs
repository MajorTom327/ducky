use ducky_core::{Page, PageRequest, RelationDetails, RelationRef, Sort, SortDirection};

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::TableMsg;
use crate::state::{
    AppState, Focus, Modal, NoticeLevel, Prompt, PromptKind, RowDetail, TabContent, TableTab,
    TableView, Workspace,
};
use crate::update::cell_edits::{self, unsaved_drafts_refusal};
use crate::update::grid::{cell_text, clamp_cursor, move_cursor, place_cursor};
use crate::update::{cell_editor, notice, references, row_deletion, selection, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: TableMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let Some(table) = workspace.active_table_mut() else {
        return Vec::new();
    };
    let is_blocked_by_drafts = match msg {
        TableMsg::NextPage
        | TableMsg::PreviousPage
        | TableMsg::CycleSort
        | TableMsg::StartFilter
        | TableMsg::Reload
        | TableMsg::DeleteSelectedRows => true,
        TableMsg::ClearFilter => table.request.filter.is_some(),
        _ => false,
    };
    if is_blocked_by_drafts && let Some(refusal) = unsaved_drafts_refusal(table) {
        state.notice = Some(refusal);
        return Vec::new();
    }
    match msg {
        TableMsg::GoToReferenced => references::go_to_referenced(state),
        TableMsg::FindReferencing => references::find_referencing(state),
        TableMsg::ToggleRowMark => selection::toggle_row_mark(table),
        TableMsg::ToggleVisualSelection => selection::toggle_visual_selection(table),
        TableMsg::ClearSelection => {
            table.clear_selection();
            Vec::new()
        }
        TableMsg::EditSelectedRows => selection::edit_selected_rows(state),
        TableMsg::DeleteSelectedRows => row_deletion::confirm(state),
        TableMsg::ChangeCell => cell_editor::change_cell(state),
        TableMsg::SetCellNull => cell_editor::set_cell_null(state),
        TableMsg::RevertCell => cell_edits::revert(table),
        TableMsg::DiscardDrafts => {
            if let Some(discarded_notice) = cell_edits::discard(table) {
                state.notice = Some(discarded_notice);
            }
            Vec::new()
        }
        TableMsg::SaveDrafts => cell_edits::save(state),
        TableMsg::ToggleTab => toggle_tab(table),
        TableMsg::Move(grid_move) => {
            if let Some(page) = table.page.ready() {
                move_cursor(&mut table.cursor, &page.result_set, grid_move);
            }
            Vec::new()
        }
        TableMsg::SelectCell(cell) => {
            if let Some(page) = table.page.ready() {
                place_cursor(&mut table.cursor, &page.result_set, cell);
            }
            workspace.focus = Focus::Main;
            Vec::new()
        }
        TableMsg::NextPage => {
            if !table.page.ready().is_some_and(|page| page.has_next_page) {
                return Vec::new();
            }
            table.request.page_index += 1;
            table.cursor.row_index = 0;
            vec![reload_page(table)]
        }
        TableMsg::PreviousPage => {
            if table.request.page_index == 0 {
                return Vec::new();
            }
            table.request.page_index -= 1;
            table.cursor.row_index = 0;
            vec![reload_page(table)]
        }
        TableMsg::CycleSort => cycle_sort(table),
        TableMsg::StartFilter => {
            let current_filter = table.request.filter.clone().unwrap_or_default();
            state.modal = Some(Modal::Prompt(Prompt::new(
                PromptKind::TableFilter,
                "WHERE",
                current_filter,
            )));
            Vec::new()
        }
        TableMsg::ClearFilter => {
            if table.request.filter.is_none() {
                return Vec::new();
            }
            apply_filter(table, None)
        }
        TableMsg::Reload => {
            let mut commands = vec![reload_page(table)];
            if table.tab == TableTab::Structure {
                table.details = Loadable::Loading;
                commands.push(Command::LoadDetails(table.relation.clone()));
            }
            commands
        }
        TableMsg::OpenRowDetail => {
            if let Some(row_detail) = row_detail(table) {
                state.modal = Some(Modal::RowDetail(row_detail));
            }
            Vec::new()
        }
        TableMsg::CopyCell => {
            let Some(value) = table
                .page
                .ready()
                .and_then(|page| cell_text(&page.result_set, table.cursor))
            else {
                return Vec::new();
            };
            state.notice = notice(NoticeLevel::Info, "Copied");
            vec![Command::CopyToClipboard(value)]
        }
    }
}

pub(crate) fn reload_page(table: &mut TableView) -> Command {
    table.page = Loadable::Loading;
    table.cell_drafts.clear();
    table.clear_selection();
    Command::LoadPage(table.request.clone())
}

pub(crate) fn apply_filter(table: &mut TableView, filter: Option<String>) -> Vec<Command> {
    table.request.filter = filter;
    table.request.page_index = 0;
    table.cursor.row_index = 0;
    vec![reload_page(table)]
}

fn toggle_tab(table: &mut TableView) -> Vec<Command> {
    match table.tab {
        TableTab::Structure => {
            table.tab = TableTab::Data;
            Vec::new()
        }
        TableTab::Data => {
            table.tab = TableTab::Structure;
            if !matches!(table.details, Loadable::Idle | Loadable::Failed(_)) {
                return Vec::new();
            }
            table.details = Loadable::Loading;
            vec![Command::LoadDetails(table.relation.clone())]
        }
    }
}

fn cycle_sort(table: &mut TableView) -> Vec<Command> {
    let Some(column_name) = table
        .page
        .ready()
        .and_then(|page| page.result_set.columns.get(table.cursor.column_index))
        .cloned()
    else {
        return Vec::new();
    };
    let next_direction = match &table.request.sort {
        Some(sort) if sort.column == column_name => match sort.direction {
            SortDirection::Ascending => Some(SortDirection::Descending),
            SortDirection::Descending => None,
        },
        _ => Some(SortDirection::Ascending),
    };
    table.request.sort = next_direction.map(|direction| Sort {
        column: column_name,
        direction,
    });
    table.request.page_index = 0;
    table.cursor.row_index = 0;
    vec![reload_page(table)]
}

fn row_detail(table: &TableView) -> Option<RowDetail> {
    let result_set = &table.page.ready()?.result_set;
    let values = result_set.rows.get(table.cursor.row_index)?.clone();
    Some(RowDetail {
        columns: result_set.columns.clone(),
        values,
        selected_field_index: table.cursor.column_index,
    })
}

pub(crate) fn table_views_mut(workspace: &mut Workspace) -> impl Iterator<Item = &mut TableView> {
    workspace
        .tabs
        .iter_mut()
        .filter_map(|tab| match &mut tab.content {
            TabContent::Table(table) => Some(table),
            _ => None,
        })
}

pub(crate) fn page_loaded(
    state: &mut AppState,
    request: PageRequest,
    result: Result<Page, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    for table in table_views_mut(workspace)
        .filter(|table| table.request == request && table.cell_drafts.is_empty())
    {
        table.page = Loadable::from_result(result.clone());
        table.clear_selection();
        if let Some(page) = table.page.ready() {
            clamp_cursor(&mut table.cursor, &page.result_set);
        }
    }
    Vec::new()
}

pub(crate) fn details_loaded(
    state: &mut AppState,
    relation: RelationRef,
    result: Result<RelationDetails, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    for table in table_views_mut(workspace).filter(|table| table.relation == relation) {
        table.details = Loadable::from_result(result.clone());
    }
    Vec::new()
}

pub(crate) fn reload_all_pages(workspace: &mut Workspace) -> Vec<Command> {
    table_views_mut(workspace).map(reload_page).collect()
}
