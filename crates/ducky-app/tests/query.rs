mod support;

use ducky_app::{
    AppState, Command, ConfirmKind, GridCursor, GridMove, Modal, ModalMsg, Msg, NoticeLevel,
    PendingSave, PromptKind, QueryMsg, RunState, SaveFailure, TabId, TabsMsg,
};
use ducky_core::{ScriptFailure, ScriptReport, TransactionMode};
use support::{
    active_query, active_tab_id, affected_result, file_entry, new_query, query_tab, query_tab_mut,
    report, rows_result, send, workspace, workspace_mut, workspace_state,
};
use tui_input::InputRequest;

fn notice_text(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.clone().expect("notice");
    (notice.level, notice.text)
}

fn query_state() -> AppState {
    let mut state = workspace_state();
    new_query(&mut state);
    state
}

fn finish(state: &mut AppState, report: ScriptReport) -> Vec<Command> {
    let tab_id = active_tab_id(state);
    finish_tab(state, tab_id, report)
}

fn finish_tab(state: &mut AppState, tab_id: TabId, report: ScriptReport) -> Vec<Command> {
    send(state, Msg::ScriptFinished { tab_id, report })
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
    let mut state = query_state();
    send(&mut state, Msg::Query(QueryMsg::ToggleTransactionMode));
    let commands = run(&mut state, "select 1");
    assert_eq!(
        commands,
        vec![Command::RunScript {
            tab_id: 1,
            sql: "select 1".to_owned(),
            mode: TransactionMode::AutoCommit,
        }]
    );
    assert_eq!(active_query(&state).run, RunState::Running);
    assert_eq!(workspace(&state).running_script_tab_id, Some(1));
}

#[test]
fn run_is_refused_while_running() {
    let mut state = query_state();
    run(&mut state, "select 1");
    assert!(run(&mut state, "select 2").is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "A script is already running".to_owned())
    );
}

#[test]
fn blank_run_is_refused() {
    let mut state = query_state();
    assert!(run(&mut state, "  \n ").is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "Nothing to run".to_owned())
    );
    assert_eq!(active_query(&state).run, RunState::Idle);
}

#[test]
fn cancel_only_while_running() {
    let mut state = query_state();
    assert!(send(&mut state, Msg::Query(QueryMsg::Cancel)).is_empty());
    run(&mut state, "select pg_sleep(1)");
    assert_eq!(
        send(&mut state, Msg::Query(QueryMsg::Cancel)),
        vec![Command::CancelScript]
    );
}

#[test]
fn success_notice_selects_last_result() {
    let mut state = query_state();
    run(&mut state, "select 1; select 2");
    let commands = finish(
        &mut state,
        report(
            TransactionMode::Transaction,
            vec![rows_result("select 1", 1), rows_result("select 2", 2)],
        ),
    );
    assert!(commands.is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "2 statement(s) in 42 ms".to_owned())
    );
    assert_eq!(active_query(&state).selected_result_index, 1);
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
        let mut state = query_state();
        let mut failed_report = report(mode, vec![rows_result("select 1", 1)]);
        failed_report.failure = Some(failure.clone());
        finish(&mut state, failed_report);
        assert_eq!(
            notice_text(&state),
            (NoticeLevel::Error, expected_text.to_owned())
        );
    }
}

#[test]
fn affected_result_reloads_catalog() {
    let mut state = query_state();
    let commands = finish(
        &mut state,
        report(
            TransactionMode::Transaction,
            vec![affected_result("create table t()", 0)],
        ),
    );
    assert_eq!(commands, vec![Command::LoadCatalog]);
}

#[test]
fn results_navigation_clamps_and_resets_cursor() {
    let mut state = query_state();
    finish(
        &mut state,
        report(
            TransactionMode::Transaction,
            vec![rows_result("select 1", 3), affected_result("delete", 2)],
        ),
    );
    send(&mut state, Msg::Query(QueryMsg::PreviousResult));
    send(&mut state, Msg::Query(QueryMsg::Move(GridMove::Down)));
    assert_eq!(active_query(&state).cursor.row_index, 1);
    send(&mut state, Msg::Query(QueryMsg::NextResult));
    send(&mut state, Msg::Query(QueryMsg::NextResult));
    assert_eq!(active_query(&state).selected_result_index, 1);
    assert_eq!(active_query(&state).cursor, GridCursor::default());
    send(&mut state, Msg::Query(QueryMsg::Move(GridMove::Down)));
    assert_eq!(active_query(&state).cursor, GridCursor::default());
    assert!(send(&mut state, Msg::Query(QueryMsg::CopyCell)).is_empty());
}

