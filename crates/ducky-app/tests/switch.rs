mod support;

use ducky_app::{
    AppState, Command, Connection, JobState, Loadable, ModalMsg, Msg, NoticeLevel, ProfilesMsg,
    QueryMsg, RunState, TableTab,
};
use ducky_core::{SessionInfo, TransactionMode};
use support::{
    active_tab_id, active_table, new_query, open_loaded_table, profiles, query_tab, query_tab_mut,
    report, rows_result, send, table_tab, table_tab_mut, tools_missing, workspace, workspace_mut,
    workspace_state,
};
use tui_input::InputRequest;

fn submit_switch_to_prod(state: &mut AppState) -> Vec<Command> {
    send(state, Msg::OpenPalette);
    for typed_char in "switch profile".chars() {
        send(
            state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
    send(state, Msg::Modal(ModalMsg::Submit));
    for typed_char in "prod".chars() {
        send(
            state,
            Msg::Profiles(ProfilesMsg::Filter(InputRequest::InsertChar(typed_char))),
        );
    }
    send(state, Msg::Profiles(ProfilesMsg::Connect))
}

fn prod_connection() -> Connection {
    Connection {
        session: SessionInfo {
            database: "prod_db".to_owned(),
            user: "reader".to_owned(),
            server_version: "17.2".to_owned(),
        },
        tools: tools_missing(),
    }
}

fn notice(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.clone().expect("notice");
    (notice.level, notice.text)
}

#[test]
fn switch_emits_connect_and_marks_the_pending_profile() {
    let mut state = workspace_state();
    let commands = submit_switch_to_prod(&mut state);
    assert_eq!(
        commands,
        vec![Command::Connect {
            profile_index: 1,
            profile: profiles()[1].clone(),
        }]
    );
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).switching_profile_index, Some(1));
    assert_eq!(workspace(&state).profile_index, 0);
    assert_eq!(
        notice(&state),
        (NoticeLevel::Info, "Connecting to Prod…".to_owned())
    );
}

#[test]
fn switch_is_refused_while_a_script_runs() {
    let mut state = workspace_state();
    workspace_mut(&mut state).running_script_tab_id = Some(1);
    assert!(submit_switch_to_prod(&mut state).is_empty());
    assert_eq!(workspace(&state).switching_profile_index, None);
    assert_eq!(
        notice(&state),
        (
            NoticeLevel::Error,
            "Cancel the running script before switching profile".to_owned()
        )
    );
}

#[test]
fn switch_is_refused_while_a_dump_or_restore_runs() {
    for is_dump in [true, false] {
        let mut state = workspace_state();
        match is_dump {
            true => workspace_mut(&mut state).dump.job = JobState::Running,
            false => workspace_mut(&mut state).restore.job = JobState::Running,
        }
        assert!(submit_switch_to_prod(&mut state).is_empty());
        assert_eq!(workspace(&state).switching_profile_index, None);
        assert_eq!(
            notice(&state),
            (
                NoticeLevel::Error,
                "Wait for the running dump or restore before switching profile".to_owned()
            )
        );
    }
}

#[test]
fn connected_switch_swaps_profile_keeps_query_tabs_and_reloads_tables() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let table_tab_id = active_tab_id(&state);
    let customers_request = active_table(&state).request.clone();
    table_tab_mut(&mut state, table_tab_id).details = Loadable::Failed("old".to_owned());
    let query_tab_id = new_query(&mut state);
    query_tab_mut(&mut state, query_tab_id).document.text = "select 1".to_owned();
    query_tab_mut(&mut state, query_tab_id).run = RunState::Finished(report(
        TransactionMode::Transaction,
        vec![rows_result("select 1", 1)],
    ));
    submit_switch_to_prod(&mut state);
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 1,
            result: Ok(prod_connection()),
        },
    );
    assert_eq!(
        commands,
        vec![Command::LoadCatalog, Command::LoadPage(customers_request)]
    );
    let workspace = workspace(&state);
    assert_eq!(workspace.profile_index, 1);
    assert_eq!(workspace.profile, profiles()[1]);
    assert_eq!(workspace.session, prod_connection().session);
    assert_eq!(workspace.tools, tools_missing());
    assert_eq!(workspace.switching_profile_index, None);
    assert!(workspace.catalog.is_loading());
    assert_eq!(workspace.tabs.len(), 2);
    let table = table_tab(&state, table_tab_id);
    assert!(table.page.is_loading());
    assert_eq!(table.details, Loadable::Idle);
    let query = query_tab(&state, query_tab_id);
    assert_eq!(query.document.text, "select 1");
    assert!(matches!(query.run, RunState::Finished(_)));
    assert_eq!(
        notice(&state),
        (NoticeLevel::Success, "Switched to Prod".to_owned())
    );
}

#[test]
fn switch_reloads_details_of_tables_shown_on_the_structure_tab() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let table_tab_id = active_tab_id(&state);
    let relation = active_table(&state).relation.clone();
    table_tab_mut(&mut state, table_tab_id).tab = TableTab::Structure;
    submit_switch_to_prod(&mut state);
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 1,
            result: Ok(prod_connection()),
        },
    );
    assert!(commands.contains(&Command::LoadDetails(relation)));
    assert!(table_tab(&state, table_tab_id).details.is_loading());
}

#[test]
fn failed_switch_keeps_the_current_profile() {
    let mut state = workspace_state();
    submit_switch_to_prod(&mut state);
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 1,
            result: Err("refused".to_owned()),
        },
    );
    assert!(commands.is_empty());
    let workspace = workspace(&state);
    assert_eq!(workspace.profile_index, 0);
    assert_eq!(workspace.profile.label, "Local");
    assert_eq!(workspace.switching_profile_index, None);
    assert!(workspace.catalog.ready().is_some());
    assert_eq!(
        notice(&state),
        (
            NoticeLevel::Error,
            "Could not connect to Prod: refused".to_owned()
        )
    );
}

#[test]
fn late_connected_for_an_unrelated_profile_is_ignored() {
    let mut state = workspace_state();
    for result in [Ok(prod_connection()), Err("late".to_owned())] {
        let commands = send(
            &mut state,
            Msg::Connected {
                profile_index: 1,
                result,
            },
        );
        assert!(commands.is_empty());
        assert_eq!(workspace(&state).profile_index, 0);
        assert_eq!(workspace(&state).session.database, "ducky");
        assert!(state.notice.is_none());
    }
}

#[test]
fn run_records_the_profile_it_ran_on() {
    let mut state = workspace_state();
    let query_tab_id = new_query(&mut state);
    assert_eq!(query_tab(&state, query_tab_id).ran_on_profile_label, None);
    send(
        &mut state,
        Msg::Query(QueryMsg::Run {
            sql: "select 1".to_owned(),
        }),
    );
    assert_eq!(
        query_tab(&state, query_tab_id)
            .ran_on_profile_label
            .as_deref(),
        Some("Local")
    );
    assert_eq!(workspace(&state).running_script_tab_id, Some(query_tab_id));
}
