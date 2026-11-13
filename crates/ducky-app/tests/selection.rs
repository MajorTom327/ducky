mod support;

use ducky_app::{
    AppState, Command, Document, Focus, GridCursor, GridMove, Loadable, Msg, NoticeLevel,
    QueryPane, TabContent, TableMsg,
};
use ducky_core::{Cell, Page, RelationRef, ResultSet};
use support::{
    activate_relation, active_query, active_tab_id, active_table, open_loaded_table, send,
    table_tab, table_tab_mut, workspace, workspace_mut, workspace_state,
};

fn table_message(state: &mut AppState, table_msg: TableMsg) -> Vec<Command> {
    send(state, Msg::Table(table_msg))
}

fn move_cursor_to(state: &mut AppState, row_index: usize, column_index: usize) {
    let tab_id = active_tab_id(state);
    table_tab_mut(state, tab_id).cursor = GridCursor {
        row_index,
        column_index,
    };
}

fn load_rows(state: &mut AppState, columns: &[&str], rows: Vec<Vec<Cell>>) {
    let commands = activate_relation(state, "customers");
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

fn edit_and_read_text(state: &mut AppState) -> String {
    let tab_count = workspace(state).tabs.len();
    assert!(table_message(state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(state).tabs.len(), tab_count + 1);
    active_query(state).document.text.clone()
}

fn error_text(state: &AppState) -> String {
    let notice = state.notice.as_ref().expect("notice shown");
    assert_eq!(notice.level, NoticeLevel::Error);
    notice.text.clone()
}

fn cell(value: &str) -> Cell {
    Some(value.to_owned())
}

#[test]
fn edit_targets_the_cursor_row_when_nothing_is_selected() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let table_tab_id = active_tab_id(&state);
    move_cursor_to(&mut state, 1, 1);
    let text = edit_and_read_text(&mut state);
    assert_eq!(
        text,
        "UPDATE \"public\".\"customers\"\nSET\n  \"name\" = :name\nWHERE \"id\" IN ('r1c0')\n;"
    );
    let query = active_query(&state);
    assert_eq!(
        query.document,
        Document {
            name: None,
            text,
            revision: 1,
        }
    );
    assert_eq!(query.pane, QueryPane::Editor);
    assert_eq!(workspace(&state).focus, Focus::Main);
    let notice = state.notice.as_ref().expect("notice shown");
    assert_eq!(notice.level, NoticeLevel::Info);
    assert_eq!(
        notice.text,
        "UPDATE preset for 1 row(s): fill :name and press F5"
    );
    assert!(!table_tab(&state, table_tab_id).has_selection());
}

#[test]
fn v_marks_consecutive_rows() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleRowMark);
    table_message(&mut state, TableMsg::ToggleRowMark);
    let table = active_table(&state);
    assert_eq!(table.selected_row_indexes(), vec![0, 1]);
    assert_eq!(table.cursor.row_index, 2);
    let table_tab_id = active_tab_id(&state);
    let text = edit_and_read_text(&mut state);
    assert_eq!(
        text,
        "UPDATE \"public\".\"customers\"\nSET\n  \"id\" = :id\nWHERE \"id\" IN ('r0c0', 'r1c0')\n;"
    );
    assert!(!table_tab(&state, table_tab_id).has_selection());
}

#[test]
fn v_on_a_marked_row_unmarks_it() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleRowMark);
    move_cursor_to(&mut state, 0, 0);
    table_message(&mut state, TableMsg::ToggleRowMark);
    assert!(active_table(&state).selected_row_indexes().is_empty());
}

#[test]
fn visual_range_follows_the_cursor_and_is_kept_by_a_second_v() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleVisualSelection);
    assert_eq!(active_table(&state).visual_anchor_row_index, Some(0));
    table_message(&mut state, TableMsg::Move(GridMove::Down));
    table_message(&mut state, TableMsg::Move(GridMove::Down));
    assert_eq!(active_table(&state).selected_row_indexes(), vec![0, 1, 2]);
    table_message(&mut state, TableMsg::ToggleVisualSelection);
    let table = active_table(&state);
    assert_eq!(table.visual_anchor_row_index, None);
    assert_eq!(table.selected_row_indexes(), vec![0, 1, 2]);
    let text = edit_and_read_text(&mut state);
    assert!(
        text.ends_with("WHERE \"id\" IN ('r0c0', 'r1c0', 'r2c0')\n;"),
        "{text}"
    );
    assert_eq!(
        state.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("UPDATE preset for 3 row(s): fill :id and press F5")
    );
}

#[test]
fn edit_combines_marks_and_the_active_range() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleRowMark);
    table_message(&mut state, TableMsg::Move(GridMove::Down));
    table_message(&mut state, TableMsg::ToggleVisualSelection);
    assert_eq!(active_table(&state).selected_row_indexes(), vec![0, 2]);
    let text = edit_and_read_text(&mut state);
    assert!(
        text.ends_with("WHERE \"id\" IN ('r0c0', 'r2c0')\n;"),
        "{text}"
    );
}