#[test]
fn copy_and_row_detail_use_selected_rows() {
    let mut state = query_state();
    finish(
        &mut state,
        report(TransactionMode::Transaction, vec![rows_result("s", 2)]),
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
    let mut state = query_state();
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
    let query = active_query(&state);
    assert_eq!(query.document.name.as_deref(), Some("report.sql"));
    assert_eq!(query.document.text, "select 1");
    assert_eq!(query.document.revision, 0);
    assert_eq!(workspace(&state).pending_save, None);
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "Saved report.sql".to_owned())
    );
}

#[test]
fn save_named_overwrites_directly() {
    let mut state = query_state();
    let tab_id = active_tab_id(&state);
    query_tab_mut(&mut state, tab_id).document.name = Some("a.sql".to_owned());
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
    assert_eq!(
        workspace(&state).pending_save,
        Some(PendingSave {
            tab_id,
            name: "a.sql".to_owned(),
            content: "x".to_owned(),
        })
    );
}

#[test]
fn file_saved_refreshes_ready_file_list() {
    let mut state = query_state();
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
    let mut state = query_state();
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
    assert_eq!(workspace(&state).pending_save, None);
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "disk full".to_owned())
    );
}

#[test]
fn cancel_works_from_another_tab() {
    let mut state = query_state();
    run(&mut state, "select pg_sleep(1)");
    send(&mut state, Msg::Tabs(TabsMsg::OpenDump));
    assert_eq!(
        send(&mut state, Msg::Query(QueryMsg::Cancel)),
        vec![Command::CancelScript]
    );
}

#[test]
fn query_messages_are_ignored_without_query_tab() {
    let mut state = workspace_state();
    assert!(run(&mut state, "select 1").is_empty());
    assert!(
        send(
            &mut state,
            Msg::Query(QueryMsg::SaveAs {
                content: "x".to_owned()
            })
        )
        .is_empty()
    );
    assert!(state.modal.is_none());
}

#[test]
fn script_is_routed_to_the_tab_that_ran_it() {
    let mut state = query_state();
    let first_tab_id = active_tab_id(&state);
    run(&mut state, "select 1");
    let second_tab_id = new_query(&mut state);
    finish_tab(
        &mut state,
        first_tab_id,
        report(
            TransactionMode::Transaction,
            vec![rows_result("select 1", 1)],
        ),
    );
    assert!(matches!(
        query_tab(&state, first_tab_id).run,
        RunState::Finished(_)
    ));
    assert_eq!(query_tab(&state, second_tab_id).run, RunState::Idle);
    assert_eq!(workspace(&state).running_script_tab_id, None);
    assert_eq!(active_tab_id(&state), second_tab_id);
}

#[test]
fn run_is_refused_while_another_tab_runs() {
    let mut state = query_state();
    run(&mut state, "select 1");
    let second_tab_id = new_query(&mut state);
    assert!(run(&mut state, "select 2").is_empty());
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Error, "A script is already running".to_owned())
    );
    assert_eq!(query_tab(&state, second_tab_id).run, RunState::Idle);
}

#[test]
fn script_finished_for_a_closed_tab_still_notifies_and_reloads_catalog() {
    let mut state = query_state();
    let commands = finish_tab(
        &mut state,
        99,
        report(
            TransactionMode::AutoCommit,
            vec![affected_result("delete", 1)],
        ),
    );
    assert_eq!(commands, vec![Command::LoadCatalog]);
    assert_eq!(
        notice_text(&state),
        (NoticeLevel::Success, "1 statement(s) in 42 ms".to_owned())
    );
    assert_eq!(active_query(&state).run, RunState::Idle);
}

#[test]
fn save_from_one_tab_names_that_tab_after_switching() {
    let mut state = query_state();
    let first_tab_id = active_tab_id(&state);
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "select 1".to_owned(),
        }),
    );
    send(
        &mut state,
        Msg::Modal(ModalMsg::Input(InputRequest::InsertChar('a'))),
    );
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    let second_tab_id = new_query(&mut state);
    send(
        &mut state,
        Msg::FileSaved {
            name: "a".to_owned(),
            result: Ok(file_entry("a.sql")),
        },
    );
    let first_document = &query_tab(&state, first_tab_id).document;
    assert_eq!(first_document.name.as_deref(), Some("a.sql"));
    assert_eq!(first_document.text, "select 1");
    assert_eq!(query_tab(&state, second_tab_id).document.name, None);
}

#[test]
fn save_as_prompt_is_prefilled_with_document_name() {
    let mut state = query_state();
    let tab_id = active_tab_id(&state);
    query_tab_mut(&mut state, tab_id).document.name = Some("a.sql".to_owned());
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "x".to_owned(),
        }),
    );
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("expected prompt");
    };
    assert_eq!(prompt.input.value(), "a.sql");
}
