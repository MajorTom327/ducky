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
    screen_with(state, &mut TuiState::new(false))
}

fn screen_with(state: &AppState, tui: &mut TuiState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
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

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn workspace_of(state: &AppState) -> &Workspace {
    state.workspace().expect("workspace")
}

#[test]
fn files_keys_map_messages_and_scroll_the_preview() {
    use ducky_tui::keys::files::handle;
    let state = workspace_state(TabContent::Files, available_tools());
    let workspace = workspace_of(&state);
    let mut tui = TuiState::new(false);
    assert!(
        handle(
            workspace,
            &mut tui,
            key(KeyCode::Char('d'), KeyModifiers::CONTROL)
        )
        .is_empty()
    );
    assert!(
        handle(
            workspace,
            &mut tui,
            key(KeyCode::Char('d'), KeyModifiers::CONTROL)
        )
        .is_empty()
    );
    assert_eq!(tui.preview_scroll, 20);
    assert!(
        handle(
            workspace,
            &mut tui,
            key(KeyCode::Char('u'), KeyModifiers::CONTROL)
        )
        .is_empty()
    );
    assert_eq!(tui.preview_scroll, 10);
    assert_eq!(
        handle(workspace, &mut tui, key(KeyCode::Down, KeyModifiers::NONE)),
        vec![Msg::Files(FilesMsg::Next)]
    );
    assert_eq!(tui.preview_scroll, 0);
    tui.preview_scroll = 5;
    assert_eq!(
        handle(workspace, &mut tui, key(KeyCode::Up, KeyModifiers::NONE)),
        vec![Msg::Files(FilesMsg::Previous)]
    );
    assert_eq!(tui.preview_scroll, 0);
    tui.preview_scroll = 5;
    let cases = [
        (KeyCode::Enter, vec![Msg::Files(FilesMsg::Open)]),
        (KeyCode::Esc, vec![Msg::ToggleFocus]),
        (KeyCode::Char('r'), vec![Msg::Files(FilesMsg::Reload)]),
        (KeyCode::Char('z'), Vec::new()),
    ];
    for (code, expected) in cases {
        assert_eq!(
            handle(workspace, &mut tui, key(code, KeyModifiers::NONE)),
            expected
        );
    }
    assert_eq!(tui.preview_scroll, 5);
}

#[test]
fn job_keys_scroll_the_log_and_return_to_follow_mode() {
    use ducky_tui::keys::jobs::handle;
    let mut state = workspace_state(TabContent::Dump, available_tools());
    workspace_mut(&mut state).dump.log_lines = (0..25).map(|index| index.to_string()).collect();
    let workspace = workspace_of(&state);
    let mut tui = TuiState::new(false);
    let ctrl_u = key(KeyCode::Char('u'), KeyModifiers::CONTROL);
    let ctrl_d = key(KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert!(handle(workspace, &mut tui, ctrl_d).is_empty());
    assert_eq!(tui.job_log_scroll, None);
    assert!(handle(workspace, &mut tui, ctrl_u).is_empty());
    assert_eq!(tui.job_log_scroll, Some(15));
    handle(workspace, &mut tui, ctrl_u);
    assert_eq!(tui.job_log_scroll, Some(5));
    handle(workspace, &mut tui, ctrl_d);
    assert_eq!(tui.job_log_scroll, Some(15));
    handle(workspace, &mut tui, ctrl_d);
    assert_eq!(tui.job_log_scroll, None);
}

#[test]
fn job_keys_map_dump_and_restore_messages() {
    use ducky_tui::keys::jobs::handle;
    let mut tui = TuiState::new(false);
    let dump_state = workspace_state(TabContent::Dump, available_tools());
    let dump = workspace_of(&dump_state);
    let dump_cases = [
        (KeyCode::Enter, vec![Msg::Dump(DumpMsg::Start)]),
        (KeyCode::Char('e'), vec![Msg::Dump(DumpMsg::EditFileName)]),
        (KeyCode::Esc, vec![Msg::ToggleFocus]),
        (KeyCode::Char('z'), Vec::new()),
    ];
    for (code, expected) in dump_cases {
        assert_eq!(
            handle(dump, &mut tui, key(code, KeyModifiers::NONE)),
            expected
        );
    }
    let restore_state = workspace_state(TabContent::Restore, available_tools());
    let restore = workspace_of(&restore_state);
    let restore_cases = [
        (KeyCode::Down, vec![Msg::Restore(RestoreMsg::Next)]),
        (KeyCode::Up, vec![Msg::Restore(RestoreMsg::Previous)]),
        (KeyCode::Char('j'), vec![Msg::Restore(RestoreMsg::Next)]),
        (KeyCode::Char('r'), vec![Msg::Restore(RestoreMsg::Reload)]),
    ];
    for (code, expected) in restore_cases {
        assert_eq!(
            handle(restore, &mut tui, key(code, KeyModifiers::NONE)),
            expected
        );
    }
    let files_state = workspace_state(TabContent::Files, available_tools());
    assert!(
        handle(
            workspace_of(&files_state),
            &mut tui,
            key(KeyCode::Enter, KeyModifiers::NONE)
        )
        .is_empty()
    );
}

#[test]
fn files_view_shows_human_sizes_and_modification_dates() {
    let mut state = workspace_state(TabContent::Files, available_tools());
    let modified_at = std::time::SystemTime::now();
    let expected_date = chrono::DateTime::<chrono::Local>::from(modified_at)
        .format("%Y-%m-%d")
        .to_string();
    workspace_mut(&mut state).files.entries = Loadable::Ready(vec![
        FileEntry {
            name: "big.sql".to_owned(),
            size_bytes: 3 * 1024 * 1024,
            modified_at: Some(modified_at),
        },
        FileEntry {
            name: "small.sql".to_owned(),
            size_bytes: 2048,
            modified_at: None,
        },
    ]);
    let text = screen(&state);
    assert!(text.contains(&format!("big.sql  3.0 MB  {expected_date}")));
    assert!(text.contains("small.sql  2.0 KB"));
}

#[test]
fn restore_view_explains_a_missing_tool() {
    let tools = PgTools {
        pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
        pg_restore: ToolStatus::Missing("`pg_restore` was not found in PATH".to_owned()),
    };
    let state = workspace_state(TabContent::Restore, tools);
    let text = screen(&state);
    assert!(text.contains("Restore is disabled: `pg_restore` was not found in PATH."));
    assert!(!text.contains("clean before restore"));
}

#[test]
fn restore_view_explains_an_empty_dump_dir() {
    let mut state = workspace_state(TabContent::Restore, available_tools());
    workspace_mut(&mut state).restore.entries = Loadable::Ready(Vec::new());
    let text = flattened(&screen(&state));
    assert!(text.contains("No dumps in /tmp/ducky/dumps. Create one from Dump database."));
}

#[test]
fn restore_view_shows_a_listing_error_and_job_failure() {
    let mut state = workspace_state(TabContent::Restore, available_tools());
    let workspace = workspace_mut(&mut state);
    workspace.restore.entries = Loadable::Failed("permission denied on dumps".to_owned());
    workspace.restore.job = JobState::Failed("pg_restore exited with 1".to_owned());
    let text = screen(&state);
    assert!(text.contains("permission denied on dumps"));
    assert!(text.contains("pg_restore exited with 1"));
}

#[test]
fn dump_view_shows_custom_target_and_success_summary() {
    let mut state = workspace_state(TabContent::Dump, available_tools());
    let workspace = workspace_mut(&mut state);
    workspace.dump.custom_file_name = Some("nightly.dump".to_owned());
    workspace.dump.job = JobState::Succeeded("Dump written to /tmp/nightly.dump".to_owned());
    let text = screen(&state);
    assert!(text.contains("/tmp/ducky/dumps/nightly.dump"));
    assert!(!text.contains("(automatic)"));
    assert!(text.contains("Dump written to /tmp/nightly.dump"));
}

#[test]
fn job_log_scroll_offset_shows_older_lines_and_clamps() {
    let mut state = workspace_state(TabContent::Dump, available_tools());
    workspace_mut(&mut state).dump.log_lines = (1..=50)
        .map(|index| format!("log line {index:02}"))
        .collect();
    let mut tui = TuiState::new(false);
    tui.job_log_scroll = Some(0);
    let text = screen_with(&state, &mut tui);
    assert!(text.contains("log line 01"));
    assert!(!text.contains("log line 50"));
    tui.job_log_scroll = Some(u16::MAX);
    let text = screen_with(&state, &mut tui);
    assert!(text.contains("log line 50"));
    assert!(!text.contains("log line 01"));
}
