mod support;

use ducky_app::{
    AppState, Command, Focus, GridCursor, Loadable, Modal, ModalMsg, Msg, NoticeLevel, TableMsg,
    TableTab,
};
use ducky_core::{
    Catalog, Cell, ForeignKeyLink, Page, RelationKind, RelationRef, ResultSet, Schema,
};
use support::{
    activate_relation, active_tab_id, active_table, connected_state, relation, send, table_tab_mut,
    workspace, workspace_mut,
};

fn relation_ref(schema: &str, name: &str) -> RelationRef {
    RelationRef {
        schema: schema.to_owned(),
        name: name.to_owned(),
    }
}

fn link(
    name: &str,
    columns: &[&str],
    referenced: RelationRef,
    referenced_columns: &[&str],
) -> ForeignKeyLink {
    ForeignKeyLink {
        name: name.to_owned(),
        columns: columns.iter().map(|column| (*column).to_owned()).collect(),
        referenced,
        referenced_columns: referenced_columns
            .iter()
            .map(|column| (*column).to_owned())
            .collect(),
    }
}

fn linked_table(name: &str, foreign_keys: Vec<ForeignKeyLink>) -> ducky_core::Relation {
    let mut linked_relation = relation(name, RelationKind::Table);
    linked_relation.foreign_keys = foreign_keys;
    linked_relation
}

fn linked_catalog() -> Catalog {
    let customers = relation_ref("public", "customers");
    let orders = relation_ref("public", "orders");
    Catalog {
        schemas: vec![
            Schema {
                name: "analytics".to_owned(),
                relations: vec![linked_table(
                    "events",
                    vec![link(
                        "events_customer_fkey",
                        &["customer_id"],
                        customers.clone(),
                        &["id"],
                    )],
                )],
            },
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    linked_table("customers", Vec::new()),
                    linked_table(
                        "orders",
                        vec![link(
                            "orders_customer_fkey",
                            &["customer_id"],
                            customers,
                            &["id"],
                        )],
                    ),
                    linked_table(
                        "shipments",
                        vec![link(
                            "shipments_order_fkey",
                            &["order_region", "order_code"],
                            orders,
                            &["region", "code"],
                        )],
                    ),
                ],
            },
        ],
    }
}

fn linked_state() -> AppState {
    let mut state = connected_state(support::tools_available());
    send(&mut state, Msg::CatalogLoaded(Ok(linked_catalog())));
    state
}

fn open_with_row(
    state: &mut AppState,
    schema: &str,
    name: &str,
    columns: &[&str],
    row: Vec<Cell>,
    column_index: usize,
) {
    workspace_mut(state).schema_name = schema.to_owned();
    let commands = activate_relation(state, name);
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
                    rows: vec![row],
                    is_truncated: false,
                },
                has_next_page: false,
            }),
        },
    );
    let tab_id = active_tab_id(state);
    table_tab_mut(state, tab_id).cursor = GridCursor {
        row_index: 0,
        column_index,
    };
}

fn cell(value: &str) -> Cell {
    Some(value.to_owned())
}

fn open_orders(state: &mut AppState, customer_id: Cell) {
    open_with_row(
        state,
        "public",
        "orders",
        &["id", "customer_id", "region", "code"],
        vec![cell("7"), customer_id, cell("eu"), cell("A'1")],
        1,
    );
}

fn loaded_request(commands: &[Command]) -> &ducky_core::PageRequest {
    match commands {
        [Command::LoadPage(request)] => request,
        other => panic!("expected one LoadPage, got {other:?}"),
    }
}

fn notice_text(state: &AppState) -> String {
    let notice = state.notice.as_ref().expect("notice shown");
    assert_eq!(notice.level, NoticeLevel::Info);
    notice.text.clone()
}

