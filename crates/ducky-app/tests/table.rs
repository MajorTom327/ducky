mod support;

use ducky_app::{
    AppState, Command, GridCursor, GridMove, Loadable, Modal, ModalMsg, Msg, NoticeLevel,
    PromptKind, TableMsg, TableTab, TableView,
};
use ducky_core::{RelationDetails, RelationRef, Sort, SortDirection};
use support::{open_loaded_table, page, send, workspace, workspace_mut, workspace_state};
use tui_input::InputRequest;

fn table(state: &AppState) -> &TableView {
    workspace(state).table.as_ref().expect("table")
}

fn load_current_page(state: &mut AppState, has_next_page: bool) {
    let request = table(state).request.clone();
    send(
        state,
        Msg::PageLoaded {
            request,
            result: Ok(page(3, has_next_page)),
        },
    );
}

#[test]
fn stale_page_is_ignored() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, true);
    send(&mut state, Msg::Table(TableMsg::NextPage));
    let mut stale_request = table(&state).request.clone();
    stale_request.page_index = 0;
    send(
        &mut state,
        Msg::PageLoaded {
            request: stale_request,
            result: Ok(page(3, true)),
        },
    );
    assert!(table(&state).page.is_loading());
}

#[test]
fn page_loaded_clamps_cursor() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    workspace_mut(&mut state)
        .table
        .as_mut()
        .expect("table")
        .cursor = GridCursor {
        row_index: 9,
        column_index: 9,
    };
    send(&mut state, Msg::Table(TableMsg::Reload));
    load_current_page(&mut state, false);
    assert_eq!(
        table(&state).cursor,
        GridCursor {
            row_index: 2,
            column_index: 1
        }
    );
}

#[test]
fn move_clamps_inside_page() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    for _ in 0..5 {
        send(&mut state, Msg::Table(TableMsg::Move(GridMove::Down)));
    }
    assert_eq!(table(&state).cursor.row_index, 2);
}

#[test]
fn sort_cycles_ascending_descending_none() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(&mut state, Msg::Table(TableMsg::Move(GridMove::Right)));
    let expected_sorts = [
        Some(Sort {
            column: "name".to_owned(),
            direction: SortDirection::Ascending,
        }),
        Some(Sort {
            column: "name".to_owned(),
            direction: SortDirection::Descending,
        }),
        None,
    ];
    for expected_sort in expected_sorts {
        let commands = send(&mut state, Msg::Table(TableMsg::CycleSort));
        assert_eq!(table(&state).request.sort, expected_sort);
        assert_eq!(table(&state).request.page_index, 0);
        assert_eq!(
            commands,
            vec![Command::LoadPage(table(&state).request.clone())]
        );
        load_current_page(&mut state, false);
    }
}

#[test]
fn next_page_requires_has_next_page() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    assert!(send(&mut state, Msg::Table(TableMsg::NextPage)).is_empty());
    load_current_page(&mut state, true);
    let commands = send(&mut state, Msg::Table(TableMsg::NextPage));
    assert_eq!(table(&state).request.page_index, 1);
    assert_eq!(
        commands,
        vec![Command::LoadPage(table(&state).request.clone())]
    );
}

#[test]
fn previous_page_requires_page_above_zero() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, true);
    assert!(send(&mut state, Msg::Table(TableMsg::PreviousPage)).is_empty());
    send(&mut state, Msg::Table(TableMsg::NextPage));
    let commands = send(&mut state, Msg::Table(TableMsg::PreviousPage));
    assert_eq!(table(&state).request.page_index, 0);
    assert_eq!(commands.len(), 1);
}

#[test]
fn filter_submit_then_clear() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, true);
    send(&mut state, Msg::Table(TableMsg::NextPage));
    send(&mut state, Msg::Table(TableMsg::StartFilter));
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("expected prompt");
    };
    assert_eq!(
        (&prompt.kind, prompt.title.as_str()),
        (&PromptKind::TableFilter, "WHERE")
    );
    for typed_char in " id > 1 ".chars() {
        send(
            &mut state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(state.modal.is_none());
    assert_eq!(table(&state).request.filter.as_deref(), Some("id > 1"));
    assert_eq!(table(&state).request.page_index, 0);
    assert_eq!(
        commands,
        vec![Command::LoadPage(table(&state).request.clone())]
    );
    let commands = send(&mut state, Msg::Table(TableMsg::ClearFilter));
    assert_eq!(table(&state).request.filter, None);
    assert_eq!(
        commands,
        vec![Command::LoadPage(table(&state).request.clone())]
    );
    assert!(send(&mut state, Msg::Table(TableMsg::ClearFilter)).is_empty());
}

#[test]
fn structure_tab_loads_details_once() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let relation = table(&state).relation.clone();
    let commands = send(&mut state, Msg::Table(TableMsg::ToggleTab));
    assert_eq!(commands, vec![Command::LoadDetails(relation.clone())]);
    assert_eq!(table(&state).tab, TableTab::Structure);
    let details = RelationDetails {
        indexes: Vec::new(),
        foreign_keys: Vec::new(),
        view_definition: None,
    };
    send(
        &mut state,
        Msg::DetailsLoaded {
            relation: relation.clone(),
            result: Ok(details.clone()),
        },
    );
    assert_eq!(table(&state).details, Loadable::Ready(details));
    send(&mut state, Msg::Table(TableMsg::ToggleTab));
    assert!(send(&mut state, Msg::Table(TableMsg::ToggleTab)).is_empty());
    let commands = send(&mut state, Msg::Table(TableMsg::Reload));
    assert_eq!(
        commands,
        vec![
            Command::LoadPage(table(&state).request.clone()),
            Command::LoadDetails(relation)
        ]
    );
}

#[test]
fn stale_details_are_ignored() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(&mut state, Msg::Table(TableMsg::ToggleTab));
    send(
        &mut state,
        Msg::DetailsLoaded {
            relation: RelationRef {
                schema: "public".to_owned(),
                name: "orders".to_owned(),
            },
            result: Err("x".to_owned()),
        },
    );
    assert!(table(&state).details.is_loading());
}

#[test]
fn row_detail_opens_on_cursor_row() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(&mut state, Msg::Table(TableMsg::Move(GridMove::Right)));
    send(&mut state, Msg::Table(TableMsg::OpenRowDetail));
    let Some(Modal::RowDetail(row_detail)) = &state.modal else {
        panic!("expected row detail");
    };
    assert_eq!(row_detail.columns, ["id", "name"]);
    assert_eq!(row_detail.values, [Some("r0c0".to_owned()), None]);
    assert_eq!(row_detail.selected_field_index, 1);
}

#[test]
fn copy_cell_copies_null_as_empty() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(&mut state, Msg::Table(TableMsg::Move(GridMove::Right)));
    let commands = send(&mut state, Msg::Table(TableMsg::CopyCell));
    assert_eq!(commands, vec![Command::CopyToClipboard(String::new())]);
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (NoticeLevel::Info, "Copied")
    );
}

#[test]
fn table_messages_without_table_are_ignored() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Table(TableMsg::Reload)).is_empty());
}
