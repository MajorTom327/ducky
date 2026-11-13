use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, GridCursor, Loadable, Modal, ModalMsg, Msg, Notice, NoticeLevel,
    ProfilesMsg, Screen, SidebarMsg, Workspace,
};
use ducky_core::{
    Catalog, Column, PgTools, Profile, ProfileKind, Relation, RelationKind, ResultSet, Schema,
    SessionInfo, StoragePaths, ToolStatus,
};
use ducky_sql::TokenKind;
use ducky_tui::keys::handle_key;
use ducky_tui::theme::{palette, token_style};
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ducky_tui::views::sidebar::compact_count;
use ducky_tui::widgets::grid::{GridOptions, GridViewport, render_grid};
use ducky_tui::widgets::sql_text::highlighted_sql;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;

fn profile(label: &str, kind: ProfileKind) -> Profile {
    Profile {
        label: label.to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind,
        project: None,
        tags: Vec::new(),
    }
}

fn storage() -> StoragePaths {
    StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    }
}

fn available_tools() -> PgTools {
    PgTools {
        pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
        pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
    }
}

fn relation(name: &str, kind: RelationKind) -> Relation {
    Relation {
        name: name.to_owned(),
        kind,
        estimated_row_count: Some(250),
        columns: vec![Column {
            name: "id".to_owned(),
            data_type: "integer".to_owned(),
            is_nullable: false,
            default_expression: None,
            is_primary_key: true,
        }],
        foreign_keys: Vec::new(),
    }
}

fn catalog() -> Catalog {
    Catalog {
        schemas: vec![
            Schema {
                name: "analytics".to_owned(),
                relations: vec![relation("events", RelationKind::Table)],
            },
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    relation("orders", RelationKind::Table),
                    relation("customers", RelationKind::Table),
                    relation("order_totals", RelationKind::View),
                ],
            },
        ],
    }
}

fn workspace_state(kind: ProfileKind, tools: PgTools) -> AppState {
    let selected_profile = profile("Dev local", kind);
    let mut state = AppState::new(vec![selected_profile.clone()], storage());
    let mut workspace = Workspace::new(
        0,
        selected_profile,
        Connection {
            session: SessionInfo {
                database: "ducky".to_owned(),
                user: "ducky".to_owned(),
                server_version: "18.1".to_owned(),
            },
            tools,
        },
    );
    workspace.catalog = Loadable::Ready(catalog());
    workspace.schema_name = "public".to_owned();
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

fn render(state: &AppState, tui: &mut TuiState, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
        .expect("draw");
    terminal
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
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

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    screen_text(terminal)
        .lines()
        .nth(usize::from(y))
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn header_background_follows_the_profile_kind() {
    for (kind, expected_color) in [
        (ProfileKind::Local, Color::Green),
        (ProfileKind::Staging, Color::Indexed(208)),
        (ProfileKind::Prod, Color::Red),
    ] {
        let state = workspace_state(kind, available_tools());
        let mut tui = TuiState::new(false);
        let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].bg, expected_color);
        assert_eq!(buffer[(WIDTH - 1, 0)].bg, expected_color);
        assert!(row_text(&terminal, 0).contains("Dev local"));
    }
}

#[test]
fn profiles_screen_lists_every_label() {
    let state = AppState::new(
        vec![
            profile("Dev local", ProfileKind::Local),
            profile("Shared staging", ProfileKind::Staging),
            profile("Production", ProfileKind::Prod),
        ],
        storage(),
    );
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
    let text = screen_text(&terminal);
    assert!(text.contains("Profiles"));
    assert!(text.contains("Connect"));
    assert!(text.contains("LOCAL   Dev local"));
    assert!(text.contains("STAGING Shared staging"));
    assert!(text.contains("PROD    Production"));
    assert_eq!(terminal.backend().buffer()[(0, 0)].bg, palette(None).accent);
}