#[test]
fn go_to_referenced_opens_a_filtered_tab_on_the_referenced_key() {
    let mut state = linked_state();
    open_orders(&mut state, cell("42"));
    let tab_count = workspace(&state).tabs.len();
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    let request = loaded_request(&commands);
    assert_eq!(request.relation, relation_ref("public", "customers"));
    assert_eq!(request.filter.as_deref(), Some("\"id\" = '42'"));
    assert_eq!(request.page_index, 0);
    assert_eq!(request.page_size, state.page_size);
    assert_eq!(workspace(&state).tabs.len(), tab_count + 1);
    assert_eq!(
        active_table(&state).relation,
        relation_ref("public", "customers")
    );
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn go_to_referenced_filters_every_column_of_a_composite_key() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "public",
        "shipments",
        &["id", "order_region", "order_code"],
        vec![cell("1"), cell("eu"), cell("A'1")],
        2,
    );
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    let request = loaded_request(&commands);
    assert_eq!(request.relation, relation_ref("public", "orders"));
    assert_eq!(
        request.filter.as_deref(),
        Some("\"region\" = 'eu' AND \"code\" = 'A''1'")
    );
}

#[test]
fn go_to_referenced_on_null_only_notifies() {
    let mut state = linked_state();
    open_orders(&mut state, None);
    let tab_count = workspace(&state).tabs.len();
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    assert!(commands.is_empty());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
    assert_eq!(
        notice_text(&state),
        "customer_id is NULL, nothing to follow"
    );
}

#[test]
fn go_to_referenced_on_a_plain_column_only_notifies() {
    let mut state = linked_state();
    open_orders(&mut state, cell("42"));
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).cursor.column_index = 2;
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    assert!(commands.is_empty());
    assert_eq!(notice_text(&state), "region is not a foreign key");
}

#[test]
fn go_to_referenced_is_ignored_on_the_structure_tab() {
    let mut state = linked_state();
    open_orders(&mut state, cell("42"));
    let tab_id = active_tab_id(&state);
    table_tab_mut(&mut state, tab_id).tab = TableTab::Structure;
    assert!(send(&mut state, Msg::Table(TableMsg::GoToReferenced)).is_empty());
    assert!(state.notice.is_none());
}

#[test]
fn go_to_referenced_reuses_an_identical_filtered_tab() {
    let mut state = linked_state();
    open_orders(&mut state, cell("42"));
    let orders_tab_id = active_tab_id(&state);
    send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    let customers_tab_id = active_tab_id(&state);
    let orders_tab_index = workspace(&state)
        .tab_index(orders_tab_id)
        .expect("orders tab");
    workspace_mut(&mut state).active_tab_index = orders_tab_index;
    let tab_count = workspace(&state).tabs.len();
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    assert!(commands.is_empty());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
    assert_eq!(active_tab_id(&state), customers_tab_id);
}

#[test]
fn go_to_referenced_in_another_schema_switches_the_sidebar_schema() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "analytics",
        "events",
        &["id", "customer_id"],
        vec![cell("1"), cell("5")],
        1,
    );
    workspace_mut(&mut state).sidebar.selected_item_index = 1;
    assert_eq!(workspace(&state).schema_name, "analytics");
    let commands = send(&mut state, Msg::Table(TableMsg::GoToReferenced));
    assert_eq!(
        loaded_request(&commands).relation,
        relation_ref("public", "customers")
    );
    assert_eq!(workspace(&state).schema_name, "public");
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
}

#[test]
fn find_referencing_without_incoming_links_notifies() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "public",
        "shipments",
        &["id", "order_region", "order_code"],
        vec![cell("1"), cell("eu"), cell("A1")],
        0,
    );
    let commands = send(&mut state, Msg::Table(TableMsg::FindReferencing));
    assert!(commands.is_empty());
    assert_eq!(notice_text(&state), "No table references public.shipments");
}

#[test]
fn find_referencing_with_one_link_opens_it_directly() {
    let mut state = linked_state();
    open_orders(&mut state, cell("42"));
    let commands = send(&mut state, Msg::Table(TableMsg::FindReferencing));
    let request = loaded_request(&commands);
    assert_eq!(request.relation, relation_ref("public", "shipments"));
    assert_eq!(
        request.filter.as_deref(),
        Some("\"order_region\" = 'eu' AND \"order_code\" = 'A''1'")
    );
    assert!(state.modal.is_none());
}

