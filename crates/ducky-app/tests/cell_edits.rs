mod support;

use std::collections::BTreeMap;

use ducky_app::{
    AppState, CellInput, Command, ConfirmKind, GridCursor, Loadable, Modal, ModalMsg, Msg,
    NoticeLevel, TabId, TableMsg, TabsMsg,
};
use ducky_core::{Cell, Column, Page, ProfileKind, ResultSet};
use support::{
    activate_relation, active_tab_id, active_table, send, table_tab, table_tab_mut, workspace,
    workspace_mut, workspace_state,
};
use tui_input::InputRequest;

const NAME: usize = 1;
const STATUS: usize = 2;
const FLAG: usize = 3;
const ARCHIVED: usize = 4;
const MOOD: usize = 5;
const SHAPE: usize = 6;

fn column(name: &str, data_type: &str, is_nullable: bool, enum_labels: &[&str]) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        is_nullable,
        default_expression: None,
        is_primary_key: name == "id",
        enum_labels: enum_labels
            .iter()
            .map(|label| (*label).to_owned())
            .collect(),
    }
}

fn cell(value: &str) -> Cell {
    Some(value.to_owned())
}

fn set_catalog_columns(state: &mut AppState, relation_name: &str, columns: Vec<Column>) {
    let Loadable::Ready(catalog) = &mut workspace_mut(state).catalog else {
        panic!("catalog ready");
    };
    catalog
        .schemas
        .iter_mut()
        .flat_map(|schema| schema.relations.iter_mut())
        .find(|relation| relation.name == relation_name)
        .expect("relation in catalog")
        .columns = columns;
}

fn set_primary_key(state: &mut AppState, primary_key_names: &[&str]) {
    let Loadable::Ready(catalog) = &mut workspace_mut(state).catalog else {
        panic!("catalog ready");
    };
    let customers = catalog
        .schemas
        .iter_mut()
        .flat_map(|schema| schema.relations.iter_mut())
        .find(|relation| relation.name == "customers")
        .expect("customers relation");
    for column in &mut customers.columns {
        column.is_primary_key = primary_key_names.contains(&column.name.as_str());
    }
}

fn load_page(state: &mut AppState, relation_name: &str, columns: &[&str], rows: Vec<Vec<Cell>>) {
    let commands = activate_relation(state, relation_name);
    let Some(Command::LoadPage(request)) = commands.into_iter().next() else {
        panic!("expected LoadPage");
    };
    send(
        state,
        Msg::PageLoaded {
            request,
            result: Ok(Page {
                result_set: ResultSet {
                    columns: columns.iter().map(|column| (*column).to_owned()).collect(),
                    rows,
                    is_truncated: false,
                },
                has_next_page: false,
            }),
        },
    );
}

fn edit_state() -> AppState {
    let mut state = workspace_state();
    set_catalog_columns(
        &mut state,
        "customers",
        vec![
            column("id", "integer", false, &[]),
            column("name", "text", true, &[]),
            column("status", "order_status", false, &["new", "paid"]),
            column("flag", "boolean", false, &[]),
            column("archived", "boolean", true, &[]),
            column("mood", "mood", true, &["happy", "sad"]),
            column("shape", "geometry(Point,4326)", true, &[]),
        ],
    );
    load_page(
        &mut state,
        "customers",
        &["id", "name", "status", "flag", "archived", "mood", "shape"],
        vec![
            vec![
                cell("1"),
                cell("Ada"),
                cell("new"),
                cell("t"),
                None,
                cell("happy"),
                None,
            ],
            vec![
                cell("2"),
                None,
                cell("paid"),
                cell("f"),
                cell("t"),
                None,
                None,
            ],
            vec![
                cell("3"),
                cell("Cy"),
                cell("new"),
                cell("t"),
                cell("f"),
                cell("sad"),
                None,
            ],
        ],
    );
    state
}

fn move_cursor_to(state: &mut AppState, row_index: usize, column_index: usize) {
    let tab_id = active_tab_id(state);
    table_tab_mut(state, tab_id).cursor = GridCursor {
        row_index,
        column_index,
    };
}

