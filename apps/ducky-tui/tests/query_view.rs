use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Document, Focus, GridMove, Loadable, Msg, PaneDirection, QueryMsg,
    QueryPane, QueryView, RunState, Screen, Tab, TabContent, TabId, Workspace,
};
use ducky_core::{
    Catalog, Column, PgTools, Profile, ProfileKind, Relation, RelationKind, ResultSet, Schema,
    ScriptFailure, ScriptReport, SessionInfo, StatementOutcome, StatementResult, StoragePaths,
    ToolStatus, TransactionMode,
};
use ducky_sql::TokenKind;
use ducky_tui::keys::query::{handle_editor, handle_results};
use ducky_tui::theme::token_style;
use ducky_tui::tui_state::{CursorShape, TuiState};
use ducky_tui::views::draw;
use edtui::{Highlight, Index2};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn column(name: &str, data_type: &str) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        is_nullable: true,
        default_expression: None,
        is_primary_key: false,
    }
}

fn table(name: &str, columns: Vec<Column>) -> Relation {
    Relation {
        name: name.to_owned(),
        kind: RelationKind::Table,
        estimated_row_count: None,
        columns,
        foreign_keys: Vec::new(),
    }
}

fn catalog() -> Catalog {
    Catalog {
        schemas: vec![
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    table(
                        "customers",
                        vec![
                            column("id", "integer"),
                            column("email", "text"),
                            column("name", "text"),
                        ],
                    ),
                    table(
                        "orders",
                        vec![
                            column("id", "integer"),
                            column("customer_id", "integer"),
                            column("status", "text"),
                        ],
                    ),
                ],
            },
            Schema {
                name: "analytics".to_owned(),
                relations: vec![table(
                    "events",
                    vec![column("id", "bigint"), column("name", "text")],
                )],
            },
        ],
    }
}

const FIRST_TAB_ID: TabId = 1;
const SECOND_TAB_ID: TabId = 2;

fn query_state() -> AppState {
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
    workspace.catalog = Loadable::Ready(catalog());
    workspace.schema_name = "public".to_owned();
    workspace.tabs = vec![
        Tab {
            id: FIRST_TAB_ID,
            content: TabContent::Query(QueryView::new(1)),
        },
        Tab {
            id: SECOND_TAB_ID,
            content: TabContent::Query(QueryView::new(2)),
        },
    ];
    workspace.active_tab_index = 0;
    workspace.next_tab_id = 3;
    workspace.next_query_number = 3;
    workspace.focus = Focus::Main;
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn workspace(state: &AppState) -> &Workspace {
    state.workspace().expect("workspace")
}

fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

fn active_query_mut(state: &mut AppState) -> &mut QueryView {
    let workspace = workspace_mut(state);
    let active_tab_index = workspace.active_tab_index;
    match &mut workspace.tabs[active_tab_index].content {
        TabContent::Query(query) => query,
        _ => panic!("expected a query tab"),
    }
}

fn press(state: &AppState, tui: &mut TuiState, code: KeyCode, modifiers: KeyModifiers) -> Vec<Msg> {
    handle_editor(workspace(state), tui, KeyEvent::new(code, modifiers))
}

fn press_in_results(
    state: &AppState,
    tui: &mut TuiState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Vec<Msg> {
    handle_results(workspace(state), tui, KeyEvent::new(code, modifiers))
}

fn type_in_insert_mode(state: &AppState, tui: &mut TuiState, text: &str) {
    press(state, tui, KeyCode::Char('i'), KeyModifiers::NONE);
    for character in text.chars() {
        press(state, tui, KeyCode::Char(character), KeyModifiers::NONE);
    }
}

fn screen_text(state: &AppState, tui: &mut TuiState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
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

fn rows_result(sql: &str, value: &str) -> StatementResult {
    StatementResult {
        sql: sql.to_owned(),
        outcome: StatementOutcome::Rows(ResultSet {
            columns: vec!["value".to_owned()],
            rows: vec![vec![Some(value.to_owned())]],
            is_truncated: false,
        }),
        elapsed: Duration::from_millis(2),
    }
}

#[test]
fn typing_in_insert_mode_updates_text_and_highlights() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select 1");
    let editor = tui.editor(FIRST_TAB_ID);
    assert_eq!(editor.text(), "select 1");
    let keyword_highlight = Highlight::new(
        Index2::new(0, 0),
        Index2::new(0, 5),
        token_style(TokenKind::Keyword),
    );
    assert!(editor.state.highlights.contains(&keyword_highlight));
}

#[test]
fn each_query_tab_keeps_its_own_buffer() {
    let mut state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select 1");
    workspace_mut(&mut state).active_tab_index = 1;
    type_in_insert_mode(&state, &mut tui, "select 2");
    assert_eq!(tui.editor(FIRST_TAB_ID).text(), "select 1");
    assert_eq!(tui.editor(SECOND_TAB_ID).text(), "select 2");
}

#[test]
fn completion_popup_opens_and_tab_accepts() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from cu");
    let popup = tui
        .editor(FIRST_TAB_ID)
        .completion
        .as_ref()
        .expect("completion popup");
    assert_eq!(popup.completion.items[0].label, "customers");
    let messages = press(&state, &mut tui, KeyCode::Tab, KeyModifiers::NONE);
    assert!(messages.is_empty());
    assert_eq!(tui.editor(FIRST_TAB_ID).text(), "select * from customers");
    assert!(tui.editor(FIRST_TAB_ID).completion.is_none());
}

#[test]
fn escape_closes_the_popup_and_stays_in_insert_mode() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from cu");
    press(&state, &mut tui, KeyCode::Esc, KeyModifiers::NONE);
    assert!(tui.editor(FIRST_TAB_ID).completion.is_none());
    assert!(tui.editor(FIRST_TAB_ID).is_insert_mode());
}

