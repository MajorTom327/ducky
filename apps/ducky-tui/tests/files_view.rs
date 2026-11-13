use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, DumpMsg, FilesMsg, Focus, JobState, Loadable, Msg, RestoreMsg, Screen,
    Tab, TabContent, Workspace,
};
use ducky_core::{
    FileEntry, PgTools, Profile, ProfileKind, RestoreOptions, SessionInfo, StoragePaths, ToolStatus,
};
use ducky_tui::keys::handle_key;
use ducky_tui::layout::SIDEBAR_WIDTH;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn available_tools() -> PgTools {
    PgTools {
        pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
        pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
    }
}

fn workspace_state(content: TabContent, tools: PgTools) -> AppState {
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
            tools,
        },
    );
    workspace.tabs = vec![Tab { id: 1, content }];
    workspace.next_tab_id = 2;
    workspace.focus = Focus::Main;
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

fn entry(name: &str) -> FileEntry {
    FileEntry {
        name: name.to_owned(),
        size_bytes: 42,
        modified_at: None,
    }
}

fn screen(state: &AppState) -> String {
    let mut tui = TuiState::new(false);
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, &mut tui))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (SIDEBAR_WIDTH..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn flattened(text: &str) -> String {
    text.replace(['│', '─', '┌', '┐', '└', '┘'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn press(state: &AppState, code: KeyCode) -> Vec<Msg> {
    let mut tui = TuiState::new(false);
    handle_key(state, &mut tui, KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn files_view_lists_entries_and_shows_the_preview() {
    let mut state = workspace_state(TabContent::Files, available_tools());
    let workspace = workspace_mut(&mut state);
    workspace.files.entries = Loadable::Ready(vec![entry("first.sql"), entry("second.sql")]);
    workspace.files.preview = Loadable::Ready("select 42 from customers".to_owned());
    let text = screen(&state);
    assert!(text.contains("SQL files — /tmp/ducky/sql"));
    assert!(text.contains("first.sql"));
    assert!(text.contains("second.sql"));
    assert!(text.contains("select 42 from customers"));
}

#[test]
fn files_view_explains_the_empty_state() {
    let mut state = workspace_state(TabContent::Files, available_tools());
    workspace_mut(&mut state).files.entries = Loadable::Ready(Vec::new());
    let text = flattened(&screen(&state));
    assert!(text.contains("No .sql files yet."));
    assert!(text.contains("Ctrl+s"));
}

#[test]
fn dump_view_explains_a_missing_tool() {
    let tools = PgTools {
        pg_dump: ToolStatus::Missing("`pg_dump` was not found in PATH".to_owned()),
        pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
    };
    let state = workspace_state(TabContent::Dump, tools);
    let text = screen(&state);
    assert!(text.contains("Dump is disabled: `pg_dump` was not found in PATH."));
    let joined = flattened(&text);
    assert!(joined.contains("brew install libpq"));
    assert!(joined.contains("apt install postgresql-client"));
}

#[test]
fn restore_view_reflects_the_options() {
    let mut state = workspace_state(TabContent::Restore, available_tools());
    let workspace = workspace_mut(&mut state);
    workspace.restore.entries = Loadable::Ready(vec![entry("ducky.dump")]);
    workspace.restore.options = RestoreOptions {
        clean_before_restore: true,
        skip_ownership: false,
    };
    let text = screen(&state);
    assert!(text.contains("ducky.dump"));
    assert!(text.contains("[x] clean before restore (c)"));
    assert!(text.contains("[ ] skip ownership (o)"));
}

#[test]
fn running_job_shows_its_last_log_line() {
    let mut state = workspace_state(TabContent::Dump, available_tools());
    let workspace = workspace_mut(&mut state);
    workspace.dump.job = JobState::Running;
    workspace.dump.log_lines = (1..=50).map(|index| format!("log line {index}")).collect();
    let text = screen(&state);
    assert!(text.contains("log line 50"));
}

#[test]
fn keys_map_to_files_and_job_messages() {
    let files_state = workspace_state(TabContent::Files, available_tools());
    assert_eq!(
        press(&files_state, KeyCode::Char('x')),
        vec![Msg::Files(FilesMsg::Execute)]
    );
    let restore_state = workspace_state(TabContent::Restore, available_tools());
    assert_eq!(
        press(&restore_state, KeyCode::Char('c')),
        vec![Msg::Restore(RestoreMsg::ToggleClean)]
    );
    assert_eq!(
        press(&restore_state, KeyCode::Char('o')),
        vec![Msg::Restore(RestoreMsg::ToggleSkipOwnership)]
    );
    assert_eq!(
        press(&restore_state, KeyCode::Enter),
        vec![Msg::Restore(RestoreMsg::Start)]
    );
    let dump_state = workspace_state(TabContent::Dump, available_tools());
    assert_eq!(
        press(&dump_state, KeyCode::Enter),
        vec![Msg::Dump(DumpMsg::Start)]
    );
}
