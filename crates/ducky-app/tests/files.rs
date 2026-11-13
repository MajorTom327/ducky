mod support;

use ducky_app::{
    AppState, Command, ConfirmKind, FilesMsg, Focus, Loadable, MainView, Modal, ModalMsg, Msg,
    NoticeLevel, QueryPane, SidebarAction, SidebarItem, SidebarMsg,
};
use ducky_core::TransactionMode;
use support::{file_entry, select_sidebar_item, send, workspace, workspace_state};

fn files_state() -> AppState {
    let mut state = workspace_state();
    select_sidebar_item(&mut state, |item| {
        *item == SidebarItem::Action(SidebarAction::SqlFiles)
    });
    send(&mut state, Msg::Sidebar(SidebarMsg::Activate));
    send(
        &mut state,
        Msg::SqlFilesListed(Ok(vec![file_entry("a.sql"), file_entry("b.sql")])),
    );
    state
}

#[test]
fn listing_loads_preview_of_selected_entry() {
    let mut state = workspace_state();
    let commands = send(
        &mut state,
        Msg::SqlFilesListed(Ok(vec![file_entry("a.sql")])),
    );
    assert_eq!(
        commands,
        vec![Command::LoadFilePreview {
            name: "a.sql".to_owned()
        }]
    );
    assert!(workspace(&state).files.preview.is_loading());
}

#[test]
fn empty_listing_loads_no_preview() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::SqlFilesListed(Ok(Vec::new()))).is_empty());
    assert_eq!(workspace(&state).files.entries, Loadable::Ready(Vec::new()));
}

#[test]
fn listing_failure_is_reported() {
    let mut state = workspace_state();
    send(&mut state, Msg::SqlFilesListed(Err("denied".to_owned())));
    assert_eq!(
        workspace(&state).files.entries,
        Loadable::Failed("denied".to_owned())
    );
}

#[test]
fn listing_clamps_selection() {
    let mut state = files_state();
    send(&mut state, Msg::Files(FilesMsg::Next));
    send(
        &mut state,
        Msg::SqlFilesListed(Ok(vec![file_entry("a.sql")])),
    );
    assert_eq!(workspace(&state).files.selected_entry_index, 0);
}

#[test]
fn next_moves_and_loads_preview() {
    let mut state = files_state();
    let commands = send(&mut state, Msg::Files(FilesMsg::Next));
    assert_eq!(
        commands,
        vec![Command::LoadFilePreview {
            name: "b.sql".to_owned()
        }]
    );
    assert!(send(&mut state, Msg::Files(FilesMsg::Next)).is_empty());
    assert_eq!(workspace(&state).files.selected_entry_index, 1);
}

#[test]
fn stale_preview_is_ignored() {
    let mut state = files_state();
    send(&mut state, Msg::Files(FilesMsg::Next));
    send(
        &mut state,
        Msg::FilePreviewLoaded {
            name: "a.sql".to_owned(),
            result: Ok("old".to_owned()),
        },
    );
    assert!(workspace(&state).files.preview.is_loading());
    send(
        &mut state,
        Msg::FilePreviewLoaded {
            name: "b.sql".to_owned(),
            result: Ok("new".to_owned()),
        },
    );
    assert_eq!(
        workspace(&state).files.preview,
        Loadable::Ready("new".to_owned())
    );
}

#[test]
fn open_emits_open_file() {
    let mut state = files_state();
    assert_eq!(
        send(&mut state, Msg::Files(FilesMsg::Open)),
        vec![Command::OpenFile {
            name: "a.sql".to_owned(),
            run_after_open: false,
        }]
    );
}

#[test]
fn reload_lists_again() {
    let mut state = files_state();
    assert_eq!(
        send(&mut state, Msg::Files(FilesMsg::Reload)),
        vec![Command::ListSqlFiles]
    );
    assert!(workspace(&state).files.entries.is_loading());
}

#[test]
fn files_messages_outside_files_view_are_ignored() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Files(FilesMsg::Reload)).is_empty());
}

#[test]
fn execute_confirm_open_then_run() {
    let mut state = files_state();
    assert!(send(&mut state, Msg::Files(FilesMsg::Execute)).is_empty());
    let Some(Modal::Confirm(confirm)) = &state.modal else {
        panic!("expected confirm");
    };
    assert_eq!(
        confirm.kind,
        ConfirmKind::ExecuteFile {
            name: "a.sql".to_owned()
        }
    );
    assert_eq!(confirm.message, "Execute a.sql against Local?");
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(
        commands,
        vec![Command::OpenFile {
            name: "a.sql".to_owned(),
            run_after_open: true,
        }]
    );
    let commands = send(
        &mut state,
        Msg::FileOpened {
            name: "a.sql".to_owned(),
            run_after_open: true,
            result: Ok("select 1".to_owned()),
        },
    );
    assert_eq!(
        commands,
        vec![Command::RunScript {
            sql: "select 1".to_owned(),
            mode: TransactionMode::Transaction,
        }]
    );
    let workspace = workspace(&state);
    assert_eq!(workspace.query.document.name.as_deref(), Some("a.sql"));
    assert_eq!(workspace.query.document.text, "select 1");
    assert_eq!(workspace.query.document.revision, 1);
    assert_eq!(
        (workspace.view, workspace.focus, workspace.query.pane),
        (MainView::Query, Focus::Main, QueryPane::Editor)
    );
}

#[test]
fn file_opened_without_run_only_loads_document() {
    let mut state = files_state();
    let commands = send(
        &mut state,
        Msg::FileOpened {
            name: "a.sql".to_owned(),
            run_after_open: false,
            result: Ok("select 1".to_owned()),
        },
    );
    assert!(commands.is_empty());
    assert_eq!(workspace(&state).view, MainView::Query);
}

#[test]
fn file_opened_error_sets_notice() {
    let mut state = files_state();
    send(
        &mut state,
        Msg::FileOpened {
            name: "a.sql".to_owned(),
            run_after_open: false,
            result: Err("missing".to_owned()),
        },
    );
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (NoticeLevel::Error, "missing")
    );
    assert_eq!(workspace(&state).view, MainView::Files);
}
