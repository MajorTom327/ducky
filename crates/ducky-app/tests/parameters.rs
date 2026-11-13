mod support;

use ducky_app::{
    AppState, Command, FilesMsg, Modal, ModalMsg, Msg, NoticeLevel, ParameterForm, QueryMsg,
    RunState, TabsMsg,
};
use ducky_core::TransactionMode;
use ducky_sql::ParameterValue;
use support::{
    active_query, active_tab_id, file_entry, new_query, query_tab_mut, report, send, workspace,
    workspace_state,
};
use tui_input::InputRequest;

const PARAMETER_SQL: &str = "select * from orders where id = :id and status = :status limit $1";

fn query_state() -> AppState {
    let mut state = workspace_state();
    new_query(&mut state);
    state
}

fn run(state: &mut AppState, sql: &str) -> Vec<Command> {
    send(
        state,
        Msg::Query(QueryMsg::Run {
            sql: sql.to_owned(),
        }),
    )
}

fn form(state: &AppState) -> &ParameterForm {
    match &state.modal {
        Some(Modal::Parameters(form)) => form,
        other => panic!("expected the parameter form, got {other:?}"),
    }
}

fn type_text(state: &mut AppState, text: &str) {
    for typed_char in text.chars() {
        send(
            state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
}

fn field_summary(state: &AppState) -> Vec<(String, String, bool)> {
    form(state)
        .fields
        .iter()
        .map(|field| {
            (
                field.name.clone(),
                field.input.value().to_owned(),
                field.is_raw,
            )
        })
        .collect()
}

#[test]
fn run_without_parameters_emits_run_script_directly() {
    let mut state = query_state();
    let commands = run(&mut state, "select ':id', 1::int");
    assert_eq!(
        commands,
        vec![Command::RunScript {
            tab_id: 1,
            sql: "select ':id', 1::int".to_owned(),
            mode: TransactionMode::Transaction,
        }]
    );
    assert!(state.modal.is_none());
}

#[test]
fn run_with_parameters_opens_the_form_without_command() {
    let mut state = query_state();
    assert!(run(&mut state, PARAMETER_SQL).is_empty());
    let form = form(&state);
    assert_eq!(form.tab_id, 1);
    assert_eq!(form.sql, PARAMETER_SQL);
    assert_eq!(form.selected_field_index, 0);
    assert_eq!(
        field_summary(&state),
        vec![
            (":id".to_owned(), String::new(), false),
            (":status".to_owned(), String::new(), false),
            ("$1".to_owned(), String::new(), false),
        ]
    );
    assert_eq!(active_query(&state).run, RunState::Idle);
}

#[test]
fn form_is_prefilled_from_the_last_values_of_the_tab() {
    let mut state = query_state();
    let tab_id = active_tab_id(&state);
    query_tab_mut(&mut state, tab_id).parameter_values = vec![
        (":id".to_owned(), ParameterValue::Text("42".to_owned())),
        ("$1".to_owned(), ParameterValue::Raw("now()".to_owned())),
    ];
    run(&mut state, PARAMETER_SQL);
    assert_eq!(
        field_summary(&state),
        vec![
            (":id".to_owned(), "42".to_owned(), false),
            (":status".to_owned(), String::new(), false),
            ("$1".to_owned(), "now()".to_owned(), true),
        ]
    );
}

#[test]
fn submit_stores_values_and_runs_the_bound_sql() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    type_text(&mut state, "42");
    send(&mut state, Msg::Modal(ModalMsg::Next));
    type_text(&mut state, "it's");
    send(&mut state, Msg::Modal(ModalMsg::Next));
    type_text(&mut state, "10");
    send(&mut state, Msg::Modal(ModalMsg::ToggleRaw));
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::RunScript {
            tab_id: 1,
            sql: "select * from orders where id = '42' and status = 'it''s' limit 10".to_owned(),
            mode: TransactionMode::Transaction,
        }]
    );
    assert!(state.modal.is_none());
    let query = active_query(&state);
    assert_eq!(query.run, RunState::Running);
    assert_eq!(
        query.parameter_values,
        vec![
            (":id".to_owned(), ParameterValue::Text("42".to_owned())),
            (
                ":status".to_owned(),
                ParameterValue::Text("it's".to_owned())
            ),
            ("$1".to_owned(), ParameterValue::Raw("10".to_owned())),
        ]
    );
}

