use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    Connection, Focus, GridCursor, Loadable, MainView, Msg, PAGE_SIZE, TableMsg, TableTab,
    TableView, Workspace,
};
use ducky_core::{
    Catalog, Column, ForeignKey, Index, Page, PageRequest, PgTools, Profile, ProfileKind, Relation,
    RelationDetails, RelationKind, RelationRef, ResultSet, Schema, SessionInfo, Sort,
    SortDirection, StoragePaths, ToolStatus,
};
use ducky_tui::keys::table::handle;
use ducky_tui::theme::palette;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::{ViewContext, table};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn column(name: &str, data_type: &str, is_nullable: bool, is_primary_key: bool) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        is_nullable,
        default_expression: None,
        is_primary_key,
    }
}

fn catalog() -> Catalog {
    Catalog {
        schemas: vec![Schema {
            name: "public".to_owned(),
            relations: vec![
                Relation {
                    name: "customers".to_owned(),
                    kind: RelationKind::Table,
                    estimated_row_count: Some(250),
                    columns: vec![
                        column("id", "integer", false, true),
                        column("email", "text", false, false),
                        column("name", "text", true, false),
                    ],
                },
                Relation {
                    name: "order_totals".to_owned(),
                    kind: RelationKind::View,
                    estimated_row_count: None,
                    columns: vec![column("email", "text", true, false)],
                },
            ],
        }],
    }
}

fn relation_ref(name: &str) -> RelationRef {
    RelationRef {
        schema: "public".to_owned(),
        name: name.to_owned(),
    }
}

fn table_view(name: &str) -> TableView {
    let relation = relation_ref(name);
    TableView {
        relation: relation.clone(),
        tab: TableTab::Data,
        request: PageRequest {
            relation,
            page_index: 0,
            page_size: PAGE_SIZE,
            sort: None,
            filter: None,
        },
        page: Loadable::Ready(Page {
            result_set: ResultSet {
                columns: vec!["id".to_owned(), "email".to_owned(), "name".to_owned()],
                rows: vec![vec![
                    Some("1".to_owned()),
                    Some("customer1@example.com".to_owned()),
                    None,
                ]],
                is_truncated: false,
            },
            has_next_page: true,
        }),
        cursor: GridCursor::default(),
        details: Loadable::Idle,
    }
}

fn workspace(table_view: TableView) -> Workspace {
    let mut workspace = Workspace::new(
        0,
        Profile {
            label: "Dev local".to_owned(),
            url: "postgres://ducky@localhost/ducky".to_owned(),
            kind: ProfileKind::Local,
        },
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
    workspace.catalog = Loadable::Ready(catalog());
    workspace.schema_name = "public".to_owned();
    workspace.view = MainView::Table;
    workspace.focus = Focus::Main;
    workspace.table = Some(table_view);
    workspace
}

fn render_text(workspace: &Workspace) -> String {
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/sql"),
        dump_dir: PathBuf::from("/tmp/dumps"),
    };
    let workspace_palette = palette(Some(ProfileKind::Local));
    let context = ViewContext {
        workspace,
        storage: &storage,
        palette: &workspace_palette,
    };
    let mut tui = TuiState::new(false);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
    terminal
        .draw(|frame| table::render(frame, frame.area(), &context, &mut tui))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_contains(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?} in\n{text}");
}

#[test]
fn data_tab_shows_headers_first_row_page_and_sort() {
    let mut view = table_view("customers");
    view.request.sort = Some(Sort {
        column: "name".to_owned(),
        direction: SortDirection::Ascending,
    });
    let text = render_text(&workspace(view));
    for expected in [
        "public.customers",
        "Data",
        "Structure",
        "email",
        "customer1@example.com",
        "NULL",
        "page 1",
        "sort: name ↑",
        "~250 rows",
    ] {
        assert_contains(&text, expected);
    }
}

