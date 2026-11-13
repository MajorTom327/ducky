mod support;

use ducky_app::{
    Command, DumpMsg, Modal, ModalMsg, Msg, NoticeLevel, PromptKind, QueryMsg, RowDetail,
    SidebarMsg, TabsMsg,
};
use support::{new_query, send, workspace, workspace_mut, workspace_state};
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