#[test]
fn sidebar_lists_relations_of_the_selected_schema_with_their_meta() {
    let state = workspace_state(ProfileKind::Local, available_tools());
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
    let text = screen_text(&terminal);
    assert!(text.contains(" public "));
    let customers_row = (0..HEIGHT)
        .map(|y| row_text(&terminal, y))
        .find(|row| row.contains("customers"))
        .expect("customers row");
    let sidebar_part: String = customers_row.chars().take(32).collect();
    assert!(sidebar_part.trim_end().ends_with("250 │"), "{sidebar_part}");
    let view_row = (0..HEIGHT)
        .map(|y| row_text(&terminal, y))
        .find(|row| row.contains("order_totals"))
        .expect("view row");
    assert!(
        view_row
            .chars()
            .take(32)
            .collect::<String>()
            .contains("view │")
    );
    for absent in [
        "events",
        "SQL command",
        "Dump database",
        "Restore dump",
        "Tables & views",
    ] {
        assert!(!text.contains(absent), "unexpected {absent}");
    }
}

#[test]
fn empty_schema_shows_no_tables() {
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    workspace_mut(&mut state).sidebar.filter = "zzz".to_owned();
    let mut tui = TuiState::new(false);
    let text = screen_text(&render(&state, &mut tui, WIDTH, HEIGHT));
    assert!(text.contains("No tables"));
    assert!(text.contains("/zzz"));
}

#[test]
fn compact_counts_use_k_and_m_suffixes() {
    assert_eq!(compact_count(250), "250");
    assert_eq!(compact_count(1_234), "1.2k");
    assert_eq!(compact_count(250_000), "250k");
    assert_eq!(compact_count(3_400_000), "3.4M");
    assert_eq!(compact_count(5_600_000_000), "5.6G");
    assert_eq!(compact_count(-1), "0");
}

#[test]
fn sidebar_labels_mviews_foreign_tables_and_truncates_long_names() {
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    let workspace = workspace_mut(&mut state);
    let Loadable::Ready(catalog) = &mut workspace.catalog else {
        panic!("expected a ready catalog");
    };
    let public_relations = &mut catalog.schemas[1].relations;
    public_relations.push(relation("daily_rollup", RelationKind::MaterializedView));
    public_relations.push(relation("remote_events", RelationKind::ForeignTable));
    public_relations.push(relation(
        "a_relation_name_far_too_long_for_the_sidebar",
        RelationKind::Table,
    ));
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
    let sidebar_rows: Vec<String> = (0..HEIGHT)
        .map(|y| row_text(&terminal, y).chars().take(32).collect())
        .collect();
    let row_with = |needle: &str| {
        sidebar_rows
            .iter()
            .find(|row| row.contains(needle))
            .cloned()
            .unwrap_or_else(|| panic!("no sidebar row with {needle}"))
    };
    assert!(row_with("daily_rollup").contains("mview │"));
    assert!(row_with("remote_events").contains("foreign │"));
    let long_row = row_with("a_relation_name");
    assert!(long_row.contains("…"));
    assert!(long_row.contains("250 │"));
    assert!(!long_row.contains("sidebar"));
}

fn render_result_set(
    result_set: &ResultSet,
    cursor: GridCursor,
    viewport: &mut GridViewport,
) -> String {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, 10)).expect("terminal");
    let grid_palette = palette(Some(ProfileKind::Local));
    terminal
        .draw(|frame| {
            render_grid(
                frame,
                frame.area(),
                result_set,
                viewport,
                GridOptions {
                    cursor,
                    sort: None,
                    first_row_number: 1,
                    is_focused: true,
                    linked_columns: &[],
                    selected_row_indexes: &[],
                },
                &grid_palette,
            )
        })
        .expect("draw");
    screen_text(&terminal)
}

#[test]
fn grid_renders_null_and_lets_long_cells_use_the_spare_width() {
    let long_value = "x".repeat(300);
    let result_set = ResultSet {
        columns: vec!["value".to_owned(), "note".to_owned()],
        rows: vec![vec![Some(long_value), None]],
        is_truncated: false,
    };
    let text = render_result_set(
        &result_set,
        GridCursor::default(),
        &mut GridViewport::default(),
    );
    assert!(text.contains("NULL"));
    assert!(text.contains("x…"));
    assert!(text.contains(&"x".repeat(80)));
    assert!(!text.contains(&"x".repeat(usize::from(WIDTH))));
}

