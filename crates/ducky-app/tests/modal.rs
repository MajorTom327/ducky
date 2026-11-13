mod support;

use ducky_app::{
    Command, DumpMsg, Modal, ModalMsg, Msg, NoticeLevel, PromptKind, QueryMsg, RowDetail,
    SidebarMsg, TabsMsg,
};
use std::time::Duration;

use ducky_app::{SaveFailure, TableMsg};
use ducky_core::{ResultSet, StatementOutcome, StatementResult, TransactionMode};
use support::{
    activate_relation, active_tab_id, new_query, report, send, table_tab, workspace, workspace_mut,
    workspace_state,
};
use tui_input::InputRequest;

fn type_text(state: &mut ducky_app::AppState, text: &str) {
    for typed_char in text.chars() {
        send(
            state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
}

#[test]
fn sidebar_filter_input_updates_filter_live() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.selected_item_index = 2;
    send(&mut state, Msg::Sidebar(SidebarMsg::StartFilter));
    type_text(&mut state, "cus");
    assert_eq!(workspace(&state).sidebar.filter, "cus");
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).sidebar.filter, "cus");
}

#[test]
fn cancel_sidebar_filter_clears_it() {
    let mut state = workspace_state();
    send(&mut state, Msg::Sidebar(SidebarMsg::StartFilter));
    type_text(&mut state, "cus");
    send(&mut state, Msg::Modal(ModalMsg::Cancel));
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).sidebar.filter, "");
}

#[test]
fn schema_picker_selects_schema() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.filter = "x".to_owned();
    workspace_mut(&mut state).sidebar.selected_item_index = 2;
    send(&mut state, Msg::OpenSchemaPicker);
    send(&mut state, Msg::Modal(ModalMsg::Previous));
    send(&mut state, Msg::Modal(ModalMsg::Previous));
    assert!(matches!(
        state.modal,
        Some(Modal::SchemaPicker {
            selected_schema_index: 0
        })
    ));
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    let workspace = workspace(&state);
    assert_eq!(workspace.schema_name, "analytics");
    assert_eq!(workspace.sidebar.filter, "");
    assert_eq!(workspace.sidebar.selected_item_index, 0);
    assert!(state.modal.is_none());
}

#[test]
fn schema_picker_next_clamps() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenSchemaPicker);
    send(&mut state, Msg::Modal(ModalMsg::Next));
    assert!(matches!(
        state.modal,
        Some(Modal::SchemaPicker {
            selected_schema_index: 1
        })
    ));
}

#[test]
fn save_as_requires_a_name() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "x".to_owned(),
        }),
    );
    type_text(&mut state, "  ");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (NoticeLevel::Error, "File name is required")
    );
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("prompt stays open");
    };
    assert_eq!(
        prompt.kind,
        PromptKind::SaveFileAs {
            content: "x".to_owned()
        }
    );
    assert_eq!(workspace(&state).pending_save, None);
}

#[test]
fn cancel_overwrite_clears_pending_save() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "x".to_owned(),
        }),
    );
    type_text(&mut state, "a");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(workspace(&state).pending_save.is_some());
    send(
        &mut state,
        Msg::FileSaved {
            name: "a".to_owned(),
            result: Err(ducky_app::SaveFailure::AlreadyExists),
        },
    );
    send(&mut state, Msg::Modal(ModalMsg::Cancel));
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).pending_save, None);
}

#[test]
fn dump_file_name_submit_sets_custom_name() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::OpenDump));
    send(&mut state, Msg::Dump(DumpMsg::EditFileName));
    type_text(&mut state, " nightly ");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        workspace(&state).dump.custom_file_name.as_deref(),
        Some("nightly")
    );
    send(&mut state, Msg::Dump(DumpMsg::EditFileName));
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("expected prompt");
    };
    assert_eq!(prompt.input.value(), "nightly");
    send(
        &mut state,
        Msg::Modal(ModalMsg::Input(InputRequest::DeleteLine)),
    );
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(workspace(&state).dump.custom_file_name, None);
}

#[test]
fn row_detail_navigates_and_copies() {
    let mut state = workspace_state();
    state.modal = Some(Modal::RowDetail(RowDetail {
        columns: vec!["a".to_owned(), "b".to_owned()],
        values: vec![Some("1".to_owned()), None],
        selected_field_index: 0,
    }));
    assert_eq!(
        send(&mut state, Msg::Modal(ModalMsg::CopyValue)),
        vec![Command::CopyToClipboard("1".to_owned())]
    );
    send(&mut state, Msg::Modal(ModalMsg::Next));
    send(&mut state, Msg::Modal(ModalMsg::Next));
    assert_eq!(
        send(&mut state, Msg::Modal(ModalMsg::CopyValue)),
        vec![Command::CopyToClipboard(String::new())]
    );
    assert_eq!(
        state.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("Copied")
    );
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(state.modal.is_none());
}

#[test]
fn modal_messages_without_modal_are_ignored() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(send(&mut state, Msg::Modal(ModalMsg::CopyValue)).is_empty());
    assert!(state.modal.is_none());
}

#[test]
fn input_and_navigation_are_ignored_by_modals_without_them() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenHelp);
    assert!(
        send(
            &mut state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar('x')))
        )
        .is_empty()
    );
    assert!(send(&mut state, Msg::Modal(ModalMsg::Next)).is_empty());
    assert!(matches!(state.modal, Some(Modal::Help)));
    assert_eq!(workspace(&state).sidebar.filter, "");
}

#[test]
fn copy_value_of_a_row_without_columns_copies_nothing() {
    let mut state = workspace_state();
    let tab_id = new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::Run {
            sql: "select from orders".to_owned(),
        }),
    );
    let zero_column_result = StatementResult {
        sql: "select from orders".to_owned(),
        outcome: StatementOutcome::Rows(ResultSet {
            columns: Vec::new(),
            rows: vec![Vec::new()],
            is_truncated: false,
        }),
        elapsed: Duration::from_millis(1),
    };
    send(
        &mut state,
        Msg::ScriptFinished {
            tab_id,
            report: report(TransactionMode::Transaction, vec![zero_column_result]),
        },
    );
    state.notice = None;
    send(&mut state, Msg::Query(QueryMsg::OpenRowDetail));
    assert!(matches!(
        &state.modal,
        Some(Modal::RowDetail(row_detail)) if row_detail.columns.is_empty()
    ));
    assert!(send(&mut state, Msg::Modal(ModalMsg::CopyValue)).is_empty());
    assert!(state.notice.is_none());
}

#[test]
fn table_filter_submitted_after_leaving_the_table_is_dropped() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    let table_tab_id = active_tab_id(&state);
    send(&mut state, Msg::Table(TableMsg::StartFilter));
    type_text(&mut state, "id = 1");
    send(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(table_tab(&state, table_tab_id).request.filter, None);
}

#[test]
fn save_as_submitted_after_closing_the_tab_saves_nothing() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "select 1".to_owned(),
        }),
    );
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    type_text(&mut state, "report");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
    assert!(workspace(&state).pending_save.is_none());
}

#[test]
fn overwrite_confirmed_after_closing_the_tab_saves_nothing() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::SaveAs {
            content: "select 1".to_owned(),
        }),
    );
    type_text(&mut state, "report");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    send(
        &mut state,
        Msg::FileSaved {
            name: "report".to_owned(),
            result: Err(SaveFailure::AlreadyExists),
        },
    );
    assert!(matches!(state.modal, Some(Modal::Confirm(_))));
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
}
