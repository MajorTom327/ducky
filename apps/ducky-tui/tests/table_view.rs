use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    Connection, Focus, GridCursor, Loadable, Msg, RowDetail, Tab, TabContent, TableMsg, TableTab,
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
use ducky_tui::views::{ViewContext, row_detail, table};
use ducky_tui::widgets::grid::{GridOptions, GridViewport, render_grid};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

const TAB_ID: u64 = 7;

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
                    foreign_keys: Vec::new(),
                },
                Relation {
                    name: "order_totals".to_owned(),
                    kind: RelationKind::View,
                    estimated_row_count: None,
                    columns: vec![column("email", "text", true, false)],
                    foreign_keys: Vec::new(),
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
            page_size: 100,
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
        marked_row_indexes: Default::default(),
        visual_anchor_row_index: None,
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
    workspace.focus = Focus::Main;
    workspace.tabs = vec![Tab {
        id: TAB_ID,
        content: TabContent::Table(table_view),
    }];
    workspace.active_tab_index = 0;
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
    buffer_text(terminal.backend().buffer())
}

fn buffer_text(buffer: &Buffer) -> String {
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
        "sort name ↑",
    ] {
        assert_contains(&text, expected);
    }
}

#[test]
fn block_titles_show_tabs_range_sort_filter_and_page() {
    let mut view = table_view("customers");
    view.request.sort = Some(Sort {
        column: "email".to_owned(),
        direction: SortDirection::Descending,
    });
    view.request.filter = Some("id > 0".to_owned());
    let text = render_text(&workspace(view));
    let lines: Vec<&str> = text.lines().collect();
    let first_line = lines.first().copied().unwrap_or_default();
    let last_line = lines.last().copied().unwrap_or_default();
    assert_contains(first_line, " public.customers ");
    assert_contains(first_line, " Data · Structure ");
    assert!(
        first_line.find("public.customers") < first_line.find("Data · Structure"),
        "{first_line}"
    );
    assert_contains(last_line, " rows 1–1 · sort email ↓ · where id > 0 ");
    assert_contains(last_line, " page 1 ");
    assert!(
        last_line.find("rows 1–1") < last_line.find("page 1"),
        "{last_line}"
    );
}

fn people_page(page_index: usize) -> TableView {
    let mut view = table_view("customers");
    view.request.page_index = page_index;
    view.page = Loadable::Ready(Page {
        result_set: ResultSet {
            columns: vec!["id".to_owned(), "email".to_owned()],
            rows: vec![
                vec![Some("101".to_owned()), Some("a@example.com".to_owned())],
                vec![Some("102".to_owned()), Some("b@example.com".to_owned())],
            ],
            is_truncated: false,
        },
        has_next_page: false,
    });
    view
}

#[test]
fn page_two_gutter_starts_at_row_one_hundred_one() {
    let text = render_text(&workspace(people_page(1)));
    assert_contains(&text, "101  101  a@example.com");
    assert_contains(&text, "102  102  b@example.com");
    assert_contains(&text, "rows 101–102");
    assert_contains(&text, "page 2");
}