#[test]
fn grid_caps_long_cells_when_columns_fill_the_width() {
    let columns: Vec<String> = (0..4).map(|index| format!("column_{index}")).collect();
    let row = columns.iter().map(|_| Some("y".repeat(60))).collect();
    let result_set = ResultSet {
        columns,
        rows: vec![row],
        is_truncated: false,
    };
    let text = render_result_set(
        &result_set,
        GridCursor::default(),
        &mut GridViewport::default(),
    );
    assert!(text.contains("y…"));
    assert!(!text.contains(&"y".repeat(41)));
}

#[test]
fn grid_scrolls_to_keep_the_cursor_column_visible() {
    let columns: Vec<String> = (0..20).map(|index| format!("column_{index:02}")).collect();
    let row = columns.iter().map(|_| Some("v".to_owned())).collect();
    let result_set = ResultSet {
        columns,
        rows: vec![row],
        is_truncated: false,
    };
    let mut viewport = GridViewport::default();
    let cursor = GridCursor {
        row_index: 0,
        column_index: 12,
    };
    let text = render_result_set(&result_set, cursor, &mut viewport);
    assert!(text.contains("column_12"));
    assert!(viewport.first_column_index > 0);
}

#[test]
fn grid_shows_headers_and_no_rows_for_an_empty_result() {
    let result_set = ResultSet {
        columns: vec!["id".to_owned()],
        rows: Vec::new(),
        is_truncated: false,
    };
    let text = render_result_set(
        &result_set,
        GridCursor::default(),
        &mut GridViewport::default(),
    );
    assert!(text.contains("id"));
    assert!(text.contains("No rows"));
}

#[test]
fn small_terminal_shows_the_minimum_size_message() {
    let state = workspace_state(ProfileKind::Local, available_tools());
    let mut tui = TuiState::new(false);
    let text = screen_text(&render(&state, &mut tui, 60, 10));
    assert!(text.contains("Terminal too small (minimum 80x20)"));
}

#[test]
fn loading_pulse_stops_once_the_catalog_is_ready() {
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    workspace_mut(&mut state).catalog = Loadable::Loading;
    let mut tui = TuiState::new(true);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
    for _ in 0..3 {
        terminal
            .draw(|frame| draw(frame, &state, &mut tui))
            .expect("draw");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(tui.effects.is_running());
    workspace_mut(&mut state).catalog = Loadable::Ready(catalog());
    let started_at = Instant::now();
    while tui.effects.is_running() && started_at.elapsed() < Duration::from_secs(1) {
        terminal
            .draw(|frame| draw(frame, &state, &mut tui))
            .expect("draw");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!tui.effects.is_running());
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

#[test]
fn ctrl_q_quits_from_every_context() {
    let mut tui = TuiState::new(false);
    let profiles_state = AppState::new(vec![profile("Dev local", ProfileKind::Local)], storage());
    assert_eq!(
        handle_key(&profiles_state, &mut tui, ctrl('q')),
        vec![Msg::Quit]
    );
    let mut modal_state = workspace_state(ProfileKind::Local, available_tools());
    modal_state.modal = Some(Modal::Help);
    assert_eq!(
        handle_key(&modal_state, &mut tui, ctrl('q')),
        vec![Msg::Quit]
    );
}

#[test]
fn open_modal_takes_keys_before_the_sidebar() {
    let mut tui = TuiState::new(false);
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('j'))),
        vec![Msg::Sidebar(SidebarMsg::Next)]
    );
    state.modal = Some(Modal::SchemaPicker {
        selected_schema_index: 0,
    });
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('j'))),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
}

#[test]
fn shown_notice_is_dismissed_before_the_key_message() {
    let mut tui = TuiState::new(false);
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    state.notice = Some(Notice {
        level: NoticeLevel::Info,
        text: "Copied".to_owned(),
    });
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('j'))),
        vec![Msg::DismissNotice, Msg::Sidebar(SidebarMsg::Next)]
    );
}

#[test]
fn profiles_keys_select_and_connect() {
    let mut tui = TuiState::new(false);
    let state = AppState::new(vec![profile("Dev local", ProfileKind::Local)], storage());
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Down)),
        vec![Msg::Profiles(ProfilesMsg::Next)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Enter)),
        vec![Msg::Profiles(ProfilesMsg::Connect)]
    );
}