fn table_message(state: &mut AppState, table_msg: TableMsg) -> Vec<Command> {
    send(state, Msg::Table(table_msg))
}

fn modal_message(state: &mut AppState, modal_msg: ModalMsg) -> Vec<Command> {
    send(state, Msg::Modal(modal_msg))
}

fn drafts(state: &AppState) -> BTreeMap<GridCursor, Cell> {
    active_table(state).cell_drafts.clone()
}

fn draft_at(row_index: usize, column_index: usize, value: Cell) -> (GridCursor, Cell) {
    (
        GridCursor {
            row_index,
            column_index,
        },
        value,
    )
}

fn text_input_value(state: &AppState) -> String {
    match &state.modal {
        Some(Modal::CellEditor(editor)) => match &editor.input {
            CellInput::Text(input) => input.value().to_owned(),
            CellInput::Choice { .. } => panic!("expected a text editor"),
        },
        other => panic!("expected a cell editor, got {other:?}"),
    }
}

fn choice_state(state: &AppState) -> (Vec<Cell>, usize) {
    match &state.modal {
        Some(Modal::CellEditor(editor)) => match &editor.input {
            CellInput::Choice {
                options,
                selected_option_index,
            } => (options.clone(), *selected_option_index),
            CellInput::Text(_) => panic!("expected a choice list"),
        },
        other => panic!("expected a cell editor, got {other:?}"),
    }
}

fn type_text(state: &mut AppState, text: &str) {
    modal_message(state, ModalMsg::Input(InputRequest::DeleteLine));
    for character in text.chars() {
        modal_message(state, ModalMsg::Input(InputRequest::InsertChar(character)));
    }
}

fn change_text(state: &mut AppState, row_index: usize, column_index: usize, text: &str) {
    move_cursor_to(state, row_index, column_index);
    table_message(state, TableMsg::ChangeCell);
    type_text(state, text);
    modal_message(state, ModalMsg::Submit);
}

fn notice_of(state: &AppState) -> (NoticeLevel, String) {
    let notice = state.notice.as_ref().expect("notice shown");
    (notice.level, notice.text.clone())
}

#[test]
fn text_editor_opens_with_the_current_value_and_submit_drafts_it() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, NAME);
    assert!(table_message(&mut state, TableMsg::ChangeCell).is_empty());
    let Some(Modal::CellEditor(editor)) = &state.modal else {
        panic!("cell editor open");
    };
    assert_eq!(editor.column_name, "name");
    assert_eq!(editor.column_index, NAME);
    assert_eq!(editor.row_indexes, vec![0]);
    assert_eq!(text_input_value(&state), "Ada");
    modal_message(&mut state, ModalMsg::Input(InputRequest::InsertChar('!')));
    assert!(modal_message(&mut state, ModalMsg::Submit).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(
        drafts(&state),
        BTreeMap::from([draft_at(0, NAME, cell("Ada!"))])
    );
    table_message(&mut state, TableMsg::ChangeCell);
    assert_eq!(text_input_value(&state), "Ada!");
}

#[test]
fn cancelling_the_editor_keeps_the_page_untouched() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, NAME);
    table_message(&mut state, TableMsg::ChangeCell);
    type_text(&mut state, "Bob");
    modal_message(&mut state, ModalMsg::Cancel);
    assert!(state.modal.is_none());
    assert!(drafts(&state).is_empty());
}

#[test]
fn submitting_the_original_value_removes_the_draft() {
    let mut state = edit_state();
    change_text(&mut state, 0, NAME, "Bob");
    assert_eq!(drafts(&state).len(), 1);
    change_text(&mut state, 0, NAME, "Ada");
    assert!(drafts(&state).is_empty());
}

#[test]
fn empty_text_over_a_null_original_is_unchanged() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 1, NAME);
    table_message(&mut state, TableMsg::ChangeCell);
    assert_eq!(text_input_value(&state), "");
    modal_message(&mut state, ModalMsg::Submit);
    assert!(drafts(&state).is_empty());
}