fn render_small_grid(
    result_set: &ResultSet,
    cursor: GridCursor,
    selected_row_indexes: &[usize],
) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).expect("terminal");
    let grid_palette = palette(Some(ProfileKind::Local));
    terminal
        .draw(|frame| {
            let options = GridOptions {
                cursor,
                sort: None,
                first_row_number: 1,
                is_focused: true,
                linked_columns: &[],
                selected_row_indexes,
            };
            render_grid(
                frame,
                frame.area(),
                result_set,
                &mut GridViewport::default(),
                options,
                &grid_palette,
            );
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

fn numeric_result_set() -> ResultSet {
    ResultSet {
        columns: vec!["id".to_owned(), "name".to_owned()],
        rows: vec![
            vec![Some("1".to_owned()), Some("a".to_owned())],
            vec![Some("10".to_owned()), None],
        ],
        is_truncated: false,
    }
}

#[test]
fn grid_has_header_rule_gutter_and_right_aligned_numbers() {
    let buffer = render_small_grid(&numeric_result_set(), GridCursor::default(), &[]);
    let text = buffer_text(&buffer);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("#  id  name"), "{text}");
    assert!(lines[1].starts_with("─  ──  ────"), "{text}");
    assert!(lines[2].starts_with("1   1  a   "), "{text}");
    assert!(lines[3].starts_with("2  10  NULL"), "{text}");
}

#[test]
fn grid_cursor_row_is_dimmed_and_cursor_cell_selected() {
    let cursor = GridCursor {
        row_index: 1,
        column_index: 1,
    };
    let buffer = render_small_grid(&numeric_result_set(), cursor, &[]);
    assert_eq!(buffer[(3, 3)].bg, Color::DarkGray);
    assert_eq!(buffer[(3, 3)].fg, Color::White);
    assert_eq!(buffer[(7, 3)].bg, Color::Green);
    assert_eq!(buffer[(3, 2)].bg, Color::Reset);
}

#[test]
fn row_detail_fits_its_fields_and_aligns_names() {
    let row_detail = RowDetail {
        columns: vec![
            "id".to_owned(),
            "email".to_owned(),
            "name".to_owned(),
            "created_at".to_owned(),
        ],
        values: vec![
            Some("7".to_owned()),
            Some("x@y.z".to_owned()),
            None,
            Some("2024".to_owned()),
        ],
        selected_field_index: 1,
    };
    let workspace_palette = palette(Some(ProfileKind::Local));
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
    let mut popup_area = Rect::default();
    terminal
        .draw(|frame| {
            popup_area = row_detail::render(frame, frame.area(), &row_detail, &workspace_palette);
        })
        .expect("draw");
    assert_eq!(popup_area.height, 6);
    assert_eq!(popup_area.width, 70);
    let text = buffer_text(terminal.backend().buffer());
    let lines: Vec<&str> = text.lines().collect();
    let name_width = "created_at".len();
    let value_column = usize::from(popup_area.x) + 2 + name_width + 2;
    for (field_offset, (name, value)) in [
        ("id", "7"),
        ("email", "x@y.z"),
        ("name", "NULL"),
        ("created_at", "2024"),
    ]
    .into_iter()
    .enumerate()
    {
        let characters: Vec<char> = lines[usize::from(popup_area.y) + 1 + field_offset]
            .chars()
            .collect();
        let value_text: String = characters[value_column..value_column + value.len()]
            .iter()
            .collect();
        let name_text: String = characters[value_column - 2 - name.len()..value_column - 2]
            .iter()
            .collect();
        assert_eq!(value_text, value);
        assert_eq!(name_text, name);
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
        "Indexes",
        "customers_email_key",
        "UNIQUE",
        "Foreign keys",
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
fn selection_keys_map_to_table_messages() {
    let view = table_view("customers");
    assert_eq!(
        press(&workspace(view.clone()), KeyCode::Char('v')),
        vec![Msg::Table(TableMsg::ToggleRowMark)]
    );
    assert_eq!(
        press(&workspace(view.clone()), KeyCode::Char('V')),
        vec![Msg::Table(TableMsg::ToggleVisualSelection)]
    );
    assert_eq!(
        press(&workspace(view), KeyCode::Char('e')),
        vec![Msg::Table(TableMsg::EditSelectedRows)]
    );
}

#[test]
fn escape_clears_the_selection_before_the_filter() {
    let mut view = table_view("customers");
    view.request.filter = Some("id > 1".to_owned());
    view.marked_row_indexes.insert(0);
    assert_eq!(
        press(&workspace(view.clone()), KeyCode::Esc),
        vec![Msg::Table(TableMsg::ClearSelection)]
    );
    view.marked_row_indexes.clear();
    view.visual_anchor_row_index = Some(0);
    assert_eq!(
        press(&workspace(view), KeyCode::Esc),
        vec![Msg::Table(TableMsg::ClearSelection)]
    );
}

#[test]
fn selected_rows_get_a_marker_and_the_border_counts_them() {
    let mut view = people_page(0);
    view.marked_row_indexes.insert(1);
    let text = render_text(&workspace(view));
    let lines: Vec<&str> = text.lines().collect();
    let first_row = lines
        .iter()
        .find(|line| line.contains("a@example.com"))
        .copied()
        .unwrap_or_default();
    let second_row = lines
        .iter()
        .find(|line| line.contains("b@example.com"))
        .copied()
        .unwrap_or_default();
    assert!(!first_row.contains('▌'), "{first_row}");
    assert!(
        second_row.contains("▌2  102  b@example.com"),
        "{second_row}"
    );
    let last_line = lines.last().copied().unwrap_or_default();
    assert_contains(last_line, "rows 1–2 · 1 selected");
}

#[test]
fn grid_marker_uses_the_title_style_on_selected_rows_only() {
    let buffer = render_small_grid(&numeric_result_set(), GridCursor::default(), &[1]);
    let text = buffer_text(&buffer);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with(" #  id  name"), "{text}");
    assert!(lines[2].starts_with(" 1   1  a   "), "{text}");
    assert!(lines[3].starts_with("▌2  10  NULL"), "{text}");
    assert_eq!(buffer[(0, 3)].fg, Color::Green);
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
    assert!(handle(&workspace, &mut tui, key(KeyCode::Char('j'))).is_empty());
    assert_eq!(tui.structure_scrolls.get(&TAB_ID), Some(&3));
    assert!(handle(&workspace, &mut tui, key(KeyCode::Char('k'))).is_empty());
    assert_eq!(tui.structure_scrolls.get(&TAB_ID), Some(&2));
    handle(&workspace, &mut tui, key(KeyCode::Char('g')));
    assert_eq!(tui.structure_scrolls.get(&TAB_ID), Some(&0));
    assert_eq!(
        handle(&workspace, &mut tui, key(KeyCode::Esc)),
        vec![Msg::ToggleFocus]
    );
    assert_eq!(
        handle(&workspace, &mut tui, key(KeyCode::Char('t'))),
        vec![Msg::Table(TableMsg::ToggleTab)]
    );
}
