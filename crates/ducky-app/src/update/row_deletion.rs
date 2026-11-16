use ducky_core::RelationKind;
use ducky_sql::quote_identifier;

use crate::command::Command;
use crate::state::{
    AppState, Confirm, ConfirmKind, Modal, Notice, NoticeLevel, TabContent, TabId, TableTab,
    Workspace,
};
use crate::update::row_keys::{row_keys, target_row_indexes};
use crate::update::table::{reload_page, table_views_mut};
use crate::update::{notice, workspace_mut};

pub(crate) fn confirm(state: &mut AppState) -> Vec<Command> {
    match state.workspace().and_then(deletion_confirm) {
        None => {}
        Some(Err(refusal)) => state.notice = Some(refusal),
        Some(Ok(confirm)) => state.modal = Some(Modal::Confirm(confirm)),
    }
    Vec::new()
}

pub(crate) fn cancelled(state: &mut AppState) {
    state.notice = notice(NoticeLevel::Info, "Delete cancelled");
}

pub(crate) fn deleted(
    state: &mut AppState,
    tab_id: TabId,
    result: Result<Vec<u64>, String>,
) -> Vec<Command> {
    let deleted_row_count: u64 = match result {
        Ok(deleted_row_counts) => deleted_row_counts.iter().sum(),
        Err(message) => {
            state.notice = notice(NoticeLevel::Error, format!("Delete failed: {message}"));
            return Vec::new();
        }
    };
    state.notice = notice(
        NoticeLevel::Success,
        format!("Deleted {deleted_row_count} row(s)"),
    );
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let Some(relation) = workspace
        .table_mut(tab_id)
        .map(|table| table.relation.clone())
    else {
        return Vec::new();
    };
    table_views_mut(workspace)
        .filter(|table| table.relation == relation && table.cell_drafts.is_empty())
        .map(reload_page)
        .collect()
}

fn deletion_confirm(workspace: &Workspace) -> Option<Result<Confirm, Notice>> {
    let tab = workspace.active_tab()?;
    let TabContent::Table(table) = &tab.content else {
        return None;
    };
    if table.tab != TableTab::Data {
        return None;
    }
    let result_set = &table.page.ready()?.result_set;
    let target_row_indexes = target_row_indexes(table, result_set);
    if target_row_indexes.is_empty() {
        return None;
    }
    let is_view = workspace
        .catalog
        .ready()
        .and_then(|catalog| catalog.relation(&table.relation))
        .is_some_and(|relation| {
            matches!(
                relation.kind,
                RelationKind::View | RelationKind::MaterializedView
            )
        });
    if is_view {
        return Some(Err(Notice::new(
            NoticeLevel::Warning,
            "Views are read-only",
        )));
    }
    let row_keys = match row_keys(workspace, table, result_set, &target_row_indexes) {
        Ok(row_keys) => row_keys,
        Err(message) => return Some(Err(Notice::new(NoticeLevel::Error, message))),
    };
    let sql = format!(
        "DELETE FROM {}.{} WHERE {} IN ({});",
        quote_identifier(&table.relation.schema),
        quote_identifier(&table.relation.name),
        row_keys.key_target,
        row_keys.key_tuples.join(", "),
    );
    let row_label = match target_row_indexes.len() {
        1 => "row",
        _ => "rows",
    };
    Some(Ok(Confirm {
        kind: ConfirmKind::DeleteRows {
            tab_id: tab.id,
            sql,
        },
        message: format!(
            "You are going to delete {} {row_label}. Are you sure?",
            target_row_indexes.len()
        ),
    }))
}
