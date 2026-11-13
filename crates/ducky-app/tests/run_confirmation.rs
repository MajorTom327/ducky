mod support;

use ducky_app::{
    AppState, Command, Confirm, ConfirmKind, FilesMsg, Modal, ModalMsg, Msg, NoticeLevel,
    ProfilesMsg, QueryMsg, Screen, TabId, TabsMsg,
};
use ducky_core::{Profile, ProfileKind, TransactionMode};
use support::{
    active_tab_id, catalog, connection, file_entry, new_query, query_tab, report, send, storage,
    tools_available, workspace, workspace_mut,
};
use tui_input::InputRequest;

const LOCAL_INDEX: usize = 0;
const STAGING_INDEX: usize = 1;
const PROD_INDEX: usize = 2;

fn profile(label: &str, url: &str, kind: ProfileKind) -> Profile {
    Profile {
        label: label.to_owned(),
        url: url.to_owned(),
        kind,
        project: None,
        tags: Vec::new(),
    }
}

fn kind_profiles() -> Vec<Profile> {
    vec![
        profile("Local", "postgres://localhost/a", ProfileKind::Local),
        profile(
            "Staging",
            "postgres://user:secret@stage.example.com:5432/app",
            ProfileKind::Staging,
        ),
        profile(
            "Prod",
            "postgres://user:secret@prod.example.com:5432/app?sslmode=require",
            ProfileKind::Prod,
        ),
    ]
}

fn state_on(profile_index: usize) -> AppState {
    let mut state = AppState::new(kind_profiles(), storage());
    if let Screen::Profiles(picker) = &mut state.screen {
        picker.selected_profile_index = profile_index;
    }
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    send(
        &mut state,
        Msg::Connected {
            profile_index,
            result: Ok(connection(tools_available())),
        },
    );
    send(&mut state, Msg::CatalogLoaded(Ok(catalog())));
    assert_eq!(workspace(&state).profile_index, profile_index);
    state
}

fn switch_to(state: &mut AppState, profile_index: usize) {
    send(state, Msg::OpenProfilePicker);
    if let Some(Modal::ProfilePicker(picker)) = &mut state.modal {
        picker.selected_profile_index = profile_index;
    }
    send(state, Msg::Profiles(ProfilesMsg::Connect));
    send(
        state,
        Msg::Connected {
            profile_index,
            result: Ok(connection(tools_available())),
        },
    );
    assert_eq!(workspace(state).profile_index, profile_index);
}

fn run(state: &mut AppState, sql: &str) -> Vec<Command> {
    send(
        state,
        Msg::Query(QueryMsg::Run {
            sql: sql.to_owned(),
        }),
    )
}

fn run_script(tab_id: TabId, sql: &str) -> Vec<Command> {
    vec![Command::RunScript {
        tab_id,
        sql: sql.to_owned(),
        mode: TransactionMode::Transaction,
    }]
}

fn finish(state: &mut AppState, tab_id: TabId) {
    send(
        state,
        Msg::ScriptFinished {
            tab_id,
            report: report(TransactionMode::Transaction, Vec::new()),
        },
    );
}

fn confirm(state: &AppState) -> &Confirm {
    match &state.modal {
        Some(Modal::Confirm(confirm)) => confirm,
        other => panic!("expected a run confirmation, got {other:?}"),
    }
}

fn submit(state: &mut AppState) -> Vec<Command> {
    send(state, Msg::Modal(ModalMsg::Submit))
}

#[test]
fn local_runs_directly() {
    let mut state = state_on(LOCAL_INDEX);
    let tab_id = new_query(&mut state);
    assert_eq!(run(&mut state, "select 1"), run_script(tab_id, "select 1"));
    assert!(state.modal.is_none());
}

#[test]
fn staging_asks_once_per_tab_then_runs_directly() {
    let mut state = state_on(STAGING_INDEX);
    let tab_id = new_query(&mut state);
    assert!(run(&mut state, "select 1; select 2").is_empty());
    let pending_confirm = confirm(&state);
    assert_eq!(
        pending_confirm.kind,
        ConfirmKind::RunScript {
            tab_id,
            sql: "select 1; select 2".to_owned(),
        }
    );
    assert_eq!(
        pending_confirm.message,
        "Run 2 statement(s) on STAGING · Staging (stage.example.com:5432/app)?"
    );
    assert_eq!(submit(&mut state), run_script(tab_id, "select 1; select 2"));
    assert_eq!(
        query_tab(&state, tab_id).confirmed_profile_index,
        Some(STAGING_INDEX)
    );
    finish(&mut state, tab_id);
    assert_eq!(run(&mut state, "select 3"), run_script(tab_id, "select 3"));
    assert!(state.modal.is_none());
}

