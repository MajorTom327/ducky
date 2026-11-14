#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ducky_app::{
    AppState, Command, Connection, Loadable, Msg, Screen, SidebarMsg, TabId, TabsMsg, Workspace,
    update,
};
use ducky_core::{
    Catalog, Column, Page, PgTools, Profile, ProfileKind, Relation, RelationKind, ResultSet,
    Schema, ScriptReport, SessionInfo, StatementOutcome, StatementResult, StoragePaths, ToolStatus,
    TransactionMode,
};
use ducky_tui::mouse::handle_mouse;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

pub const WIDTH: u16 = 120;
pub const HEIGHT: u16 = 36;

fn column(name: &str) -> Column {
    Column {
        name: name.to_owned(),
        data_type: "text".to_owned(),
        is_nullable: true,
        default_expression: None,
        is_primary_key: name == "id",
        enum_labels: Vec::new(),
    }
}

fn relation(name: &str, kind: RelationKind) -> Relation {
    Relation {
        name: name.to_owned(),
        kind,
        estimated_row_count: Some(3),
        columns: vec![column("id"), column("name")],
        foreign_keys: Vec::new(),
    }
}

pub fn profile(label: &str) -> Profile {
    Profile {
        label: label.to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
        project: None,
        tags: Vec::new(),
    }
}

pub fn storage() -> StoragePaths {
    StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    }
}

pub fn workspace_state() -> AppState {
    let profiles = vec![profile("Dev local"), profile("Staging copy")];
    let mut state = AppState::new(profiles.clone(), storage());
    let mut workspace = Workspace::new(
        0,
        profiles[0].clone(),
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
        schemas: vec![
            Schema {
                name: "analytics".to_owned(),
                relations: vec![relation("events", RelationKind::Table)],
            },
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    relation("customers", RelationKind::Table),
                    relation("order_totals", RelationKind::View),
                    relation("orders", RelationKind::Table),
                ],
            },
        ],
    });
    workspace.schema_name = "public".to_owned();
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

pub fn workspace(state: &AppState) -> &Workspace {
    state.workspace().expect("a workspace is open")
}

pub fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

pub fn send(state: &mut AppState, messages: impl IntoIterator<Item = Msg>) -> Vec<Command> {
    messages
        .into_iter()
        .flat_map(|message| update(state, message))
        .collect()
}

pub fn people() -> ResultSet {
    ResultSet {
        columns: vec!["id".to_owned(), "name".to_owned()],
        rows: [("1", "Ada"), ("2", "Bea"), ("3", "Cyd")]
            .into_iter()
            .map(|(id, name)| vec![Some(id.to_owned()), Some(name.to_owned())])
            .collect(),
        is_truncated: false,
    }
}

pub fn table_state() -> AppState {
    let mut state = workspace_state();
    let commands = send(&mut state, [Msg::Sidebar(SidebarMsg::Activate)]);
    let Some(Command::LoadPage(request)) = commands.into_iter().next() else {
        panic!("expected a page load");
    };
    send(
        &mut state,
        [Msg::PageLoaded {
            request,
            result: Ok(Page {
                result_set: people(),
                has_next_page: false,
            }),
        }],
    );
    state
}

pub fn active_tab_id(state: &AppState) -> TabId {
    workspace(state).active_tab().expect("an active tab").id
}

pub fn query_state_with_results() -> AppState {
    let mut state = workspace_state();
    send(&mut state, [Msg::Tabs(TabsMsg::NewQuery)]);
    let tab_id = active_tab_id(&state);
    send(
        &mut state,
        [Msg::ScriptFinished {
            tab_id,
            report: ScriptReport {
                mode: TransactionMode::Transaction,
                results: vec![StatementResult {
                    sql: "select * from people".to_owned(),
                    outcome: StatementOutcome::Rows(people()),
                    elapsed: Duration::from_millis(1),
                }],
                failure: None,
                elapsed: Duration::from_millis(1),
            },
        }],
    );
    state.notice = None;
    state
}

pub fn draw_rows(state: &AppState, tui: &mut TuiState) -> Vec<String> {
    if let Some(workspace) = state.workspace() {
        tui.sync_workspace(workspace);
    }
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
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
        .collect()
}

pub fn position_of(rows: &[String], needle: &str, min_column: u16) -> (u16, u16) {
    rows.iter()
        .enumerate()
        .find_map(|(row_index, row)| {
            let characters: Vec<char> = row.chars().collect();
            let needle_characters: Vec<char> = needle.chars().collect();
            (usize::from(min_column)..characters.len())
                .find(|start| characters[*start..].starts_with(&needle_characters))
                .map(|start| {
                    (
                        u16::try_from(start).expect("column"),
                        u16::try_from(row_index).expect("row"),
                    )
                })
        })
        .unwrap_or_else(|| panic!("`{needle}` is on screen"))
}

pub fn mouse_event(kind: MouseEventKind, (column, row): (u16, u16)) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub fn click(state: &AppState, tui: &mut TuiState, position: (u16, u16)) -> Vec<Msg> {
    handle_mouse(
        state,
        tui,
        mouse_event(MouseEventKind::Down(MouseButton::Left), position),
    )
}

pub fn wheel(
    state: &AppState,
    tui: &mut TuiState,
    position: (u16, u16),
    is_down: bool,
) -> Vec<Msg> {
    let kind = match is_down {
        true => MouseEventKind::ScrollDown,
        false => MouseEventKind::ScrollUp,
    };
    handle_mouse(state, tui, mouse_event(kind, position))
}
