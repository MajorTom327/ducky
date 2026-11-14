use ducky_core::{Cell, ProfileKind};
use ducky_sql::{quote_identifier, quote_literal};

use crate::command::Command;
use crate::state::{
    AppState, Confirm, ConfirmKind, Modal, Notice, NoticeLevel, TabContent, TabId, TableView,
    Workspace,
};
use crate::update::row_keys::row_keys;
use crate::update::run_confirmation::profile_description;
use crate::update::table::reload_page;
use crate::update::{notice, workspace_mut};

const NULL_LITERAL: &str = "NULL";

struct SaveRequest {
    tab_id: TabId,
    sql: String,
    confirmation_message: Option<String>,
}

pub(crate) fn unsaved_drafts_refusal(table: &TableView) -> Option<Notice> {
    if table.cell_drafts.is_empty() {
        return None;
    }
    notice(
        NoticeLevel::Warning,
        format!(
            "Save (Ctrl+s) or discard (U) the {} unsaved change(s) first",
            table.cell_drafts.len()
        ),
    )
}

pub(crate) fn revert(table: &mut TableView) -> Vec<Command> {
    table.cell_drafts.remove(&table.cursor);
    Vec::new()
}

pub(crate) fn discard(table: &mut TableView) -> Option<Notice> {
    if table.cell_drafts.is_empty() {
        return None;
    }
    let discarded_draft_count = table.cell_drafts.len();
    table.cell_drafts.clear();
    notice(
        NoticeLevel::Info,
        format!("Discarded {discarded_draft_count} change(s)"),
    )
}

pub(crate) fn save(state: &mut AppState) -> Vec<Command> {
    let save_request = match state.workspace().and_then(save_request) {
        None => return Vec::new(),
        Some(Err(message)) => {
            state.notice = notice(NoticeLevel::Error, message);
            return Vec::new();
        }
        Some(Ok(save_request)) => save_request,
    };
    let SaveRequest {
        tab_id,
        sql,
        confirmation_message,
    } = save_request;
    match confirmation_message {
        None => run(state, tab_id, sql),
        Some(message) => {
            state.modal = Some(Modal::Confirm(Confirm {
                kind: ConfirmKind::SaveCellEdits { tab_id, sql },
                message,
            }));
            Vec::new()
        }
    }
}

pub(crate) fn run(state: &mut AppState, tab_id: TabId, sql: String) -> Vec<Command> {
    let Some(table) =
        workspace_mut(&mut state.screen).and_then(|workspace| workspace.table_mut(tab_id))
    else {
        return Vec::new();
    };
    if table.is_saving_drafts {
        return Vec::new();
    }
    table.is_saving_drafts = true;
    vec![Command::SaveCellEdits { tab_id, sql }]
}

pub(crate) fn cancelled(state: &mut AppState) {
    state.notice = notice(NoticeLevel::Info, "Save cancelled");
}

pub(crate) fn saved(
    state: &mut AppState,
    tab_id: TabId,
    result: Result<Vec<u64>, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let table = workspace.table_mut(tab_id);
    let updated_row_counts = match result {
        Ok(updated_row_counts) => updated_row_counts,
        Err(message) => {
            state.notice = notice(NoticeLevel::Error, format!("Save failed: {message}"));
            if let Some(table) = table {
                table.is_saving_drafts = false;
            }
            return Vec::new();
        }
    };
    let edited_row_count = updated_row_counts.len();
    state.notice = match updated_row_counts
        .iter()
        .all(|updated_row_count| *updated_row_count == 1)
    {
        true => notice(
            NoticeLevel::Success,
            format!("Saved {edited_row_count} row(s)"),
        ),
        false => notice(
            NoticeLevel::Warning,
            format!(
                "Saved: {} row(s) updated for {edited_row_count} edited row(s)",
                updated_row_counts.iter().sum::<u64>()
            ),
        ),
    };
    let Some(table) = table else {
        return Vec::new();
    };
    table.is_saving_drafts = false;
    vec![reload_page(table)]
}

fn save_request(workspace: &Workspace) -> Option<Result<SaveRequest, String>> {
    let tab = workspace.active_tab()?;
    let TabContent::Table(table) = &tab.content else {
        return None;
    };
    if table.cell_drafts.is_empty() || table.is_saving_drafts {
        return None;
    }
    let mut drafted_row_indexes: Vec<usize> = table
        .cell_drafts
        .keys()
        .map(|cursor| cursor.row_index)
        .collect();
    drafted_row_indexes.dedup();
    let sql = match save_sql(workspace, table, &drafted_row_indexes)? {
        Ok(sql) => sql,
        Err(message) => return Some(Err(message)),
    };
    let confirmation_message = match workspace.profile.kind {
        ProfileKind::Local => None,
        ProfileKind::Staging | ProfileKind::Prod => Some(format!(
            "Save {} change(s) in {} row(s) on {}?",
            table.cell_drafts.len(),
            drafted_row_indexes.len(),
            profile_description(workspace)
        )),
    };
    Some(Ok(SaveRequest {
        tab_id: tab.id,
        sql,
        confirmation_message,
    }))
}

fn save_sql(
    workspace: &Workspace,
    table: &TableView,
    drafted_row_indexes: &[usize],
) -> Option<Result<String, String>> {
    let result_set = &table.page.ready()?.result_set;
    let row_keys = match row_keys(workspace, table, result_set, drafted_row_indexes) {
        Ok(row_keys) => row_keys,
        Err(message) => return Some(Err(message)),
    };
    let relation_name = format!(
        "{}.{}",
        quote_identifier(&table.relation.schema),
        quote_identifier(&table.relation.name)
    );
    let statements: Vec<String> = drafted_row_indexes
        .iter()
        .zip(&row_keys.key_tuples)
        .map(|(row_index, key_tuple)| {
            let assignments: Vec<String> = table
                .cell_drafts
                .iter()
                .filter(|(cursor, _)| cursor.row_index == *row_index)
                .filter_map(|(cursor, draft_value)| {
                    let column_name = result_set.columns.get(cursor.column_index)?;
                    Some(format!(
                        "{} = {}",
                        quote_identifier(column_name),
                        sql_value(draft_value)
                    ))
                })
                .collect();
            format!(
                "UPDATE {relation_name} SET {} WHERE {} = {key_tuple};",
                assignments.join(", "),
                row_keys.key_target
            )
        })
        .collect();
    Some(Ok(statements.join("\n")))
}

fn sql_value(draft_value: &Cell) -> String {
    draft_value
        .as_deref()
        .map_or_else(|| NULL_LITERAL.to_owned(), quote_literal)
}
