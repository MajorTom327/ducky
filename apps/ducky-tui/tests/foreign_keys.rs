use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Focus, GridCursor, GridMove, Loadable, Modal, ModalMsg, Msg, QueryMsg,
    QueryPane, QueryView, Screen, Tab, TabContent, TableMsg, TableTab, TableView, Workspace,
};
use ducky_core::{
    Catalog, Column, ForeignKeyLink, Page, PageRequest, PgTools, Profile, ProfileKind, Relation,
    RelationKind, RelationRef, ResultSet, Schema, SessionInfo, StoragePaths, ToolStatus,
};
use ducky_tui::keys::handle_key;
use ducky_tui::keys::sequence::{GridPane, KeyPrefix};
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;

fn relation_ref(schema: &str, name: &str) -> RelationRef {
    RelationRef {
        schema: schema.to_owned(),
        name: name.to_owned(),
    }
}

fn column(name: &str) -> Column {
    Column {
        name: name.to_owned(),
        data_type: "integer".to_owned(),
        is_nullable: false,
        default_expression: None,
        is_primary_key: name == "id",
        enum_labels: Vec::new(),
    }
}

fn customer_link(name: &str, column_name: &str) -> ForeignKeyLink {
    ForeignKeyLink {
        name: name.to_owned(),
        columns: vec![column_name.to_owned()],
        referenced: relation_ref("public", "customers"),
        referenced_columns: vec!["id".to_owned()],
    }
}

fn table(name: &str, columns: &[&str], foreign_keys: Vec<ForeignKeyLink>) -> Relation {
    Relation {
        name: name.to_owned(),
        kind: RelationKind::Table,
        estimated_row_count: Some(2),
        columns: columns
            .iter()
            .map(|column_name| column(column_name))
            .collect(),
        foreign_keys,
    }
}

fn orders_view() -> TableView {
    let relation = relation_ref("public", "orders");
    TableView {
        relation: relation.clone(),
        tab: TableTab::Data,
        request: PageRequest {
            relation,
            page_index: 0,
            page_size: 50,
            sort: None,
            filter: None,
        },
        page: Loadable::Ready(Page {
            result_set: ResultSet {
                columns: vec!["id".to_owned(), "customer_id".to_owned()],
                rows: vec![vec![Some("1".to_owned()), Some("42".to_owned())]],
                is_truncated: false,
            },
            has_next_page: false,
        }),
        cursor: GridCursor {
            row_index: 0,
            column_index: 1,
        },
        details: Loadable::Idle,
        marked_row_indexes: Default::default(),
        visual_anchor_row_index: None,
    }
}

fn state_with(content: TabContent) -> AppState {
    let profile = Profile {
        label: "Dev local".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
        project: None,
        tags: Vec::new(),
    };
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    };
    let mut state = AppState::new(vec![profile.clone()], storage);
    let mut workspace = Workspace::new(
        0,
        profile,
        Connection {
            session: SessionInfo {
                database: "ducky".to_owned(),
                user: "ducky".to_owned(),
                server_version: "18.1".to_owned(),
            },
            tools: PgTools {
                pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
                pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
            },
        },
    );
    workspace.catalog = Loadable::Ready(Catalog {
        schemas: vec![Schema {
            name: "public".to_owned(),
            relations: vec![
                table("customers", &["id", "email"], Vec::new()),
                table(
                    "orders",
                    &["id", "customer_id"],
                    vec![customer_link("orders_customer_id_fkey", "customer_id")],
                ),
            ],
        }],
    });
    workspace.schema_name = "public".to_owned();
    workspace.focus = Focus::Main;
    workspace.tabs = vec![Tab { id: 1, content }];
    workspace.next_tab_id = 2;
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn results_state() -> AppState {
    let mut query = QueryView::new(1);
    query.pane = QueryPane::Results;
    state_with(TabContent::Query(query))
}

fn press_all(state: &AppState, tui: &mut TuiState, characters: &str) -> Vec<Msg> {
    characters
        .chars()
        .flat_map(|character| {
            handle_key(
                state,
                tui,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            )
        })
        .collect()
}

fn render(state: &AppState, tui: &mut TuiState) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
        .expect("draw");
    terminal
}

fn screen_rows(state: &AppState, tui: &mut TuiState) -> Vec<String> {
    rows_of(&render(state, tui))
}

fn rows_of(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn g_prefix_maps_gd_gr_and_gg_in_table_data() {
    let state = state_with(TabContent::Table(orders_view()));
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, "gd"),
        vec![Msg::Table(TableMsg::GoToReferenced)]
    );
    assert_eq!(
        press_all(&state, &mut tui, "gr"),
        vec![Msg::Table(TableMsg::FindReferencing)]
    );
    assert_eq!(
        press_all(&state, &mut tui, "gg"),
        vec![Msg::Table(TableMsg::Move(GridMove::FirstRow))]
    );
    assert_eq!(tui.pending_prefix, None);
}

#[test]
fn g_prefix_is_cancelled_by_any_other_key() {
    let state = state_with(TabContent::Table(orders_view()));
    let mut tui = TuiState::new(false);
    assert!(press_all(&state, &mut tui, "g").is_empty());
    assert_eq!(tui.pending_prefix, Some(KeyPrefix::Go(GridPane::TableData)));
    assert!(press_all(&state, &mut tui, "x").is_empty());
    assert_eq!(tui.pending_prefix, None);
    assert!(press_all(&state, &mut tui, "g").is_empty());
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(handle_key(&state, &mut tui, escape).is_empty());
    assert_eq!(tui.pending_prefix, None);
}

