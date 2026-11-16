mod support;

use ducky_app::{
    AppState, Command, Confirm, ConfirmKind, GridCursor, Loadable, Modal, ModalMsg, Msg,
    NoticeLevel, TabContent, TableMsg,
};
use support::{
    active_tab_id, active_table, open_loaded_table, send, table_tab, table_tab_mut, workspace,
    workspace_mut, workspace_state,
};

fn delete_state() -> AppState {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    state
}

fn notice_of(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.as_ref().expect("notice shown");
    (notice.level, notice.text.clone())
}

fn open_confirm(state: &mut AppState) -> Confirm {
    assert!(send(state, Msg::Table(TableMsg::DeleteSelectedRows)).is_empty());
    match &state.modal {
        Some(Modal::Confirm(confirm)) => confirm.clone(),
        other => panic!("expected a confirm, got {other:?}"),
    }
}

#[test]
fn delete_targets_the_cursor_row_when_nothing_is_selected_and_runs_on_yes() {
    let mut state = delete_state();
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).cursor.row_index = 1;
    let sql = "DELETE FROM \"public\".\"customers\" WHERE \"id\" IN ('r1c0');".to_owned();
    assert_eq!(
        open_confirm(&mut state),
        Confirm {
            kind: ConfirmKind::DeleteRows {
                tab_id,
                sql: sql.clone()
            },
            message: "You are going to delete 1 row. Are you sure?".to_owned(),
        }
    );
    assert_eq!(
        send(&mut state, Msg::Modal(ModalMsg::Submit)),
        vec![Command::DeleteRows { tab_id, sql }]
    );
    assert!(state.modal.is_none());
}

#[test]
fn delete_targets_every_selected_row() {
    let mut state = delete_state();
    send(&mut state, Msg::Table(TableMsg::ToggleRowMark));
    send(&mut state, Msg::Table(TableMsg::ToggleRowMark));
    let confirm = open_confirm(&mut state);
    assert_eq!(
        confirm.message,
        "You are going to delete 2 rows. Are you sure?"
    );
    assert!(matches!(
        confirm.kind,
        ConfirmKind::DeleteRows { sql, .. }
            if sql == "DELETE FROM \"public\".\"customers\" WHERE \"id\" IN ('r0c0', 'r1c0');"
    ));
}

#[test]
fn cancelling_keeps_the_rows() {
    let mut state = delete_state();
    open_confirm(&mut state);
    assert!(send(&mut state, Msg::Modal(ModalMsg::Cancel)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Info, "Delete cancelled".to_owned())
    );
}

#[test]
fn unsaved_drafts_block_the_delete() {
    let mut state = delete_state();
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id)
        .cell_drafts
        .insert(GridCursor::default(), None);
    assert!(send(&mut state, Msg::Table(TableMsg::DeleteSelectedRows)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(notice_of(&state).0, NoticeLevel::Warning);
}

#[test]
fn deleted_rows_reload_the_page_and_failures_are_reported() {
    let mut state = delete_state();
    let tab_id = active_tab_id(&state);
    let request = active_table(&state).request.clone();
    assert_eq!(
        send(
            &mut state,
            Msg::RowsDeleted {
                tab_id,
                result: Ok(vec![2]),
            },
        ),
        vec![Command::LoadPage(request)]
    );
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Success, "Deleted 2 row(s)".to_owned())
    );
    let failed_commands = send(
        &mut state,
        Msg::RowsDeleted {
            tab_id,
            result: Err("ERROR: fk violation".to_owned()),
        },
    );
    assert!(failed_commands.is_empty());
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Error,
            "Delete failed: ERROR: fk violation".to_owned()
        )
    );
}

#[test]
fn deleted_rows_reload_every_tab_of_the_relation() {
    let mut state = delete_state();
    let tab_id = active_tab_id(&state);
    let mut filtered_tab = workspace(&state).active_tab().expect("tab").clone();
    filtered_tab.id = 999;
    if let TabContent::Table(table) = &mut filtered_tab.content {
        table.request.filter = Some("id > 1".to_owned());
    }
    workspace_mut(&mut state).tabs.push(filtered_tab);
    let commands = send(
        &mut state,
        Msg::RowsDeleted {
            tab_id,
            result: Ok(vec![1]),
        },
    );
    assert_eq!(commands.len(), 2);
    assert_eq!(table_tab(&state, 999).page, Loadable::Loading);
}