#[test]
fn boolean_toggle_cycles_true_and_false_on_a_not_null_column() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, FLAG);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert_eq!(
        drafts(&state),
        BTreeMap::from([draft_at(0, FLAG, cell("f"))])
    );
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(drafts(&state).is_empty());
    move_cursor_to(&mut state, 1, FLAG);
    table_message(&mut state, TableMsg::ChangeCell);
    assert_eq!(
        drafts(&state),
        BTreeMap::from([draft_at(1, FLAG, cell("t"))])
    );
}

#[test]
fn boolean_toggle_cycles_through_null_on_a_nullable_column() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, ARCHIVED);
    let mut drafted_values = Vec::new();
    for _ in 0..3 {
        table_message(&mut state, TableMsg::ChangeCell);
        drafted_values.push(
            drafts(&state)
                .get(&GridCursor {
                    row_index: 0,
                    column_index: ARCHIVED,
                })
                .cloned(),
        );
    }
    assert_eq!(drafted_values, vec![Some(cell("t")), Some(cell("f")), None]);
}

#[test]
fn enum_choice_lists_labels_and_preselects_the_current_value() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 1, STATUS);
    table_message(&mut state, TableMsg::ChangeCell);
    assert_eq!(choice_state(&state), (vec![cell("new"), cell("paid")], 1));
    modal_message(&mut state, ModalMsg::Previous);
    modal_message(&mut state, ModalMsg::Previous);
    assert_eq!(choice_state(&state).1, 0);
    modal_message(&mut state, ModalMsg::Submit);
    assert_eq!(
        drafts(&state),
        BTreeMap::from([draft_at(1, STATUS, cell("new"))])
    );
}

#[test]
fn nullable_enum_choice_ends_with_null() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 1, MOOD);
    table_message(&mut state, TableMsg::ChangeCell);
    assert_eq!(
        choice_state(&state),
        (vec![cell("happy"), cell("sad"), None], 2)
    );
    modal_message(&mut state, ModalMsg::Next);
    assert_eq!(choice_state(&state).1, 2);
    move_cursor_to(&mut state, 0, MOOD);
    modal_message(&mut state, ModalMsg::Cancel);
    table_message(&mut state, TableMsg::ChangeCell);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Submit);
    assert_eq!(drafts(&state), BTreeMap::from([draft_at(0, MOOD, None)]));
}

#[test]
fn change_applies_to_every_selected_row_and_clears_the_selection() {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).marked_row_indexes = [0, 2].into();
    move_cursor_to(&mut state, 1, NAME);
    table_message(&mut state, TableMsg::ChangeCell);
    let Some(Modal::CellEditor(editor)) = &state.modal else {
        panic!("cell editor open");
    };
    assert_eq!(editor.row_indexes, vec![0, 2]);
    type_text(&mut state, "Zed");
    modal_message(&mut state, ModalMsg::Submit);
    assert_eq!(
        drafts(&state),
        BTreeMap::from([
            draft_at(0, NAME, cell("Zed")),
            draft_at(2, NAME, cell("Zed"))
        ])
    );
    assert!(!table_tab(&state, tab_id).has_selection());
}

#[test]
fn set_null_drafts_null_on_every_selected_row_and_geometry_is_allowed() {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).marked_row_indexes = [0, 1].into();
    move_cursor_to(&mut state, 0, NAME);
    table_message(&mut state, TableMsg::SetCellNull);
    assert!(state.modal.is_none());
    assert_eq!(drafts(&state), BTreeMap::from([draft_at(0, NAME, None)]));
    assert!(!table_tab(&state, tab_id).has_selection());
    move_cursor_to(&mut state, 0, SHAPE);
    table_message(&mut state, TableMsg::SetCellNull);
    assert_eq!(drafts(&state).len(), 1);
    assert_eq!(state.notice, None);
}

#[test]
fn set_null_is_refused_on_a_not_null_column() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, STATUS);
    table_message(&mut state, TableMsg::SetCellNull);
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Warning, "status is NOT NULL".to_owned())
    );
    assert!(drafts(&state).is_empty());
}

#[test]
fn views_are_refused() {
    let mut state = workspace_state();
    load_page(
        &mut state,
        "order_totals",
        &["id", "name"],
        vec![vec![cell("1"), cell("a")]],
    );
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Warning, "Views are read-only".to_owned())
    );
}