#[test]
fn sidebar_escape_clears_only_an_active_filter() {
    let mut tui = TuiState::new(false);
    let mut state = workspace_state(ProfileKind::Local, available_tools());
    assert!(handle_key(&state, &mut tui, key(KeyCode::Esc)).is_empty());
    workspace_mut(&mut state).sidebar.filter = "ord".to_owned();
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        vec![Msg::Sidebar(SidebarMsg::ClearFilter)]
    );
}

fn numbered_rows(row_count: usize) -> ResultSet {
    ResultSet {
        columns: vec!["value".to_owned()],
        rows: (1..=row_count)
            .map(|index| vec![Some(format!("value_{index:02}"))])
            .collect(),
        is_truncated: false,
    }
}

#[test]
fn grid_marks_truncated_results() {
    let mut result_set = numbered_rows(3);
    result_set.is_truncated = true;
    let text = render_result_set(
        &result_set,
        GridCursor::default(),
        &mut GridViewport::default(),
    );
    assert!(text.contains("Showing first 3 rows"));
}

#[test]
fn grid_scrolls_rows_down_and_back_up_with_the_cursor() {
    let result_set = numbered_rows(30);
    let mut viewport = GridViewport::default();
    let text = render_result_set(
        &result_set,
        GridCursor {
            row_index: 25,
            column_index: 0,
        },
        &mut viewport,
    );
    assert!(text.contains("value_26"));
    assert!(!text.contains("value_01"));
    assert!(viewport.first_row_index > 0);
    let text = render_result_set(&result_set, GridCursor::default(), &mut viewport);
    assert_eq!(viewport.first_row_index, 0);
    assert!(text.contains("value_01"));
    assert!(!text.contains("value_26"));
}

#[test]
fn grid_scrolls_columns_back_left_with_the_cursor() {
    let columns: Vec<String> = (0..20).map(|index| format!("column_{index:02}")).collect();
    let row = columns.iter().map(|_| Some("v".to_owned())).collect();
    let result_set = ResultSet {
        columns,
        rows: vec![row],
        is_truncated: false,
    };
    let mut viewport = GridViewport {
        first_row_index: 0,
        first_column_index: 15,
    };
    let text = render_result_set(
        &result_set,
        GridCursor {
            row_index: 0,
            column_index: 2,
        },
        &mut viewport,
    );
    assert_eq!(viewport.first_column_index, 2);
    assert!(text.contains("column_02"));
    assert!(!text.contains("column_01"));
}

#[test]
fn grid_in_an_empty_area_draws_nothing_and_keeps_the_viewport() {
    let mut terminal = Terminal::new(TestBackend::new(20, 4)).expect("terminal");
    let grid_palette = palette(None);
    let mut viewport = GridViewport {
        first_row_index: 7,
        first_column_index: 3,
    };
    terminal
        .draw(|frame| {
            render_grid(
                frame,
                Rect::new(0, 0, 20, 0),
                &numbered_rows(30),
                &mut viewport,
                GridOptions {
                    cursor: GridCursor::default(),
                    sort: None,
                    first_row_number: 1,
                    is_focused: true,
                    linked_columns: &[],
                    selected_row_indexes: &[],
                },
                &grid_palette,
            )
        })
        .expect("draw");
    assert!(screen_text(&terminal).trim().is_empty());
    assert_eq!(
        viewport,
        GridViewport {
            first_row_index: 7,
            first_column_index: 3,
        }
    );
}

#[test]
fn highlighted_sql_splits_multi_line_spans_and_keeps_trailing_text() {
    let text = highlighted_sql("select /* first\nsecond */ id  ");
    assert_eq!(text.lines.len(), 2);
    let comment_style = token_style(TokenKind::Comment);
    let first_line = &text.lines[0];
    assert_eq!(first_line.spans[0].content, "select");
    assert_eq!(first_line.spans[0].style, token_style(TokenKind::Keyword));
    let first_comment = first_line.spans.last().expect("comment span");
    assert_eq!(first_comment.content, "/* first");
    assert_eq!(first_comment.style, comment_style);
    let second_line = &text.lines[1];
    assert_eq!(second_line.spans[0].content, "second */");
    assert_eq!(second_line.spans[0].style, comment_style);
    let rendered: String = second_line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(rendered, "second */ id  ");
    let trailing = second_line.spans.last().expect("trailing span");
    assert_eq!(trailing.content, "  ");
    assert_eq!(trailing.style, ratatui::style::Style::default());
}