#[test]
fn next_and_previous_are_clamped() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    send(&mut state, Msg::Modal(ModalMsg::Previous));
    assert_eq!(form(&state).selected_field_index, 0);
    for _ in 0..5 {
        send(&mut state, Msg::Modal(ModalMsg::Next));
    }
    assert_eq!(form(&state).selected_field_index, 2);
}

#[test]
fn toggle_raw_flips_the_selected_field_only() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    send(&mut state, Msg::Modal(ModalMsg::Next));
    send(&mut state, Msg::Modal(ModalMsg::ToggleRaw));
    let raw_flags: Vec<bool> = form(&state)
        .fields
        .iter()
        .map(|field| field.is_raw)
        .collect();
    assert_eq!(raw_flags, vec![false, true, false]);
    send(&mut state, Msg::Modal(ModalMsg::ToggleRaw));
    assert!(!form(&state).fields[1].is_raw);
}

#[test]
fn cancel_closes_the_form_without_command() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    type_text(&mut state, "42");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Cancel)).is_empty());
    assert!(state.modal.is_none());
    let query = active_query(&state);
    assert_eq!(query.run, RunState::Idle);
    assert!(query.parameter_values.is_empty());
}

#[test]
fn running_guard_applies_before_opening_the_form() {
    let mut state = query_state();
    run(&mut state, "select 1");
    send(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    assert!(run(&mut state, PARAMETER_SQL).is_empty());
    assert!(state.modal.is_none());
    let notice = state.notice.clone().expect("notice");
    assert_eq!(notice.level, NoticeLevel::Error);
    assert_eq!(notice.text, "A script is already running");
}

#[test]
fn submit_rechecks_the_running_guard() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    let pending_form = state.modal.take();
    send(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    run(&mut state, "select 1");
    state.modal = pending_form;
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert_eq!(
        state.notice.clone().expect("notice").text,
        "A script is already running"
    );
}

#[test]
fn executing_a_file_with_parameters_opens_the_form() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::OpenFiles));
    send(
        &mut state,
        Msg::SqlFilesListed(Ok(vec![file_entry("a.sql")])),
    );
    send(&mut state, Msg::Files(FilesMsg::Execute));
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    let commands = send(
        &mut state,
        Msg::FileOpened {
            name: "a.sql".to_owned(),
            run_after_open: true,
            result: Ok("select :id".to_owned()),
        },
    );
    assert!(commands.is_empty());
    let form = form(&state);
    assert_eq!(form.tab_id, active_tab_id(&state));
    assert_eq!(form.sql, "select :id");
    assert_eq!(form.fields.len(), 1);
}

#[test]
fn selected_field_follows_navigation() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    let selected_name =
        |state: &AppState| form(state).selected_field().map(|field| field.name.clone());
    assert_eq!(selected_name(&state).as_deref(), Some(":id"));
    send(&mut state, Msg::Modal(ModalMsg::Next));
    assert_eq!(selected_name(&state).as_deref(), Some(":status"));
}

#[test]
fn submit_after_closing_the_tab_runs_nothing() {
    let mut state = query_state();
    run(&mut state, PARAMETER_SQL);
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    type_text(&mut state, "42");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).running_script_tab_id, None);
}

#[test]
fn resubmitting_replaces_only_the_submitted_values() {
    let mut state = query_state();
    let tab_id = active_tab_id(&state);
    run(&mut state, "select :id, :status");
    type_text(&mut state, "1");
    send(&mut state, Msg::Modal(ModalMsg::Next));
    type_text(&mut state, "open");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    send(
        &mut state,
        Msg::ScriptFinished {
            tab_id,
            report: report(TransactionMode::Transaction, Vec::new()),
        },
    );
    run(&mut state, "select :id");
    type_text(&mut state, "2");
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::RunScript {
            tab_id,
            sql: "select '12'".to_owned(),
            mode: TransactionMode::Transaction,
        }]
    );
    assert_eq!(
        active_query(&state).parameter_values,
        vec![
            (
                ":status".to_owned(),
                ParameterValue::Text("open".to_owned())
            ),
            (":id".to_owned(), ParameterValue::Text("12".to_owned())),
        ]
    );
}