#[test]
fn rows_without_a_key_are_refused() {
    let mut state = workspace_state();
    load_page(&mut state, "customers", &["name"], vec![vec![cell("a")]]);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Error,
            "No primary key or id column to target the rows".to_owned()
        )
    );
}

#[test]
fn rows_with_a_null_key_are_refused() {
    let mut state = workspace_state();
    load_page(
        &mut state,
        "customers",
        &["id", "name"],
        vec![vec![cell("1"), cell("a")], vec![None, cell("b")]],
    );
    move_cursor_to(&mut state, 1, 1);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Error,
            "Row 2 has a NULL key, cannot target it".to_owned()
        )
    );
}

#[test]
fn geometry_columns_are_refused() {
    let mut state = edit_state();
    move_cursor_to(&mut state, 0, SHAPE);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Warning,
            "Geometry columns cannot be edited inline".to_owned()
        )
    );
}

#[test]
fn change_is_ignored_while_saving_or_outside_the_catalog() {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).is_saving_drafts = true;
    move_cursor_to(&mut state, 0, NAME);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    table_tab_mut(&mut state, tab_id).is_saving_drafts = false;
    set_catalog_columns(
        &mut state,
        "customers",
        vec![column("id", "integer", false, &[])],
    );
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(state.modal.is_none());
    assert!(state.notice.is_none());
}

#[test]
fn page_changes_filter_reload_and_close_are_blocked_by_drafts() {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    change_text(&mut state, 0, NAME, "Bob");
    table_tab_mut(&mut state, tab_id).request.filter = Some("id > 0".to_owned());
    table_tab_mut(&mut state, tab_id).request.page_index = 1;
    let expected_notice = (
        NoticeLevel::Warning,
        "Save (Ctrl+s) or discard (U) the 1 unsaved change(s) first".to_owned(),
    );
    for msg in [
        Msg::Table(TableMsg::NextPage),
        Msg::Table(TableMsg::PreviousPage),
        Msg::Table(TableMsg::CycleSort),
        Msg::Table(TableMsg::StartFilter),
        Msg::Table(TableMsg::ClearFilter),
        Msg::Table(TableMsg::Reload),
        Msg::Tabs(TabsMsg::Close),
    ] {
        state.notice = None;
        let msg_text = format!("{msg:?}");
        assert!(
            send(&mut state, msg).is_empty(),
            "{msg_text} emitted commands"
        );
        assert!(state.modal.is_none(), "{msg_text} opened a modal");
        assert_eq!(notice_of(&state), expected_notice, "{msg_text}");
    }
    assert_eq!(workspace(&state).tabs.len(), 1);
    assert_eq!(drafts(&state).len(), 1);
    state.notice = None;
    table_message(&mut state, TableMsg::OpenRowDetail);
    assert!(matches!(state.modal, Some(Modal::RowDetail(_))));
    assert!(state.notice.is_none());
}

#[test]
fn page_loaded_keeps_a_drafted_table() {
    let mut state = edit_state();
    change_text(&mut state, 0, NAME, "Bob");
    let request = active_table(&state).request.clone();
    let page_before = active_table(&state).page.clone();
    send(
        &mut state,
        Msg::PageLoaded {
            request,
            result: Ok(Page {
                result_set: ResultSet {
                    columns: vec!["id".to_owned()],
                    rows: Vec::new(),
                    is_truncated: false,
                },
                has_next_page: false,
            }),
        },
    );
    assert_eq!(active_table(&state).page, page_before);
    assert_eq!(drafts(&state).len(), 1);
}

#[test]
fn revert_removes_the_cursor_draft_and_discard_removes_all() {
    let mut state = edit_state();
    change_text(&mut state, 0, NAME, "Bob");
    change_text(&mut state, 2, NAME, "Cyd");
    move_cursor_to(&mut state, 0, NAME);
    assert!(table_message(&mut state, TableMsg::RevertCell).is_empty());
    assert_eq!(
        drafts(&state),
        BTreeMap::from([draft_at(2, NAME, cell("Cyd"))])
    );
    change_text(&mut state, 1, NAME, "Bea");
    state.notice = None;
    table_message(&mut state, TableMsg::DiscardDrafts);
    assert!(drafts(&state).is_empty());
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Info, "Discarded 2 change(s)".to_owned())
    );
    state.notice = None;
    table_message(&mut state, TableMsg::DiscardDrafts);
    assert!(state.notice.is_none());
}

