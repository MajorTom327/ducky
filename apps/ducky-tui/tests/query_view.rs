use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Document, Focus, GridMove, Loadable, MainView, Msg, QueryMsg, QueryPane,
    RunState, Screen, Workspace,
};
use ducky_core::{
    Catalog, Column, PgTools, Profile, ProfileKind, Relation, RelationKind, ResultSet, Schema,
    ScriptFailure, ScriptReport, SessionInfo, StatementOutcome, StatementResult, StoragePaths,
    ToolStatus, TransactionMode,
};
use ducky_sql::TokenKind;
use ducky_tui::keys::query::{handle_editor, handle_results};
use ducky_tui::theme::token_style;
use ducky_tui::tui_state::TuiState;
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

fn query_state() -> AppState {
    let profile = Profile {
        label: "Dev local".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
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
    workspace.view = MainView::Query;
    workspace.focus = Focus::Main;
    workspace.query.pane = QueryPane::Editor;
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

fn press(state: &AppState, tui: &mut TuiState, code: KeyCode, modifiers: KeyModifiers) -> Vec<Msg> {
    handle_editor(workspace(state), tui, KeyEvent::new(code, modifiers))
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
    assert_eq!(tui.editor.text(), "select 1");
    let keyword_highlight = Highlight::new(
        Index2::new(0, 0),
        Index2::new(0, 5),
        token_style(TokenKind::Keyword),
    );
    assert!(tui.editor.state.highlights.contains(&keyword_highlight));
}

#[test]
fn completion_popup_opens_and_tab_accepts() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from cu");
    let popup = tui.editor.completion.as_ref().expect("completion popup");
    assert_eq!(popup.completion.items[0].label, "customers");
    let messages = press(&state, &mut tui, KeyCode::Tab, KeyModifiers::NONE);
    assert!(messages.is_empty());
    assert_eq!(tui.editor.text(), "select * from customers");
    assert!(tui.editor.completion.is_none());
}

#[test]
fn escape_closes_the_popup_and_stays_in_insert_mode() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    type_in_insert_mode(&state, &mut tui, "select * from cu");
    press(&state, &mut tui, KeyCode::Esc, KeyModifiers::NONE);
    assert!(tui.editor.completion.is_none());
    assert!(tui.editor.is_insert_mode());
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
fn tab_in_normal_mode_toggles_focus() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    let messages = press(&state, &mut tui, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(messages, vec![Msg::ToggleFocus]);
}

#[test]
fn sync_document_replaces_the_buffer_on_a_new_revision() {
    let mut tui = TuiState::new(false);
    tui.editor.sync_document(&Document {
        name: None,
        text: "select 1".to_owned(),
        revision: 1,
    });
    assert_eq!(tui.editor.text(), "select 1");
    tui.editor.sync_document(&Document {
        name: Some("report".to_owned()),
        text: "select 2".to_owned(),
        revision: 1,
    });
    assert_eq!(tui.editor.text(), "select 1");
    tui.editor.sync_document(&Document {
        name: Some("report".to_owned()),
        text: "select 3".to_owned(),
        revision: 2,
    });
    assert_eq!(tui.editor.text(), "select 3");
}

#[test]
fn results_keys_map_to_query_messages() {
    let state = query_state();
    let mut tui = TuiState::new(false);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert_eq!(
        handle_results(workspace(&state), &mut tui, key(KeyCode::Char('j'))),
        vec![Msg::Query(QueryMsg::Move(GridMove::Down))]
    );
    assert_eq!(
        handle_results(workspace(&state), &mut tui, key(KeyCode::Char(']'))),
        vec![Msg::Query(QueryMsg::NextResult)]
    );
    assert_eq!(
        handle_results(workspace(&state), &mut tui, key(KeyCode::Esc)),
        vec![Msg::ToggleFocus]
    );
}

#[test]
fn results_view_renders_a_failure_report() {
    let mut state = query_state();
    workspace_mut(&mut state).query.run = RunState::Finished(ScriptReport {
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
fn results_view_lists_every_statement() {
    let mut state = query_state();
    let query = &mut workspace_mut(&mut state).query;
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
    assert!(text.contains("1: select 'first' (1 rows)"));
    assert!(text.contains("2: select 'second' (1 rows)"));
    assert!(text.contains("7 ms · autocommit"));
    assert!(text.contains("second"));
}
