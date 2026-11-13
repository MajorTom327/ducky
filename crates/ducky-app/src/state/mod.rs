mod files;
mod jobs;
mod modal;
mod profiles;
mod query;
mod sidebar;
mod table;
mod workspace;

use ducky_core::{Profile, StoragePaths};

pub use files::FilesView;
pub use jobs::{DumpView, JobKind, JobState, RestoreView};
pub use modal::{Confirm, ConfirmKind, Modal, Prompt, PromptKind, RowDetail};
pub use profiles::ProfilePicker;
pub use query::{Document, PendingSave, QueryPane, QueryView, RunState};
pub use sidebar::{Sidebar, SidebarAction, SidebarItem};
pub use table::{GridCursor, TableTab, TableView};
pub use workspace::{Focus, MainView, Workspace};

pub const PAGE_SIZE: usize = 100;
pub const MAX_JOB_LOG_LINES: usize = 1000;

#[derive(Debug, Clone)]
pub struct AppState {
    pub profiles: Vec<Profile>,
    pub storage: StoragePaths,
    pub screen: Screen,
    pub modal: Option<Modal>,
    pub notice: Option<Notice>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(profiles: Vec<Profile>, storage: StoragePaths) -> AppState {
        AppState {
            profiles,
            storage,
            screen: Screen::Profiles(ProfilePicker::default()),
            modal: None,
            notice: None,
            should_quit: false,
        }
    }

    pub fn workspace(&self) -> Option<&Workspace> {
        match &self.screen {
            Screen::Workspace(workspace) => Some(workspace.as_ref()),
            Screen::Profiles(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Profiles(ProfilePicker),
    Workspace(Box<Workspace>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub level: NoticeLevel,
    pub text: String,
}

impl Notice {
    pub(crate) fn new(level: NoticeLevel, text: impl Into<String>) -> Notice {
        Notice {
            level,
            text: text.into(),
        }
    }
}