#[test]
fn save_on_local_emits_one_update_per_row() {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    change_text(&mut state, 0, NAME, "O'Hara");
    move_cursor_to(&mut state, 0, MOOD);
    table_message(&mut state, TableMsg::ChangeCell);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Submit);
    move_cursor_to(&mut state, 0, STATUS);
    table_message(&mut state, TableMsg::ChangeCell);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Submit);
    move_cursor_to(&mut state, 1, FLAG);
    table_message(&mut state, TableMsg::ChangeCell);
    let commands = table_message(&mut state, TableMsg::SaveDrafts);
    assert_eq!(
        commands,
        vec![Command::SaveCellEdits {
            tab_id,
            sql: "UPDATE \"public\".\"customers\" SET \"name\" = 'O''Hara', \"status\" = 'paid', \"mood\" = NULL WHERE \"id\" = '1';\nUPDATE \"public\".\"customers\" SET \"flag\" = 't' WHERE \"id\" = '2';".to_owned(),
        }]
    );
    assert!(active_table(&state).is_saving_drafts);
    assert!(table_message(&mut state, TableMsg::SaveDrafts).is_empty());
}

#[test]
fn save_with_a_composite_key_uses_the_original_key_values() {
    let mut state = edit_state();
    set_primary_key(&mut state, &["id", "status"]);
    let tab_id = active_tab_id(&state);
    change_text(&mut state, 0, NAME, "Zed");
    move_cursor_to(&mut state, 0, STATUS);
    table_message(&mut state, TableMsg::ChangeCell);
    modal_message(&mut state, ModalMsg::Next);
    modal_message(&mut state, ModalMsg::Submit);
    assert_eq!(
        table_message(&mut state, TableMsg::SaveDrafts),
        vec![Command::SaveCellEdits {
            tab_id,
            sql: "UPDATE \"public\".\"customers\" SET \"name\" = 'Zed', \"status\" = 'paid' WHERE (\"id\", \"status\") = ('1', 'new');".to_owned(),
        }]
    );
}

#[test]
fn save_without_drafts_does_nothing_and_a_lost_key_is_reported() {
    let mut state = edit_state();
    assert!(table_message(&mut state, TableMsg::SaveDrafts).is_empty());
    change_text(&mut state, 0, NAME, "Zed");
    let mut legacy_key_column = column("legacy_key", "integer", false, &[]);
    legacy_key_column.is_primary_key = true;
    set_catalog_columns(
        &mut state,
        "customers",
        vec![legacy_key_column, column("name", "text", true, &[])],
    );
    assert!(table_message(&mut state, TableMsg::SaveDrafts).is_empty());
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Error,
            "No primary key or id column to target the rows".to_owned()
        )
    );
    assert!(!active_table(&state).is_saving_drafts);
}

#[test]
fn save_on_prod_asks_for_confirmation_then_runs() {
    let mut state = edit_state();
    workspace_mut(&mut state).profile.kind = ProfileKind::Prod;
    let tab_id = active_tab_id(&state);
    change_text(&mut state, 0, NAME, "Zed");
    move_cursor_to(&mut state, 0, FLAG);
    table_message(&mut state, TableMsg::ChangeCell);
    assert!(table_message(&mut state, TableMsg::SaveDrafts).is_empty());
    let profile = &workspace(&state).profile;
    let expected_message = format!(
        "Save 2 change(s) in 1 row(s) on PROD · {} ({})?",
        profile.display_name(),
        profile.target()
    );
    let expected_sql =
        "UPDATE \"public\".\"customers\" SET \"name\" = 'Zed', \"flag\" = 'f' WHERE \"id\" = '1';"
            .to_owned();
    let Some(Modal::Confirm(confirm)) = &state.modal else {
        panic!("confirm open");
    };
    assert_eq!(confirm.message, expected_message);
    assert_eq!(
        confirm.kind,
        ConfirmKind::SaveCellEdits {
            tab_id,
            sql: expected_sql.clone(),
        }
    );
    assert!(!active_table(&state).is_saving_drafts);
    assert_eq!(
        modal_message(&mut state, ModalMsg::Submit),
        vec![Command::SaveCellEdits {
            tab_id,
            sql: expected_sql,
        }]
    );
    assert!(active_table(&state).is_saving_drafts);
}