#[test]
fn structure_tab_shows_columns_indexes_and_foreign_keys() {
    let mut view = table_view("customers");
    view.tab = TableTab::Structure;
    view.details = Loadable::Ready(RelationDetails {
        indexes: vec![Index {
            name: "customers_email_key".to_owned(),
            definition:
                "CREATE UNIQUE INDEX customers_email_key ON public.customers USING btree (email)"
                    .to_owned(),
            is_unique: true,
            is_primary: false,
        }],
        foreign_keys: vec![ForeignKey {
            name: "customers_owner_fkey".to_owned(),
            definition: "FOREIGN KEY (owner_id) REFERENCES owners(id)".to_owned(),
        }],
        view_definition: None,
    });
    let text = render_text(&workspace(view));
    for expected in [
        "Columns",
        "NOT NULL",
        "PK",
        "customers_email_key",
        "UNIQUE",
        "FOREIGN KEY (owner_id) REFERENCES owners(id)",
    ] {
        assert_contains(&text, expected);
    }
}

#[test]
fn structure_tab_shows_view_definition() {
    let mut view = table_view("order_totals");
    view.tab = TableTab::Structure;
    view.details = Loadable::Ready(RelationDetails {
        indexes: Vec::new(),
        foreign_keys: Vec::new(),
        view_definition: Some("SELECT email FROM customers".to_owned()),
    });
    let text = render_text(&workspace(view));
    assert_contains(&text, "Definition");
    assert_contains(&text, "SELECT email FROM customers");
}

#[test]
fn structure_tab_shows_columns_while_details_load() {
    let mut view = table_view("customers");
    view.tab = TableTab::Structure;
    view.details = Loadable::Loading;
    let text = render_text(&workspace(view));
    assert_contains(&text, "Columns");
    assert_contains(&text, "Loading details");
}

#[test]
fn failed_page_shows_the_error_and_hint() {
    let mut view = table_view("customers");
    view.page = Loadable::Failed("ERROR: column \"nope\" does not exist".to_owned());
    let text = render_text(&workspace(view));
    assert_contains(&text, "ERROR: column \"nope\" does not exist");
    assert_contains(&text, "Esc clear filter");
}

fn press(workspace: &Workspace, code: KeyCode) -> Vec<Msg> {
    let mut tui = TuiState::new(false);
    handle(workspace, &mut tui, KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn data_keys_map_to_table_messages() {
    let workspace = workspace(table_view("customers"));
    assert_eq!(
        press(&workspace, KeyCode::Char('n')),
        vec![Msg::Table(TableMsg::NextPage)]
    );
    assert_eq!(
        press(&workspace, KeyCode::Char('o')),
        vec![Msg::Table(TableMsg::CycleSort)]
    );
    assert_eq!(
        press(&workspace, KeyCode::Char('/')),
        vec![Msg::Table(TableMsg::StartFilter)]
    );
    assert_eq!(
        press(&workspace, KeyCode::Enter),
        vec![Msg::Table(TableMsg::OpenRowDetail)]
    );
}

#[test]
fn escape_clears_a_filter_or_toggles_focus() {
    let mut view = table_view("customers");
    assert_eq!(
        press(&workspace(view.clone()), KeyCode::Esc),
        vec![Msg::ToggleFocus]
    );
    view.request.filter = Some("id > 1".to_owned());
    assert_eq!(
        press(&workspace(view), KeyCode::Esc),
        vec![Msg::Table(TableMsg::ClearFilter)]
    );
}

#[test]
fn structure_keys_scroll_locally() {
    let mut view = table_view("customers");
    view.tab = TableTab::Structure;
    let workspace = workspace(view);
    let mut tui = TuiState::new(false);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert!(handle(&workspace, &mut tui, key(KeyCode::Char('j'))).is_empty());
    assert!(handle(&workspace, &mut tui, key(KeyCode::Char('j'))).is_empty());
    assert_eq!(tui.structure_scroll, 2);
    handle(&workspace, &mut tui, key(KeyCode::Char('g')));
    assert_eq!(tui.structure_scroll, 0);
    assert_eq!(
        handle(&workspace, &mut tui, key(KeyCode::Char('t'))),
        vec![Msg::Table(TableMsg::ToggleTab)]
    );
}
