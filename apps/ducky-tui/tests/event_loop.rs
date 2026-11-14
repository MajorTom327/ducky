use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ducky_app::{
    AppState, Connection, Focus, Msg, Notice, NoticeLevel, QueryView, Screen, Tab, TabContent,
    Workspace,
};
use ducky_core::{PgTools, Profile, ProfileKind, SessionInfo, StoragePaths, ToolStatus};
use ducky_runtime::Runtime;
use ducky_tui::event_loop::{TerminalIo, run};
use ducky_tui::tui_state::TuiState;
use futures::stream::{self, BoxStream, StreamExt};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tempfile::TempDir;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

const STEADY_BLOCK: &str = "\x1b[2 q";
const STEADY_BAR: &str = "\x1b[6 q";
const RUN_LIMIT: Duration = Duration::from_secs(5);

fn profile() -> Profile {
    Profile {
        label: "Dev local".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
        project: None,
        tags: Vec::new(),
    }
}

fn storage(storage_dir: &TempDir) -> StoragePaths {
    StoragePaths {
        sql_dir: storage_dir.path().join("sql"),
        dump_dir: storage_dir.path().join("dumps"),
    }
}

fn profiles_state(storage_dir: &TempDir) -> AppState {
    AppState::new(vec![profile()], storage(storage_dir))
}

fn query_editor_state(storage_dir: &TempDir) -> AppState {
    let mut state = profiles_state(storage_dir);
    let mut workspace = Workspace::new(
        0,
        profile(),
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
    workspace.tabs = vec![Tab {
        id: 1,
        content: TabContent::Query(QueryView::new(1)),
    }];
    workspace.active_tab_index = 0;
    workspace.next_tab_id = 2;
    workspace.next_query_number = 2;
    workspace.focus = Focus::Main;
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn notice(text: &str) -> Msg {
    Msg::Notified(Notice {
        level: NoticeLevel::Info,
        text: text.to_owned(),
    })
}

fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> io::Result<Event> {
    Ok(Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::NONE,
    }))
}

fn press(code: KeyCode) -> io::Result<Event> {
    key(code, KeyModifiers::NONE, KeyEventKind::Press)
}

fn ending(events: Vec<io::Result<Event>>) -> BoxStream<'static, io::Result<Event>> {
    stream::iter(events).boxed()
}

fn then_pending(events: Vec<io::Result<Event>>) -> BoxStream<'static, io::Result<Event>> {
    stream::iter(events).chain(stream::pending()).boxed()
}

struct LoopRun {
    outcome: anyhow::Result<()>,
    screen: String,
    cursor_bytes: String,
}

struct LoopSetup {
    state: AppState,
    events: BoxStream<'static, io::Result<Event>>,
    startup_messages: Vec<Msg>,
    channel_messages: Vec<Msg>,
}

async fn run_loop(setup: LoopSetup) -> LoopRun {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let (runtime_sender, receiver) = unbounded_channel();
    let test_sender: UnboundedSender<Msg> = runtime_sender.clone();
    let runtime = Runtime::new(storage(&storage_dir), runtime_sender);
    for message in setup.channel_messages {
        test_sender
            .send(message)
            .expect("the loop receives messages");
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("creates a test terminal");
    let mut cursor_writer = Vec::new();
    let terminal_io = TerminalIo {
        terminal: &mut terminal,
        events: setup.events,
        cursor_writer: &mut cursor_writer,
    };
    let outcome = tokio::time::timeout(
        RUN_LIMIT,
        run(
            terminal_io,
            setup.state,
            TuiState::new(false),
            runtime,
            receiver,
            setup.startup_messages,
        ),
    )
    .await
    .expect("the event loop returns");
    let buffer = terminal.backend().buffer();
    let screen = buffer
        .content()
        .chunks(usize::from(buffer.area.width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    LoopRun {
        outcome,
        screen,
        cursor_bytes: String::from_utf8(cursor_writer).expect("cursor bytes are utf-8"),
    }
}

#[tokio::test]
async fn startup_messages_are_applied_before_the_first_frame() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: ending(Vec::new()),
        startup_messages: vec![notice("startup notice")],
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(loop_run.screen.contains("Dev local"));
    assert!(loop_run.screen.contains("startup notice"));
}

#[tokio::test]
async fn ctrl_q_press_quits_the_loop() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: then_pending(vec![key(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        )]),
        startup_messages: Vec::new(),
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
}

#[tokio::test]
async fn key_press_is_routed_and_dismisses_the_notice() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: ending(vec![press(KeyCode::Down)]),
        startup_messages: vec![notice("startup notice")],
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(!loop_run.screen.contains("startup notice"));
}

#[tokio::test]
async fn key_release_and_other_events_are_ignored() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: ending(vec![
            key(
                KeyCode::Char('q'),
                KeyModifiers::CONTROL,
                KeyEventKind::Release,
            ),
            key(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release),
            Ok(Event::FocusGained),
        ]),
        startup_messages: vec![notice("startup notice")],
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(loop_run.screen.contains("startup notice"));
}

#[tokio::test]
async fn paste_is_forwarded_to_the_focused_editor() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: query_editor_state(&storage_dir),
        events: ending(vec![Ok(Event::Paste("select 4242".to_owned()))]),
        startup_messages: Vec::new(),
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(loop_run.screen.contains("select 4242"));
}

#[tokio::test]
async fn channel_messages_are_applied() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: then_pending(Vec::new()),
        startup_messages: Vec::new(),
        channel_messages: vec![notice("runtime notice"), Msg::Quit],
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(loop_run.screen.contains("runtime notice"));
}

#[tokio::test]
async fn event_stream_error_is_returned() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: then_pending(vec![Err(io::Error::other("terminal gone"))]),
        startup_messages: Vec::new(),
        channel_messages: Vec::new(),
    })
    .await;
    let error = loop_run
        .outcome
        .expect_err("the stream error ends the loop");
    assert_eq!(error.to_string(), "terminal gone");
}

#[tokio::test]
async fn cursor_style_is_written_only_when_the_shape_changes() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: query_editor_state(&storage_dir),
        events: ending(vec![
            press(KeyCode::Char('i')),
            press(KeyCode::Char('x')),
            press(KeyCode::Esc),
        ]),
        startup_messages: Vec::new(),
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert_eq!(
        loop_run.cursor_bytes,
        format!("{STEADY_BLOCK}{STEADY_BAR}{STEADY_BLOCK}")
    );
}

#[tokio::test]
async fn profiles_screen_uses_the_bar_cursor_once() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let loop_run = run_loop(LoopSetup {
        state: profiles_state(&storage_dir),
        events: ending(vec![press(KeyCode::Down), press(KeyCode::Up)]),
        startup_messages: Vec::new(),
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert_eq!(loop_run.cursor_bytes, STEADY_BAR);
}

#[tokio::test]
async fn mouse_press_is_routed_and_dismisses_the_notice() {
    let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
    let sidebar_click = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 10,
        modifiers: KeyModifiers::NONE,
    });
    let loop_run = run_loop(LoopSetup {
        state: query_editor_state(&storage_dir),
        events: ending(vec![Ok(sidebar_click)]),
        startup_messages: vec![notice("startup notice")],
        channel_messages: Vec::new(),
    })
    .await;
    assert!(loop_run.outcome.is_ok());
    assert!(!loop_run.screen.contains("startup notice"));
    let status_line = loop_run.screen.lines().last().expect("a status line");
    assert!(status_line.trim_end().ends_with("Sidebar"));
}
