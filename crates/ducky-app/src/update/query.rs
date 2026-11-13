use ducky_core::{FileEntry, ResultSet, ScriptReport, StatementOutcome, TransactionMode};

use crate::command::Command;
use crate::msg::{QueryMsg, SaveFailure};
use crate::state::{
    AppState, Confirm, ConfirmKind, GridCursor, Modal, NoticeLevel, PendingSave, Prompt,
    PromptKind, RowDetail, RunState, TabId,
};
use crate::update::grid::{cell_text, move_cursor};
use crate::update::{notice, step_index, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: QueryMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if msg == QueryMsg::Cancel {
        return match workspace.running_script_tab_id {
            Some(_) => vec![Command::CancelScript],
            None => Vec::new(),
        };
    }
    let Some((tab_id, query)) = workspace.active_query_mut() else {
        return Vec::new();
    };
    match msg {
        QueryMsg::Run { sql } => run(state, tab_id, sql),
        QueryMsg::Cancel => Vec::new(),
        QueryMsg::ToggleTransactionMode => {
            query.transaction_mode = match query.transaction_mode {
                TransactionMode::Transaction => TransactionMode::AutoCommit,
                TransactionMode::AutoCommit => TransactionMode::Transaction,
            };
            Vec::new()
        }
        QueryMsg::NextResult | QueryMsg::PreviousResult => {
            if let RunState::Finished(report) = &query.run {
                query.selected_result_index = step_index(
                    query.selected_result_index,
                    report.results.len(),
                    msg == QueryMsg::NextResult,
                );
                query.cursor = GridCursor::default();
            }
            Vec::new()
        }
        QueryMsg::Move(grid_move) => {
            if let Some(result_set) = selected_rows(&query.run, query.selected_result_index) {
                move_cursor(&mut query.cursor, result_set, grid_move);
            }
            Vec::new()
        }
        QueryMsg::OpenRowDetail => {
            let cursor = query.cursor;
            if let Some(result_set) = selected_rows(&query.run, query.selected_result_index)
                && let Some(row) = result_set.rows.get(cursor.row_index)
            {
                state.modal = Some(Modal::RowDetail(RowDetail {
                    columns: result_set.columns.clone(),
                    values: row.clone(),
                    selected_field_index: cursor.column_index,
                }));
            }
            Vec::new()
        }
        QueryMsg::CopyCell => {
            let Some(value) = selected_rows(&query.run, query.selected_result_index)
                .and_then(|result_set| cell_text(result_set, query.cursor))
            else {
                return Vec::new();
            };
            state.notice = notice(NoticeLevel::Info, "Copied");
            vec![Command::CopyToClipboard(value)]
        }
        QueryMsg::Save { content } => match query.document.name.clone() {
            Some(name) => {
                workspace.pending_save = Some(PendingSave {
                    tab_id,
                    name: name.clone(),
                    content: content.clone(),
                });
                vec![Command::SaveFile {
                    name,
                    content,
                    overwrite: true,
                }]
            }
            None => open_save_as(state, None, content),
        },
        QueryMsg::SaveAs { content } => {
            let current_name = query.document.name.clone();
            open_save_as(state, current_name, content)
        }
    }
}

fn selected_rows(run: &RunState, selected_result_index: usize) -> Option<&ResultSet> {
    let RunState::Finished(report) = run else {
        return None;
    };
    match &report.results.get(selected_result_index)?.outcome {
        StatementOutcome::Rows(result_set) => Some(result_set),
        StatementOutcome::Affected(_) => None,
    }
}

fn open_save_as(
    state: &mut AppState,
    current_name: Option<String>,
    content: String,
) -> Vec<Command> {
    state.modal = Some(Modal::Prompt(Prompt::new(
        PromptKind::SaveFileAs { content },
        "Save as",
        current_name.unwrap_or_default(),
    )));
    Vec::new()
}

pub(crate) fn run(state: &mut AppState, tab_id: TabId, sql: String) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if workspace.running_script_tab_id.is_some() {
        state.notice = notice(NoticeLevel::Error, "A script is already running");
        return Vec::new();
    }
    if sql.trim().is_empty() {
        state.notice = notice(NoticeLevel::Error, "Nothing to run");
        return Vec::new();
    }
    let profile_label = workspace.profile.label.clone();
    let Some(query) = workspace.query_mut(tab_id) else {
        return Vec::new();
    };
    query.run = RunState::Running;
    query.ran_on_profile_label = Some(profile_label);
    let mode = query.transaction_mode;
    workspace.running_script_tab_id = Some(tab_id);
    vec![Command::RunScript { tab_id, sql, mode }]
}

pub(crate) fn script_finished(
    state: &mut AppState,
    tab_id: TabId,
    report: ScriptReport,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    workspace.running_script_tab_id = None;
    let report_notice = match (&report.failure, report.mode) {
        (None, _) => notice(
            NoticeLevel::Success,
            format!(
                "{} statement(s) in {} ms",
                report.results.len(),
                report.elapsed.as_millis()
            ),
        ),
        (Some(failure), TransactionMode::Transaction) => notice(
            NoticeLevel::Error,
            format!(
                "Statement {} failed, transaction rolled back",
                failure.statement_index + 1
            ),
        ),
        (Some(failure), TransactionMode::AutoCommit) => notice(
            NoticeLevel::Error,
            format!(
                "Statement {} failed, earlier statements were committed",
                failure.statement_index + 1
            ),
        ),
    };
    let has_affected_result = report
        .results
        .iter()
        .any(|statement_result| matches!(statement_result.outcome, StatementOutcome::Affected(_)));
    if let Some(query) = workspace.query_mut(tab_id) {
        query.selected_result_index = report.results.len().saturating_sub(1);
        query.cursor = GridCursor::default();
        query.run = RunState::Finished(report);
    }
    state.notice = report_notice;
    if has_affected_result {
        vec![Command::LoadCatalog]
    } else {
        Vec::new()
    }
}

pub(crate) fn file_saved(
    state: &mut AppState,
    name: String,
    result: Result<FileEntry, SaveFailure>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    match result {
        Ok(entry) => {
            state.notice = notice(NoticeLevel::Success, format!("Saved {}", entry.name));
            if let Some(pending_save) = workspace.pending_save.take()
                && let Some(query) = workspace.query_mut(pending_save.tab_id)
            {
                query.document.name = Some(entry.name);
                query.document.text = pending_save.content;
            }
            if workspace.files.entries.ready().is_some() {
                vec![Command::ListSqlFiles]
            } else {
                Vec::new()
            }
        }
        Err(SaveFailure::AlreadyExists) => {
            if workspace.pending_save.is_some() {
                state.modal = Some(Modal::Confirm(Confirm {
                    kind: ConfirmKind::OverwriteFile,
                    message: format!("{name} already exists. Overwrite?"),
                }));
            } else {
                state.notice = notice(NoticeLevel::Error, format!("{name} already exists"));
            }
            Vec::new()
        }
        Err(SaveFailure::Other(message)) => {
            workspace.pending_save = None;
            state.notice = notice(NoticeLevel::Error, message);
            Vec::new()
        }
    }
}