#[test]
fn f5_runs_the_buffer_text() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select 2");
    let messages = press(&state, &mut tui, KeyCode::F(5), KeyModifiers::NONE);
    assert_eq!(
        messages,
        vec![Msg::Query(QueryMsg::Run {
            sql: "select 2".to_owned()
        })]
    );
}

#[test]
fn results_pane_runs_and_saves_the_tab_text() {
    let mut state = query_state();
    let mut tui = TuiState::new(false);
    workspace_mut(&mut state).active_tab_index = 1;
    type_in_insert_mode(&state, &mut tui, "select 7");
    active_query_mut(&mut state).pane = QueryPane::Results;
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::F(5), KeyModifiers::NONE),
        vec![Msg::Query(QueryMsg::Run {
            sql: "select 7".to_owned()
        })]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Char('e'), KeyModifiers::CONTROL),
        vec![Msg::Query(QueryMsg::Run {
            sql: "select 7".to_owned()
        })]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Char('s'), KeyModifiers::CONTROL),
        vec![Msg::Query(QueryMsg::Save {
            content: "select 7".to_owned()
        })]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::F(2), KeyModifiers::NONE),
        vec![Msg::Query(QueryMsg::SaveAs {
            content: "select 7".to_owned()
        })]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Char('t'), KeyModifiers::CONTROL),
        vec![Msg::Query(QueryMsg::ToggleTransactionMode)]
    );
}

#[test]
fn tab_in_normal_mode_toggles_focus() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    let messages = press(&state, &mut tui, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(messages, vec![Msg::ToggleFocus]);
}

#[test]
fn sync_document_replaces_the_buffer_on_a_new_revision() {
    let mut tui = TuiState::new(false);
    let editor = tui.editor(FIRST_TAB_ID);
    editor.sync_document(&Document {
        name: None,
        text: "select 1".to_owned(),
        revision: 1,
    });
    assert_eq!(editor.text(), "select 1");
    editor.sync_document(&Document {
        name: Some("report".to_owned()),
        text: "select 2".to_owned(),
        revision: 1,
    });
    assert_eq!(editor.text(), "select 1");
    editor.sync_document(&Document {
        name: Some("report".to_owned()),
        text: "select 3".to_owned(),
        revision: 2,
    });
    assert_eq!(editor.text(), "select 3");
}

#[test]
fn a_freshly_loaded_document_is_not_modified() {
    let mut tui = TuiState::new(false);
    let document = Document {
        name: Some("report.sql".to_owned()),
        text: "select 1;\r\nselect 2;\n".to_owned(),
        revision: 1,
    };
    let editor = tui.editor(FIRST_TAB_ID);
    editor.sync_document(&document);
    assert!(!editor.is_modified(&document));
}

#[test]
fn editor_title_shows_name_marker_mode_and_transaction() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    let untouched = screen_text(&state, &mut tui);
    assert!(untouched.contains(" Query 1 "));
    assert!(!untouched.contains("Query 1 •"));
    let bottom_border = untouched
        .lines()
        .find(|line| line.contains(" NORMAL "))
        .expect("the mode badge is on the bottom border");
    let mode_position = bottom_border.find(" NORMAL ").unwrap_or(usize::MAX);
    let transaction_position = bottom_border
        .find(" one transaction · Ctrl+t ")
        .expect("the transaction mode is on the bottom border");
    assert!(mode_position < transaction_position);
    type_in_insert_mode(&state, &mut tui, "select 1");
    let modified = screen_text(&state, &mut tui);
    assert!(modified.contains("Query 1 •"));
    assert!(modified.contains(" INSERT "));
}

