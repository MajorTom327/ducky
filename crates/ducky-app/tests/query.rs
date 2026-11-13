mod support;

use ducky_app::{
    AppState, Command, ConfirmKind, GridCursor, GridMove, Modal, ModalMsg, Msg, NoticeLevel,
    PromptKind, QueryMsg, RunState, SaveFailure,
};
use ducky_core::{ScriptFailure, TransactionMode};
use support::{
    affected_result, file_entry, report, rows_result, send, workspace, workspace_mut,
    workspace_state,
};
use tui_input::InputRequest;

fn notice_text(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.clone().expect("notice");
    (notice.level, notice.text)
}

fn run(state: &mut AppState, sql: &str) -> Vec<Command> {
    send(
        state,
        Msg::Query(QueryMsg::Run {
            sql: sql.to_owned(),
        }),
    )
}

#[test]
fn run_emits_script_with_mode() {
    let mut state = workspace_state();
    send(&mut state, Msg::Query(QueryMsg::ToggleTransactionMode));
    let commands = run(&mut state, "select 1");
    assert_eq!(
        commands,
        vec![Command::RunScript {
            sql: "select 1".to_owned(),
            mode: TransactionMode::AutoCommit,
        }]
    );
    assert_eq!(workspace(&state).query.run, RunState::Running);
}

#[test]
fn run_is_refused_while_running() {
    let mut state = workspace_state();
    run(&mut state, "select 1");
    assert!(run(&mut state, "select 2").is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "A script is already running".to_owned())
    );
}

#[test]
fn blank_run_is_refused() {
    let mut state = workspace_state();
    assert!(run(&mut state, "  \n ").is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "Nothing to run".to_owned())
    );
    assert_eq!(workspace(&state).query.run, RunState::Idle);
}

#[test]
fn cancel_only_while_running() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Query(QueryMsg::Cancel)).is_empty());
    run(&mut state, "select pg_sleep(1)");
    assert_eq!(
        send(&mut state, Msg::Query(QueryMsg::Cancel)),
        vec![Command::CancelScript]
    );
}

#[test]
fn success_notice_selects_last_result() {
    let mut state = workspace_state();
    run(&mut state, "select 1; select 2");
    let commands = send(
        &mut state,
        Msg::ScriptFinished(report(
            TransactionMode::Transaction,
            vec![rows_result("select 1", 1), rows_result("select 2", 2)],
        )),
    );
    assert!(commands.is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "2 statement(s) in 42 ms".to_owned())
    );
    assert_eq!(workspace(&state).query.selected_result_index, 1);
}

#[test]
fn failure_notices_depend_on_mode() {
    let failure = ScriptFailure {
        statement_index: 1,
        sql: "select nope".to_owned(),
        message: "boom".to_owned(),
    };
    let expectations = [
        (
            TransactionMode::Transaction,
            "Statement 2 failed, transaction rolled back",
        ),
        (
            TransactionMode::AutoCommit,
            "Statement 2 failed, earlier statements were committed",
        ),
    ];
    for (mode, expected_text) in expectations {
        let mut state = workspace_state();
        let mut failed_report = report(mode, vec![rows_result("select 1", 1)]);
        failed_report.failure = Some(failure.clone());
        send(&mut state, Msg::ScriptFinished(failed_report));
        assert_eq!(
            notice_text(&state),
            (NoticeLevel::Error, expected_text.to_owned())
        );
    }
}

#[test]
fn affected_result_reloads_catalog() {
    let mut state = workspace_state();
    let commands = send(
        &mut state,
        Msg::ScriptFinished(report(
            TransactionMode::Transaction,
            vec![affected_result("create table t()", 0)],
        )),
    );
    assert_eq!(commands, vec![Command::LoadCatalog]);
}

#[test]
fn results_navigation_clamps_and_resets_cursor() {
    let mut state = workspace_state();
    send(
        &mut state,
        Msg::ScriptFinished(report(
            TransactionMode::Transaction,
            vec![rows_result("select 1", 3), affected_result("delete", 2)],
        )),
    );
    send(&mut state, Msg::Query(QueryMsg::PreviousResult));
    send(&mut state, Msg::Query(QueryMsg::Move(GridMove::Down)));
    assert_eq!(workspace(&state).query.cursor.row_index, 1);
    send(&mut state, Msg::Query(QueryMsg::NextResult));
    send(&mut state, Msg::Query(QueryMsg::NextResult));
    assert_eq!(workspace(&state).query.selected_result_index, 1);
    assert_eq!(workspace(&state).query.cursor, GridCursor::default());
    send(&mut state, Msg::Query(QueryMsg::Move(GridMove::Down)));
    assert_eq!(workspace(&state).query.cursor, GridCursor::default());
    assert!(send(&mut state, Msg::Query(QueryMsg::CopyCell)).is_empty());
}

