use ducky_core::{FileEntry, ToolStatus};

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::{DumpMsg, RestoreMsg};
use crate::state::{
    AppState, Confirm, ConfirmKind, JobKind, JobState, MAX_JOB_LOG_LINES, Modal, NoticeLevel,
    Prompt, PromptKind, TabContent,
};
use crate::update::{clamp_index, notice, step_index, workspace_mut};

pub(crate) fn update_dump(state: &mut AppState, msg: DumpMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if !workspace.is_active_content(|content| matches!(content, TabContent::Dump)) {
        return Vec::new();
    }
    let dump = &mut workspace.dump;
    match msg {
        DumpMsg::EditFileName => {
            state.modal = Some(Modal::Prompt(Prompt::new(
                PromptKind::DumpFileName,
                "Dump file name",
                dump.custom_file_name.clone().unwrap_or_default(),
            )));
            Vec::new()
        }
        DumpMsg::Start => {
            if let ToolStatus::Missing(reason) = &workspace.tools.pg_dump {
                state.notice = notice(NoticeLevel::Error, reason.clone());
                return Vec::new();
            }
            if dump.job == JobState::Running {
                return Vec::new();
            }
            dump.job = JobState::Running;
            dump.log_lines.clear();
            vec![Command::StartDump {
                custom_file_name: dump.custom_file_name.clone(),
            }]
        }
    }
}

pub(crate) fn update_restore(state: &mut AppState, msg: RestoreMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if !workspace.is_active_content(|content| matches!(content, TabContent::Restore)) {
        return Vec::new();
    }
    let restore = &mut workspace.restore;
    match msg {
        RestoreMsg::Next | RestoreMsg::Previous => {
            if let Some(entry_count) = restore.entries.ready().map(Vec::len) {
                restore.selected_entry_index = step_index(
                    restore.selected_entry_index,
                    entry_count,
                    msg == RestoreMsg::Next,
                );
            }
            Vec::new()
        }
        RestoreMsg::Select(entry_index) => {
            if let Some(entry_count) = restore.entries.ready().map(Vec::len) {
                restore.selected_entry_index = clamp_index(entry_index, entry_count);
            }
            Vec::new()
        }
        RestoreMsg::ToggleClean => {
            restore.options.clean_before_restore = !restore.options.clean_before_restore;
            Vec::new()
        }
        RestoreMsg::ToggleSkipOwnership => {
            restore.options.skip_ownership = !restore.options.skip_ownership;
            Vec::new()
        }
        RestoreMsg::Reload => {
            restore.entries = Loadable::Loading;
            vec![Command::ListDumps]
        }
        RestoreMsg::Start => {
            if let ToolStatus::Missing(reason) = &workspace.tools.pg_restore {
                state.notice = notice(NoticeLevel::Error, reason.clone());
                return Vec::new();
            }
            if restore.job == JobState::Running {
                return Vec::new();
            }
            let Some(entry) = restore.selected_entry() else {
                return Vec::new();
            };
            state.modal = Some(Modal::Confirm(Confirm {
                kind: ConfirmKind::Restore {
                    file_name: entry.name.clone(),
                },
                message: format!(
                    "Restore {} into {}? Existing data can be overwritten.",
                    entry.name, workspace.session.database
                ),
            }));
            Vec::new()
        }
    }
}

pub(crate) fn dumps_listed(
    state: &mut AppState,
    result: Result<Vec<FileEntry>, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let restore = &mut workspace.restore;
    restore.entries = Loadable::from_result(result);
    if let Some(entry_count) = restore.entries.ready().map(Vec::len) {
        restore.selected_entry_index = clamp_index(restore.selected_entry_index, entry_count);
    }
    Vec::new()
}

pub(crate) fn logged(state: &mut AppState, job: JobKind, line: String) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let log_lines = match job {
        JobKind::Dump => &mut workspace.dump.log_lines,
        JobKind::Restore => &mut workspace.restore.log_lines,
    };
    log_lines.push(line);
    if log_lines.len() > MAX_JOB_LOG_LINES {
        let overflow_count = log_lines.len() - MAX_JOB_LOG_LINES;
        log_lines.drain(..overflow_count);
    }
    Vec::new()
}

pub(crate) fn finished(
    state: &mut AppState,
    job: JobKind,
    result: Result<String, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let job_state = match job {
        JobKind::Dump => &mut workspace.dump.job,
        JobKind::Restore => &mut workspace.restore.job,
    };
    match result {
        Ok(summary) => {
            state.notice = notice(NoticeLevel::Success, summary.clone());
            *job_state = JobState::Succeeded(summary);
            match job {
                JobKind::Dump => vec![Command::ListDumps],
                JobKind::Restore => vec![Command::LoadCatalog],
            }
        }
        Err(message) => {
            state.notice = notice(NoticeLevel::Error, message.clone());
            *job_state = JobState::Failed(message);
            Vec::new()
        }
    }
}