#[test]
fn the_terminal_cursor_is_a_bar_in_insert_mode_and_a_block_otherwise() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    screen_text(&state, &mut tui);
    assert_eq!(tui.cursor_shape, CursorShape::Block);
    type_in_insert_mode(&state, &mut tui, "select 1");
    screen_text(&state, &mut tui);
    assert_eq!(tui.cursor_shape, CursorShape::Bar);
}

#[test]
fn results_keys_map_to_query_messages() {
    let mut state = query_state();
    active_query_mut(&mut state).pane = QueryPane::Results;
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Char('j'), KeyModifiers::NONE),
        vec![Msg::Query(QueryMsg::Move(GridMove::Down))]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Char('n'), KeyModifiers::NONE),
        vec![Msg::Query(QueryMsg::NextResult)]
    );
    assert_eq!(
        press_in_results(&state, &mut tui, KeyCode::Esc, KeyModifiers::NONE),
        vec![Msg::ToggleFocus]
    );
}

#[test]
fn results_view_renders_a_failure_report() {
    let mut state = query_state();
    active_query_mut(&mut state).run = RunState::Finished(ScriptReport {
        mode: TransactionMode::Transaction,
        results: vec![rows_result("select 1", "1")],
        failure: Some(ScriptFailure {
            statement_index: 1,
            sql: "select nope".to_owned(),
            message: "ERROR: column \"nope\" does not exist".to_owned(),
        }),
        elapsed: Duration::from_millis(5),
    });
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(text.contains("Statement 2 failed"));
    assert!(text.contains("column \"nope\" does not exist"));
    assert!(text.contains("Transaction rolled back"));
}

#[test]
fn results_view_renders_statement_tabs_and_timing() {
    let mut state = query_state();
    let query = active_query_mut(&mut state);
    query.run = RunState::Finished(ScriptReport {
        mode: TransactionMode::AutoCommit,
        results: vec![
            rows_result("select 'first'", "first"),
            rows_result("select 'second'", "second"),
        ],
        failure: None,
        elapsed: Duration::from_millis(7),
    });
    query.selected_result_index = 1;
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(text.contains(" 1 select 'first' 1 row "));
    assert!(text.contains(" 2 select 'second' 1 row "));
    assert!(text.contains(" 7 ms · autocommit "));
    assert!(text.contains("second"));
}

#[test]
fn results_bottom_title_names_the_profile_it_ran_on() {
    let mut state = query_state();
    let query = active_query_mut(&mut state);
    query.run = RunState::Finished(ScriptReport {
        mode: TransactionMode::Transaction,
        results: vec![rows_result("select 1", "1")],
        failure: None,
        elapsed: Duration::from_millis(5),
    });
    query.ran_on_profile_label = Some("Dev local".to_owned());
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(
        text.contains(" 5 ms · one transaction · on Dev local "),
        "{text}"
    );
}

#[test]
fn results_view_shows_the_affected_count() {
    let mut state = query_state();
    active_query_mut(&mut state).run = RunState::Finished(ScriptReport {
        mode: TransactionMode::Transaction,
        results: vec![StatementResult {
            sql: "delete from orders".to_owned(),
            outcome: StatementOutcome::Affected(2),
            elapsed: Duration::from_millis(3),
        }],
        failure: None,
        elapsed: Duration::from_millis(3),
    });
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(text.contains(" 1 delete from orders 2 rows affected "));
    assert_eq!(text.matches(" 2 rows affected ").count(), 2);
}

#[test]
fn ctrl_hjkl_move_the_focus_even_while_typing_in_the_editor() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select");
    for (character, direction) in [
        ('h', PaneDirection::Left),
        ('j', PaneDirection::Down),
        ('k', PaneDirection::Up),
        ('l', PaneDirection::Right),
    ] {
        let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL);
        assert_eq!(
            ducky_tui::keys::handle_key(&state, &mut tui, key),
            vec![Msg::FocusPane(direction)]
        );
    }
    assert_eq!(tui.editor(FIRST_TAB_ID).text(), "select");
}

#[test]
fn editor_and_results_keys_ignore_non_query_tabs() {
    let mut state = query_state();
    let mut tui = TuiState::new(false);
    workspace_mut(&mut state).tabs[0].content = TabContent::Files;
    let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
    assert!(handle_editor(workspace(&state), &mut tui, key).is_empty());
    assert!(handle_results(workspace(&state), &mut tui, key).is_empty());
    assert!(
        ducky_tui::keys::query::handle_paste(workspace(&state), &mut tui, "x".to_owned())
            .is_empty()
    );
}