#[test]
fn copy_and_row_detail_use_selected_rows() {
    let mut state = workspace_state();
    send(
        &mut state,
        Msg::ScriptFinished(report(
            TransactionMode::Transaction,
            vec![rows_result("s", 2)],
        )),
    );
    send(&mut state, Msg::Query(QueryMsg::Move(GridMove::Down)));
    assert_eq!(
        send(&mut state, Msg::Query(QueryMsg::CopyCell)),
        vec![Command::CopyToClipboard("r1c0".to_owned())]
    );
    send(&mut state, Msg::Query(QueryMsg::OpenRowDetail));
    assert!(
        matches!(&state.modal, Some(Modal::RowDetail(detail)) if detail.values[1].as_deref() == Some("r1c1"))
    );
}

#[test]
fn save_unnamed_prompts_then_confirms_overwrite() {
    let mut state = workspace_state();
    let content = "select 1".to_owned();
    assert!(
        send(
            &mut state,
            Msg::Query(QueryMsg::Save {
                content: content.clone()
            })
        )
        .is_empty()
    );
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("expected prompt");
    };
    assert_eq!(
        prompt.kind,
        PromptKind::SaveFileAs {
            content: content.clone()
        }
    );
    assert_eq!(prompt.title, "Save as");
    for typed_char in "report".chars() {
        send(
            &mut state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::SaveFile {
            name: "report".to_owned(),
            content: content.clone(),
            overwrite: false,
        }]
    );
    send(
        &mut state,
        Msg::FileSaved {
            name: "report".to_owned(),
            result: Err(SaveFailure::AlreadyExists),
        },
    );
    let Some(Modal::Confirm(confirm)) = &state.modal else {
        panic!("expected confirm");
    };
    assert_eq!(confirm.kind, ConfirmKind::OverwriteFile);
    assert_eq!(confirm.message, "report already exists. Overwrite?");
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::SaveFile {
            name: "report".to_owned(),
            content,
            overwrite: true,
        }]
    );
    send(
        &mut state,
        Msg::FileSaved {
            name: "report".to_owned(),
            result: Ok(file_entry("report.sql")),
        },
    );
    let query = &workspace(&state).query;
    assert_eq!(query.document.name.as_deref(), Some("report.sql"));
    assert_eq!(query.pending_save, None);
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "Saved report.sql".to_owned())
    );
}

#[test]
fn save_named_overwrites_directly() {
    let mut state = workspace_state();
    workspace_mut(&mut state).query.document.name = Some("a.sql".to_owned());
    let commands = send(
        &mut state,
        Msg::Query(QueryMsg::Save {
            content: "x".to_owned(),
        }),
    );
    assert_eq!(
        commands,
        vec![Command::SaveFile {
            name: "a.sql".to_owned(),
            content: "x".to_owned(),
            overwrite: true,
        }]
    );
}

#[test]
fn file_saved_refreshes_ready_file_list() {
    let mut state = workspace_state();
    workspace_mut(&mut state).files.entries = ducky_app::Loadable::Ready(Vec::new());
    let commands = send(
        &mut state,
        Msg::FileSaved {
            name: "a".to_owned(),
            result: Ok(file_entry("a.sql")),
        },
    );
    assert_eq!(commands, vec![Command::ListSqlFiles]);
}

#[test]
fn save_failure_clears_pending_save() {
    let mut state = workspace_state();
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "x".to_owned(),
        }),
    );
    send(
        &mut state,
        Msg::Modal(ModalMsg::Input(InputRequest::InsertChar('a'))),
    );
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    send(
        &mut state,
        Msg::FileSaved {
            name: "a".to_owned(),
            result: Err(SaveFailure::Other("disk full".to_owned())),
        },
    );
    assert_eq!(workspace(&state).query.pending_save, None);
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "disk full".to_owned())
    );
}