#[test]
fn find_referencing_with_several_links_opens_the_picker() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "public",
        "customers",
        &["id", "name"],
        vec![cell("42"), cell("Ada")],
        1,
    );
    let commands = send(&mut state, Msg::Table(TableMsg::FindReferencing));
    assert!(commands.is_empty());
    let Some(Modal::ReferencePicker {
        links,
        selected_link_index,
    }) = &state.modal
    else {
        panic!("expected the reference picker");
    };
    let referencing_relations: Vec<&RelationRef> =
        links.iter().map(|(referencing, _)| referencing).collect();
    assert_eq!(
        referencing_relations,
        [
            &relation_ref("analytics", "events"),
            &relation_ref("public", "orders")
        ]
    );
    assert_eq!(*selected_link_index, 0);
    send(&mut state, Msg::Modal(ModalMsg::Next));
    send(&mut state, Msg::Modal(ModalMsg::Next));
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    let request = loaded_request(&commands);
    assert_eq!(request.relation, relation_ref("public", "orders"));
    assert_eq!(request.filter.as_deref(), Some("\"customer_id\" = '42'"));
    assert!(state.modal.is_none());
}

#[test]
fn reference_picker_cancel_opens_nothing() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "public",
        "customers",
        &["id", "name"],
        vec![cell("42"), cell("Ada")],
        0,
    );
    send(&mut state, Msg::Table(TableMsg::FindReferencing));
    let tab_count = workspace(&state).tabs.len();
    assert!(send(&mut state, Msg::Modal(ModalMsg::Cancel)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).tabs.len(), tab_count);
}

#[test]
fn find_referencing_on_a_null_key_notifies() {
    let mut state = linked_state();
    open_with_row(
        &mut state,
        "public",
        "orders",
        &["id", "customer_id", "region", "code"],
        vec![cell("7"), cell("1"), None, cell("A1")],
        0,
    );
    let commands = send(&mut state, Msg::Table(TableMsg::FindReferencing));
    assert!(commands.is_empty());
    assert_eq!(notice_text(&state), "region is NULL, nothing to follow");
}

fn clear_page_rows(state: &mut AppState) {
    let tab_id = active_tab_id(state);
    if let Loadable::Ready(page) = &mut table_tab_mut(state, tab_id).page {
        page.result_set.rows.clear();
    }
}

fn assert_nothing_followed(state: &AppState) {
    assert_eq!(workspace(state).tabs.len(), 1);
    assert!(state.modal.is_none());
    assert!(state.notice.is_none());
}

#[test]
fn following_links_from_an_empty_page_does_nothing() {
    let mut state = linked_state();
    open_orders(&mut state, cell("3"));
    clear_page_rows(&mut state);
    assert!(send(&mut state, Msg::Table(TableMsg::GoToReferenced)).is_empty());
    assert_nothing_followed(&state);
    assert!(send(&mut state, Msg::Table(TableMsg::FindReferencing)).is_empty());
    assert_nothing_followed(&state);
}

#[test]
fn find_referencing_needs_the_data_tab_and_a_ready_catalog() {
    let mut state = linked_state();
    open_orders(&mut state, cell("3"));
    send(&mut state, Msg::Table(TableMsg::ToggleTab));
    assert_eq!(active_table(&state).tab, TableTab::Structure);
    assert!(send(&mut state, Msg::Table(TableMsg::FindReferencing)).is_empty());
    assert_nothing_followed(&state);
    send(&mut state, Msg::Table(TableMsg::ToggleTab));
    send(
        &mut state,
        Msg::CatalogLoaded(Err("connection reset".to_owned())),
    );
    assert!(send(&mut state, Msg::Table(TableMsg::FindReferencing)).is_empty());
    assert_nothing_followed(&state);
}