#[test]
fn composite_primary_key_targets_tuples() {
    let mut state = workspace_state();
    set_primary_key(&mut state, &["id", "name"]);
    open_loaded_table(&mut state, false);
    move_cursor_to(&mut state, 1, 1);
    table_message(&mut state, TableMsg::ToggleRowMark);
    table_message(&mut state, TableMsg::ToggleRowMark);
    let text = edit_and_read_text(&mut state);
    assert!(
        text.ends_with("WHERE (\"id\", \"name\") IN (('r1c0', 'r1c1'), ('r2c0', 'r2c1'))\n;"),
        "{text}"
    );
}

#[test]
fn primary_key_from_the_catalog_wins_over_id() {
    let mut state = workspace_state();
    set_primary_key(&mut state, &["name"]);
    open_loaded_table(&mut state, false);
    move_cursor_to(&mut state, 2, 0);
    let text = edit_and_read_text(&mut state);
    assert!(text.ends_with("WHERE \"name\" IN ('r2c1')\n;"), "{text}");
}

#[test]
fn missing_key_column_notifies_without_a_tab() {
    let mut state = workspace_state();
    load_rows(
        &mut state,
        &["code", "name"],
        vec![vec![cell("a"), cell("b")]],
    );
    let tab_count = workspace(&state).tabs.len();
    assert!(table_message(&mut state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
    assert_eq!(
        error_text(&state),
        "No primary key or id column to target the rows"
    );
}

#[test]
fn null_key_value_notifies_without_a_tab() {
    let mut state = workspace_state();
    set_primary_key(&mut state, &["name"]);
    open_loaded_table(&mut state, false);
    let tab_count = workspace(&state).tabs.len();
    assert!(table_message(&mut state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
    assert_eq!(error_text(&state), "Row 1 has a NULL key, cannot target it");
    assert!(matches!(
        workspace(&state).active_tab().map(|tab| &tab.content),
        Some(TabContent::Table(_))
    ));
}

#[test]
fn identifiers_and_values_are_quoted() {
    let mut state = workspace_state();
    load_rows(
        &mut state,
        &["id", "Col\"x"],
        vec![vec![cell("it's"), cell("v")]],
    );
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).relation = RelationRef {
        schema: "my schema".to_owned(),
        name: "we\"ird".to_owned(),
    };
    move_cursor_to(&mut state, 0, 1);
    let text = edit_and_read_text(&mut state);
    assert_eq!(
        text,
        "UPDATE \"my schema\".\"we\"\"ird\"\nSET\n  \"Col\"\"x\" = :value\nWHERE \"id\" IN ('it''s')\n;"
    );
    assert_eq!(
        state.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("UPDATE preset for 1 row(s): fill :value and press F5")
    );
}

#[test]
fn next_page_clears_the_selection() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, true);
    table_message(&mut state, TableMsg::ToggleRowMark);
    table_message(&mut state, TableMsg::ToggleVisualSelection);
    assert!(active_table(&state).has_selection());
    table_message(&mut state, TableMsg::NextPage);
    let table = active_table(&state);
    assert!(table.marked_row_indexes.is_empty());
    assert_eq!(table.visual_anchor_row_index, None);
}

#[test]
fn page_loaded_clears_the_selection() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleRowMark);
    let request = active_table(&state).request.clone();
    send(
        &mut state,
        Msg::PageLoaded {
            request,
            result: Ok(support::page(3, false)),
        },
    );
    assert!(!active_table(&state).has_selection());
}

#[test]
fn clear_selection_drops_marks_and_anchor() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleRowMark);
    table_message(&mut state, TableMsg::ToggleVisualSelection);
    table_message(&mut state, TableMsg::ClearSelection);
    assert!(!active_table(&state).has_selection());
}

#[test]
fn edit_without_a_ready_page_does_nothing() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    let tab_count = workspace(&state).tabs.len();
    assert!(table_message(&mut state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
}

#[test]
fn marking_waits_for_the_page() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    assert!(table_message(&mut state, TableMsg::ToggleRowMark).is_empty());
    assert!(active_table(&state).marked_row_indexes.is_empty());
}

#[test]
fn marking_and_editing_an_empty_page_do_nothing() {
    let mut state = workspace_state();
    load_rows(&mut state, &["id", "name"], Vec::new());
    assert!(table_message(&mut state, TableMsg::ToggleRowMark).is_empty());
    assert!(active_table(&state).marked_row_indexes.is_empty());
    assert_eq!(active_table(&state).cursor, GridCursor::default());
    assert!(table_message(&mut state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(&state).tabs.len(), 1);
    assert!(state.notice.is_none());
}

#[test]
fn editing_rows_from_the_structure_tab_does_nothing() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    table_message(&mut state, TableMsg::ToggleTab);
    assert!(table_message(&mut state, TableMsg::EditSelectedRows).is_empty());
    assert_eq!(workspace(&state).tabs.len(), 1);
    assert!(state.notice.is_none());
}