#[test]
fn cancelling_the_staging_confirmation_keeps_the_drafts() {
    let mut state = edit_state();
    workspace_mut(&mut state).profile.kind = ProfileKind::Staging;
    change_text(&mut state, 0, NAME, "Zed");
    table_message(&mut state, TableMsg::SaveDrafts);
    assert!(matches!(state.modal, Some(Modal::Confirm(_))));
    assert!(modal_message(&mut state, ModalMsg::Cancel).is_empty());
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Info, "Save cancelled".to_owned())
    );
    assert_eq!(drafts(&state).len(), 1);
    assert!(!active_table(&state).is_saving_drafts);
}

fn saving_state() -> (AppState, TabId) {
    let mut state = edit_state();
    let tab_id = active_tab_id(&state);
    change_text(&mut state, 0, NAME, "Zed");
    change_text(&mut state, 2, NAME, "Cyd");
    table_message(&mut state, TableMsg::SaveDrafts);
    (state, tab_id)
}

#[test]
fn saved_with_every_row_updated_reloads_the_page() {
    let (mut state, tab_id) = saving_state();
    let request = active_table(&state).request.clone();
    let commands = send(
        &mut state,
        Msg::CellEditsSaved {
            tab_id,
            result: Ok(vec![1, 1]),
        },
    );
    assert_eq!(commands, vec![Command::LoadPage(request)]);
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Success, "Saved 2 row(s)".to_owned())
    );
    let table = active_table(&state);
    assert!(table.cell_drafts.is_empty());
    assert!(!table.is_saving_drafts);
    assert_eq!(table.page, Loadable::Loading);
}

#[test]
fn saved_with_a_missed_row_warns() {
    let (mut state, tab_id) = saving_state();
    let commands = send(
        &mut state,
        Msg::CellEditsSaved {
            tab_id,
            result: Ok(vec![1, 0]),
        },
    );
    assert_eq!(commands.len(), 1);
    assert_eq!(
        notice_of(&state),
        (
            NoticeLevel::Warning,
            "Saved: 1 row(s) updated for 2 edited row(s)".to_owned()
        )
    );
    assert!(drafts(&state).is_empty());
}

#[test]
fn failed_save_keeps_the_drafts() {
    let (mut state, tab_id) = saving_state();
    let commands = send(
        &mut state,
        Msg::CellEditsSaved {
            tab_id,
            result: Err("ERROR: boom".to_owned()),
        },
    );
    assert!(commands.is_empty());
    assert_eq!(
        notice_of(&state),
        (NoticeLevel::Error, "Save failed: ERROR: boom".to_owned())
    );
    assert_eq!(drafts(&state).len(), 2);
    assert!(!active_table(&state).is_saving_drafts);
}

#[test]
fn saved_for_a_closed_tab_only_notifies() {
    let (mut state, _) = saving_state();
    for result in [Ok(vec![1]), Err("gone".to_owned())] {
        let commands = send(
            &mut state,
            Msg::CellEditsSaved {
                tab_id: 999,
                result,
            },
        );
        assert!(commands.is_empty());
        assert!(state.notice.is_some());
    }
    assert!(active_table(&state).is_saving_drafts);
    assert_eq!(drafts(&state).len(), 2);
}

#[test]
fn reconnecting_reloads_the_page_and_drops_the_drafts() {
    let mut state = edit_state();
    change_text(&mut state, 0, NAME, "Zed");
    send(&mut state, Msg::Reconnect);
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(support::connection(support::tools_available())),
        },
    );
    assert!(commands.contains(&Command::LoadPage(active_table(&state).request.clone())));
    assert!(drafts(&state).is_empty());
}