#[test]
fn staging_asks_again_after_switching_away_and_back() {
    let mut state = state_on(STAGING_INDEX);
    let tab_id = new_query(&mut state);
    run(&mut state, "select 1");
    submit(&mut state);
    finish(&mut state, tab_id);
    switch_to(&mut state, LOCAL_INDEX);
    assert_eq!(query_tab(&state, tab_id).confirmed_profile_index, None);
    switch_to(&mut state, STAGING_INDEX);
    assert_eq!(active_tab_id(&state), tab_id);
    assert!(run(&mut state, "select 1").is_empty());
    confirm(&state);
}

#[test]
fn second_staging_tab_asks_independently() {
    let mut state = state_on(STAGING_INDEX);
    let first_tab_id = new_query(&mut state);
    run(&mut state, "select 1");
    submit(&mut state);
    finish(&mut state, first_tab_id);
    let second_tab_id = new_query(&mut state);
    assert!(run(&mut state, "select 2").is_empty());
    assert_eq!(
        confirm(&state).kind,
        ConfirmKind::RunScript {
            tab_id: second_tab_id,
            sql: "select 2".to_owned(),
        }
    );
}

#[test]
fn prod_asks_every_time() {
    let mut state = state_on(PROD_INDEX);
    let tab_id = new_query(&mut state);
    for _ in 0..2 {
        assert!(run(&mut state, "select 1").is_empty());
        assert_eq!(
            confirm(&state).message,
            "Run 1 statement(s) on PROD · Prod (prod.example.com:5432/app)?"
        );
        assert_eq!(submit(&mut state), run_script(tab_id, "select 1"));
        assert_eq!(query_tab(&state, tab_id).confirmed_profile_index, None);
        finish(&mut state, tab_id);
    }
}

#[test]
fn cancel_runs_nothing_and_notices() {
    let mut state = state_on(PROD_INDEX);
    let tab_id = new_query(&mut state);
    run(&mut state, "select 1");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Cancel)).is_empty());
    assert!(state.modal.is_none());
    let notice = state.notice.clone().expect("notice");
    assert_eq!(notice.level, NoticeLevel::Info);
    assert_eq!(notice.text, "Run cancelled");
    assert_eq!(workspace(&state).running_script_tab_id, None);
    assert_eq!(query_tab(&state, tab_id).confirmed_profile_index, None);
}

#[test]
fn parameters_form_then_confirmation_of_the_bound_sql() {
    let mut state = state_on(PROD_INDEX);
    let tab_id = new_query(&mut state);
    assert!(run(&mut state, "select :id").is_empty());
    assert!(matches!(state.modal, Some(Modal::Parameters(_))));
    send(
        &mut state,
        Msg::Modal(ModalMsg::Input(InputRequest::InsertChar('7'))),
    );
    assert!(submit(&mut state).is_empty());
    assert_eq!(
        confirm(&state).kind,
        ConfirmKind::RunScript {
            tab_id,
            sql: "select '7'".to_owned(),
        }
    );
    assert_eq!(submit(&mut state), run_script(tab_id, "select '7'"));
}

fn execute_file(state: &mut AppState) -> Vec<Command> {
    send(state, Msg::Tabs(TabsMsg::OpenFiles));
    send(state, Msg::SqlFilesListed(Ok(vec![file_entry("a.sql")])));
    let commands = send(state, Msg::Files(FilesMsg::Execute));
    assert_eq!(
        commands,
        vec![Command::OpenFile {
            name: "a.sql".to_owned(),
            run_after_open: true,
        }]
    );
    assert!(state.modal.is_none());
    send(
        state,
        Msg::FileOpened {
            name: "a.sql".to_owned(),
            run_after_open: true,
            result: Ok("select 1".to_owned()),
        },
    )
}

#[test]
fn file_execute_on_local_runs_directly() {
    let mut state = state_on(LOCAL_INDEX);
    let commands = execute_file(&mut state);
    assert_eq!(commands, run_script(active_tab_id(&state), "select 1"));
}

#[test]
fn file_execute_on_prod_asks() {
    let mut state = state_on(PROD_INDEX);
    assert!(execute_file(&mut state).is_empty());
    let tab_id = active_tab_id(&state);
    assert_eq!(
        confirm(&state).kind,
        ConfirmKind::RunScript {
            tab_id,
            sql: "select 1".to_owned(),
        }
    );
    assert_eq!(submit(&mut state), run_script(tab_id, "select 1"));
}

#[test]
fn confirm_rechecks_the_running_guard() {
    let mut state = state_on(PROD_INDEX);
    let tab_id = new_query(&mut state);
    run(&mut state, "select 1");
    workspace_mut(&mut state).running_script_tab_id = Some(tab_id + 100);
    assert!(submit(&mut state).is_empty());
    let notice = state.notice.clone().expect("notice");
    assert_eq!(notice.level, NoticeLevel::Error);
    assert_eq!(notice.text, "A script is already running");
}

#[test]
fn confirming_after_closing_the_tab_runs_nothing() {
    let mut state = state_on(PROD_INDEX);
    new_query(&mut state);
    run(&mut state, "select 1");
    confirm(&state);
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert!(submit(&mut state).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).running_script_tab_id, None);
}