#[test]
fn pending_g_prefix_is_shown_in_the_status_bar() {
    let state = state_with(TabContent::Table(orders_view()));
    let mut tui = TuiState::new(false);
    press_all(&state, &mut tui, "g");
    let status_row = screen_rows(&state, &mut tui).pop().unwrap_or_default();
    assert!(status_row.contains("g first row"), "{status_row}");
    assert!(status_row.contains("d referenced row"), "{status_row}");
    assert!(status_row.contains("r referencing rows"), "{status_row}");
}

#[test]
fn g_prefix_in_results_only_goes_to_the_first_row() {
    let state = results_state();
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, "gg"),
        vec![Msg::Query(QueryMsg::Move(GridMove::FirstRow))]
    );
    assert!(press_all(&state, &mut tui, "gd").is_empty());
    assert!(press_all(&state, &mut tui, "gr").is_empty());
    assert_eq!(tui.pending_prefix, None);
}

#[test]
fn pending_g_prefix_in_results_only_hints_the_first_row() {
    let state = results_state();
    let mut tui = TuiState::new(false);
    press_all(&state, &mut tui, "g");
    assert_eq!(tui.pending_prefix, Some(KeyPrefix::Go(GridPane::Results)));
    let status_row = screen_rows(&state, &mut tui).pop().unwrap_or_default();
    assert!(status_row.contains("g first row"), "{status_row}");
    assert!(!status_row.contains("referenced"), "{status_row}");
    assert!(!status_row.contains("referencing"), "{status_row}");
}

#[test]
fn visual_anchor_switches_the_status_hints() {
    let mut view = orders_view();
    view.visual_anchor_row_index = Some(0);
    let state = state_with(TabContent::Table(view));
    let mut tui = TuiState::new(false);
    let status_row = screen_rows(&state, &mut tui).pop().unwrap_or_default();
    assert!(
        status_row.contains("VISUAL  j/k extend · V keep · e edit · Esc clear"),
        "{status_row}"
    );
}

#[test]
fn g_is_not_a_prefix_on_the_structure_tab() {
    let mut view = orders_view();
    view.tab = TableTab::Structure;
    let state = state_with(TabContent::Table(view));
    let mut tui = TuiState::new(false);
    assert!(press_all(&state, &mut tui, "g").is_empty());
    assert_eq!(tui.pending_prefix, None);
}

#[test]
fn foreign_key_columns_get_an_arrow_in_the_grid_header() {
    let state = state_with(TabContent::Table(orders_view()));
    let mut tui = TuiState::new(false);
    let rows = screen_rows(&state, &mut tui);
    let header_row = rows
        .iter()
        .find(|row| row.contains("customer_id →"))
        .unwrap_or_else(|| panic!("no linked header in\n{}", rows.join("\n")));
    assert_eq!(header_row.matches('→').count(), 1, "{header_row}");
}

fn picker_state() -> AppState {
    let mut state = state_with(TabContent::Table(orders_view()));
    state.modal = Some(Modal::ReferencePicker {
        links: vec![
            (
                relation_ref("public", "invoices"),
                customer_link("invoices_customer_id_fkey", "customer_id"),
            ),
            (
                relation_ref("public", "orders"),
                customer_link("orders_customer_id_fkey", "customer_id"),
            ),
        ],
        selected_link_index: 1,
    });
    state
}

#[test]
fn reference_picker_lists_links_and_its_footer() {
    let state = picker_state();
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui);
    let rows = rows_of(&terminal);
    let screen = rows.join("\n");
    assert!(
        screen.contains("invoices.customer_id → customers.id"),
        "{screen}"
    );
    assert!(
        screen.contains("Enter open · j/k move · Esc cancel"),
        "{screen}"
    );
    let selected_row = rows
        .iter()
        .position(|row| row.contains("orders.customer_id → customers.id"))
        .expect("orders link listed");
    let selected_column = rows[selected_row]
        .find("orders.customer_id")
        .map(|byte_offset| rows[selected_row][..byte_offset].chars().count())
        .expect("column");
    let selected_cell = &terminal.backend().buffer()[(
        u16::try_from(selected_column).expect("x"),
        u16::try_from(selected_row).expect("y"),
    )];
    assert_eq!(selected_cell.bg, Color::Green);
}

#[test]
fn reference_picker_keys_move_submit_and_cancel() {
    let state = picker_state();
    let mut tui = TuiState::new(false);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('j'))),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('k'))),
        vec![Msg::Modal(ModalMsg::Previous)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Enter)),
        vec![Msg::Modal(ModalMsg::Submit)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
}

#[test]
fn link_label_qualifies_cross_schema_sources_and_lists_composite_columns() {
    let same_schema = ducky_tui::views::reference_picker::link_label(
        &relation_ref("public", "orders"),
        &customer_link("orders_customer_id_fkey", "customer_id"),
    );
    assert_eq!(same_schema, "orders.customer_id → customers.id");
    let composite = ForeignKeyLink {
        name: "events_order_fkey".to_owned(),
        columns: vec!["order_id".to_owned(), "line_number".to_owned()],
        referenced: relation_ref("public", "order_lines"),
        referenced_columns: vec!["order_id".to_owned(), "number".to_owned()],
    };
    let cross_schema = ducky_tui::views::reference_picker::link_label(
        &relation_ref("analytics", "events"),
        &composite,
    );
    assert_eq!(
        cross_schema,
        "analytics.events.(order_id, line_number) → order_lines.(order_id, number)"
    );
}
