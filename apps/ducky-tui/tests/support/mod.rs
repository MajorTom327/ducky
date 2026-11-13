#![allow(dead_code)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{AppState, JobState, Msg, Screen, TabContent, Workspace, update};
use ducky_core::{Profile, StoragePaths};
use ducky_runtime::Runtime;
use ducky_tui::keys::handle_key;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use edtui::{EditorState, Lines};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tempfile::TempDir;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

const WIDTH: u16 = 140;
const HEIGHT: u16 = 40;
const QUIET_PERIOD: Duration = Duration::from_millis(300);
const SETTLE_LIMIT: Duration = Duration::from_secs(15);

pub struct Harness {
    pub state: AppState,
    pub tui: TuiState,
    pub runtime: Runtime,
    pub messages: UnboundedReceiver<Msg>,
    pub terminal: Terminal<TestBackend>,
    pub storage_dir: TempDir,
}

impl Harness {
    pub fn new(profiles: Vec<Profile>) -> Harness {
        let storage_dir = tempfile::tempdir().expect("creates a temporary directory");
        let storage = StoragePaths {
            sql_dir: storage_dir.path().join("sql"),
            dump_dir: storage_dir.path().join("dumps"),
        };
        let (sender, messages) = unbounded_channel();
        let runtime = Runtime::new(storage.clone(), sender);
        let terminal =
            Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("creates a test terminal");
        Harness {
            state: AppState::new(profiles, storage),
            tui: TuiState::new(false),
            runtime,
            messages,
            terminal,
            storage_dir,
        }
    }

    pub fn sql_dir(&self) -> PathBuf {
        self.state.storage.sql_dir.clone()
    }

    pub fn workspace(&self) -> &Workspace {
        self.state.workspace().expect("a workspace is open")
    }

    pub fn send(&mut self, msg: Msg) {
        for command in update(&mut self.state, msg) {
            self.runtime.dispatch(command);
        }
    }

    pub fn press(&mut self, key: KeyEvent) {
        self.sync_editor();
        for msg in handle_key(&self.state, &mut self.tui, key) {
            self.send(msg);
        }
    }

    pub fn press_code(&mut self, code: KeyCode) {
        self.press(KeyEvent::new(code, KeyModifiers::NONE));
    }

    pub fn press_char(&mut self, character: char) {
        self.press_code(KeyCode::Char(character));
    }

    pub fn press_ctrl(&mut self, character: char) {
        self.press(KeyEvent::new(
            KeyCode::Char(character),
            KeyModifiers::CONTROL,
        ));
    }

    pub fn type_text(&mut self, text: &str) {
        for character in text.chars() {
            self.press_char(character);
        }
    }

    pub fn set_editor_text(&mut self, text: &str) {
        self.sync_editor();
        let tab_id = self
            .workspace()
            .active_tab()
            .expect("a query tab is active")
            .id;
        self.tui.editor(tab_id).state = EditorState::new(Lines::from(text));
    }

    pub async fn settle(&mut self) {
        let deadline = Instant::now() + SETTLE_LIMIT;
        loop {
            assert!(
                Instant::now() < deadline,
                "state did not settle within {SETTLE_LIMIT:?}"
            );
            match tokio::time::timeout(QUIET_PERIOD, self.messages.recv()).await {
                Ok(Some(msg)) => self.send(msg),
                Ok(None) => panic!("runtime message channel closed"),
                Err(_) if !is_busy(&self.state) => return,
                Err(_) => {}
            }
        }
    }

    pub fn screen(&mut self) -> String {
        self.sync_editor();
        let state = &self.state;
        let tui = &mut self.tui;
        self.terminal
            .draw(|frame| draw(frame, state, tui))
            .expect("draws a frame");
        let buffer = self.terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn sync_editor(&mut self) {
        if let Some(workspace) = self.state.workspace() {
            self.tui.sync_workspace(workspace);
        }
    }
}

fn is_busy(state: &AppState) -> bool {
    let workspace = match &state.screen {
        Screen::Profiles(picker) => return picker.connecting_profile_index.is_some(),
        Screen::Workspace(workspace) => workspace,
    };
    let is_tab_loading = workspace.tabs.iter().any(|tab| match &tab.content {
        TabContent::Table(table) => table.page.is_loading() || table.details.is_loading(),
        TabContent::Query(_) | TabContent::Files | TabContent::Dump | TabContent::Restore => false,
    });
    workspace.catalog.is_loading()
        || is_tab_loading
        || workspace.running_script_tab_id.is_some()
        || workspace.switching_profile_index.is_some()
        || workspace.files.entries.is_loading()
        || workspace.files.preview.is_loading()
        || workspace.restore.entries.is_loading()
        || matches!(workspace.dump.job, JobState::Running)
        || matches!(workspace.restore.job, JobState::Running)
}