#[test]
fn paste_inserts_text_and_closes_completion() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from cu");
    assert!(tui.editor(FIRST_TAB_ID).completion.is_some());
    let messages =
        ducky_tui::keys::query::handle_paste(workspace(&state), &mut tui, "stomers".to_owned());
    assert!(messages.is_empty());
    let editor = tui.editor(FIRST_TAB_ID);
    assert!(editor.completion.is_none());
    assert!(editor.text().contains("stomers"));
}

#[test]
fn ctrl_space_forces_completion_and_arrows_move_the_selection() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from ");
    tui.editor(FIRST_TAB_ID).completion = None;
    let messages = press(&state, &mut tui, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert!(messages.is_empty());
    let popup = tui.editor(FIRST_TAB_ID).completion.as_ref().expect("popup");
    assert!(popup.completion.items.len() > 1);
    assert_eq!(popup.selected_item_index, 0);
    press(&state, &mut tui, KeyCode::Down, KeyModifiers::NONE);
    let selected = |tui: &mut TuiState| {
        tui.editor(FIRST_TAB_ID)
            .completion
            .as_ref()
            .expect("popup")
            .selected_item_index
    };
    assert_eq!(selected(&mut tui), 1);
    press(&state, &mut tui, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(selected(&mut tui), 0);
    let text_before = tui.editor(FIRST_TAB_ID).text();
    press(&state, &mut tui, KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(tui.editor(FIRST_TAB_ID).text(), format!("{text_before}x"));
}

#[test]
fn results_arrows_enter_and_copy_map_to_query_messages() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Left, Msg::Query(QueryMsg::Move(GridMove::Left))),
        (KeyCode::Right, Msg::Query(QueryMsg::Move(GridMove::Right))),
        (KeyCode::Up, Msg::Query(QueryMsg::Move(GridMove::Up))),
        (KeyCode::Down, Msg::Query(QueryMsg::Move(GridMove::Down))),
        (KeyCode::Enter, Msg::Query(QueryMsg::OpenRowDetail)),
        (KeyCode::Char('y'), Msg::Query(QueryMsg::CopyCell)),
    ];
    for (code, expected) in cases {
        assert_eq!(
            press_in_results(&state, &mut tui, code, KeyModifiers::NONE),
            vec![expected],
            "{code:?}"
        );
    }
    assert!(press_in_results(&state, &mut tui, KeyCode::Char('z'), KeyModifiers::NONE).is_empty());
}

#[test]
fn editor_mode_badge_shows_visual_and_search_modes() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    press(&state, &mut tui, KeyCode::Char('v'), KeyModifiers::NONE);
    assert!(screen_text(&state, &mut tui).contains(" VISUAL "));
    press(&state, &mut tui, KeyCode::Esc, KeyModifiers::NONE);
    press(&state, &mut tui, KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(screen_text(&state, &mut tui).contains(" SEARCH "));
}

#[test]
fn running_script_shows_the_cancel_hint() {
    let mut state = query_state();
    active_query_mut(&mut state).run = RunState::Running;
    let mut tui = TuiState::new(false);
    assert!(screen_text(&state, &mut tui).contains("Loading script (Ctrl+c cancels)"));
}

#[test]
fn report_without_results_shows_only_the_failure() {
    let mut state = query_state();
    active_query_mut(&mut state).run = RunState::Finished(ScriptReport {
        mode: TransactionMode::Transaction,
        results: Vec::new(),
        failure: Some(ScriptFailure {
            statement_index: 0,
            sql: "BEGIN".to_owned(),
            message: "ERROR: connection lost".to_owned(),
        }),
        elapsed: Duration::from_millis(1),
    });
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(text.contains("Statement 1 failed"));
    assert!(text.contains("ERROR: connection lost"));
    assert!(!text.contains("affected"));
}

#[test]
fn statement_labels_scroll_to_keep_the_selected_result_visible() {
    let mut state = query_state();
    let query = active_query_mut(&mut state);
    query.run = RunState::Finished(ScriptReport {
        mode: TransactionMode::Transaction,
        results: (1..=12)
            .map(|index| rows_result(&format!("select 'statement number {index:02}'"), "v"))
            .collect(),
        failure: None,
        elapsed: Duration::from_millis(3),
    });
    query.selected_result_index = 11;
    let mut tui = TuiState::new(false);
    let text = screen_text(&state, &mut tui);
    assert!(text.contains(" 12 select 'statement number… 1 row "));
    assert!(text.contains(" 11 select 'statement number… 1 row "));
    assert!(!text.contains(" 1 select 'statement number… 1 row "));
}
