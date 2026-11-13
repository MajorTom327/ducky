mod support;

use ducky_app::{
    AppState, Command, ConfirmKind, DumpMsg, JobKind, JobState, Loadable, MAX_JOB_LOG_LINES, Modal,
    ModalMsg, Msg, NoticeLevel, RestoreMsg, SidebarAction, SidebarItem, SidebarMsg,
};
use ducky_core::{PgTools, RestoreOptions};
use support::{
    connected_state, file_entry, select_sidebar_item, send, tools_available, tools_missing,
    workspace, workspace_state,
};

fn state_on(action: SidebarAction, tools: PgTools) -> AppState {
    let mut state = connected_state(tools);
    send(&mut state, Msg::CatalogLoaded(Ok(support::catalog())));
    select_sidebar_item(&mut state, |item| *item == SidebarItem::Action(action));
    send(&mut state, Msg::Sidebar(SidebarMsg::Activate));
    state
}

fn notice_text(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.clone().expect("notice");
    (notice.level, notice.text)
}

#[test]
fn dump_with_missing_tool_emits_nothing() {
    let mut state = state_on(SidebarAction::Dump, tools_missing());
    assert!(send(&mut state, Msg::Dump(DumpMsg::Start)).is_empty());
    assert_eq!(
        notice_text(&state),
        (
            NoticeLevel::Error,
            "`pg_dump` was not found in PATH".to_owned()
        )
    );
    assert_eq!(workspace(&state).dump.job, JobState::Idle);
}

#[test]
fn dump_start_runs_once() {
    let mut state = state_on(SidebarAction::Dump, tools_available());
    send(
        &mut state,
        Msg::JobLogged {
            job: JobKind::Dump,
            line: "old".to_owned(),
        },
    );
    let commands = send(&mut state, Msg::Dump(DumpMsg::Start));
    assert_eq!(
        commands,
        vec![Command::StartDump {
            custom_file_name: None
        }]
    );
    assert_eq!(workspace(&state).dump.job, JobState::Running);
    assert!(workspace(&state).dump.log_lines.is_empty());
    assert!(send(&mut state, Msg::Dump(DumpMsg::Start)).is_empty());
}

#[test]
fn dump_finished_lists_dumps() {
    let mut state = state_on(SidebarAction::Dump, tools_available());
    send(&mut state, Msg::Dump(DumpMsg::Start));
    let commands = send(
        &mut state,
        Msg::JobFinished {
            job: JobKind::Dump,
            result: Ok("Dump written to x".to_owned()),
        },
    );
    assert_eq!(commands, vec![Command::ListDumps]);
    assert_eq!(
        workspace(&state).dump.job,
        JobState::Succeeded("Dump written to x".to_owned())
    );
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "Dump written to x".to_owned())
    );
}

#[test]
fn job_failure_is_reported() {
    let mut state = state_on(SidebarAction::Dump, tools_available());
    send(&mut state, Msg::Dump(DumpMsg::Start));
    let commands = send(
        &mut state,
        Msg::JobFinished {
            job: JobKind::Dump,
            result: Err("exit 1".to_owned()),
        },
    );
    assert!(commands.is_empty());
    assert_eq!(
        workspace(&state).dump.job,
        JobState::Failed("exit 1".to_owned())
    );
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "exit 1".to_owned())
    );
}

#[test]
fn job_log_is_capped() {
    let mut state = workspace_state();
    for line_index in 0..MAX_JOB_LOG_LINES + 5 {
        send(
            &mut state,
            Msg::JobLogged {
                job: JobKind::Restore,
                line: format!("line {line_index}"),
            },
        );
    }
    let log_lines = &workspace(&state).restore.log_lines;
    assert_eq!(log_lines.len(), MAX_JOB_LOG_LINES);
    assert_eq!(log_lines[0], "line 5");
}

#[test]
fn restore_with_missing_tool_emits_nothing() {
    let mut state = state_on(SidebarAction::Restore, tools_missing());
    send(&mut state, Msg::DumpsListed(Ok(vec![file_entry("a.dump")])));
    assert!(send(&mut state, Msg::Restore(RestoreMsg::Start)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(notice_text(&state).0, NoticeLevel::Error);
}

#[test]
fn restore_without_entry_is_ignored() {
    let mut state = state_on(SidebarAction::Restore, tools_available());
    send(&mut state, Msg::DumpsListed(Ok(Vec::new())));
    assert!(send(&mut state, Msg::Restore(RestoreMsg::Start)).is_empty());
    assert!(state.modal.is_none());
}

#[test]
fn restore_confirm_flow() {
    let mut state = state_on(SidebarAction::Restore, tools_available());
    send(
        &mut state,
        Msg::DumpsListed(Ok(vec![file_entry("new.dump"), file_entry("old.dump")])),
    );
    send(&mut state, Msg::Restore(RestoreMsg::Next));
    send(&mut state, Msg::Restore(RestoreMsg::Next));
    send(&mut state, Msg::Restore(RestoreMsg::ToggleClean));
    send(&mut state, Msg::Restore(RestoreMsg::ToggleSkipOwnership));
    send(&mut state, Msg::Restore(RestoreMsg::ToggleSkipOwnership));
    assert!(send(&mut state, Msg::Restore(RestoreMsg::Start)).is_empty());
    let Some(Modal::Confirm(confirm)) = &state.modal else {
        panic!("expected confirm");
    };
    assert_eq!(
        confirm.kind,
        ConfirmKind::Restore {
            file_name: "old.dump".to_owned()
        }
    );
    assert_eq!(
        confirm.message,
        "Restore old.dump into ducky? Existing data can be overwritten."
    );
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::StartRestore {
            file_name: "old.dump".to_owned(),
            options: RestoreOptions {
                clean_before_restore: true,
                skip_ownership: false,
            },
        }]
    );
    assert_eq!(workspace(&state).restore.job, JobState::Running);
    assert!(send(&mut state, Msg::Restore(RestoreMsg::Start)).is_empty());
    let commands = send(
        &mut state,
        Msg::JobFinished {
            job: JobKind::Restore,
            result: Ok("Restored old.dump into ducky".to_owned()),
        },
    );
    assert_eq!(commands, vec![Command::LoadCatalog]);
}

#[test]
fn restore_reload_lists_dumps_and_listing_clamps() {
    let mut state = state_on(SidebarAction::Restore, tools_available());
    send(
        &mut state,
        Msg::DumpsListed(Ok(vec![file_entry("a.dump"), file_entry("b.dump")])),
    );
    send(&mut state, Msg::Restore(RestoreMsg::Next));
    assert_eq!(
        send(&mut state, Msg::Restore(RestoreMsg::Reload)),
        vec![Command::ListDumps]
    );
    send(&mut state, Msg::DumpsListed(Ok(vec![file_entry("a.dump")])));
    assert_eq!(workspace(&state).restore.selected_entry_index, 0);
    send(&mut state, Msg::DumpsListed(Err("denied".to_owned())));
    assert_eq!(
        workspace(&state).restore.entries,
        Loadable::Failed("denied".to_owned())
    );
}
