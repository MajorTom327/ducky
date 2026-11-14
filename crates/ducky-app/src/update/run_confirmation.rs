use ducky_core::ProfileKind;
use ducky_sql::split_statements;

use crate::command::Command;
use crate::state::{
    AppState, Confirm, ConfirmKind, Modal, NoticeLevel, TabContent, TabId, Workspace,
};
use crate::update::query::{is_run_refused, run};
use crate::update::{notice, workspace_mut};

pub(crate) fn request(state: &mut AppState, tab_id: TabId, sql: String) -> Vec<Command> {
    if is_run_refused(state, &sql) {
        return Vec::new();
    }
    let Some(workspace) = state.workspace() else {
        return Vec::new();
    };
    if !needs_confirmation(workspace, tab_id) {
        return run(state, tab_id, sql);
    }
    let message = confirmation_message(workspace, &sql);
    state.modal = Some(Modal::Confirm(Confirm {
        kind: ConfirmKind::RunScript { tab_id, sql },
        message,
    }));
    Vec::new()
}

pub(crate) fn confirmed(state: &mut AppState, tab_id: TabId, sql: String) -> Vec<Command> {
    if is_run_refused(state, &sql) {
        return Vec::new();
    }
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let profile_index = workspace.profile_index;
    let is_staging = workspace.profile.kind == ProfileKind::Staging;
    let Some(query) = workspace.query_mut(tab_id) else {
        return Vec::new();
    };
    if is_staging {
        query.confirmed_profile_index = Some(profile_index);
    }
    run(state, tab_id, sql)
}

pub(crate) fn cancelled(state: &mut AppState) {
    state.notice = notice(NoticeLevel::Info, "Run cancelled");
}

pub(crate) fn forget_confirmations(workspace: &mut Workspace) {
    for tab in &mut workspace.tabs {
        if let TabContent::Query(query) = &mut tab.content {
            query.confirmed_profile_index = None;
        }
    }
}

fn needs_confirmation(workspace: &Workspace, tab_id: TabId) -> bool {
    match workspace.profile.kind {
        ProfileKind::Local => false,
        ProfileKind::Prod => true,
        ProfileKind::Staging => workspace
            .query(tab_id)
            .is_none_or(|query| query.confirmed_profile_index != Some(workspace.profile_index)),
    }
}

fn confirmation_message(workspace: &Workspace, sql: &str) -> String {
    format!(
        "Run {} statement(s) on {}?",
        split_statements(sql).len(),
        profile_description(workspace)
    )
}

pub(crate) fn profile_description(workspace: &Workspace) -> String {
    let kind_word = match workspace.profile.kind {
        ProfileKind::Local => "LOCAL",
        ProfileKind::Staging => "STAGING",
        ProfileKind::Prod => "PROD",
    };
    format!(
        "{kind_word} · {} ({})",
        workspace.profile.display_name(),
        workspace.profile.target()
    )
}
