use ducky_core::{FileEntry, RestoreOptions};

use crate::loadable::Loadable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Dump,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum JobState {
    #[default]
    Idle,
    Running,
    Succeeded(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DumpView {
    pub custom_file_name: Option<String>,
    pub job: JobState,
    pub log_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RestoreView {
    pub entries: Loadable<Vec<FileEntry>>,
    pub selected_entry_index: usize,
    pub options: RestoreOptions,
    pub job: JobState,
    pub log_lines: Vec<String>,
}

impl RestoreView {
    pub(crate) fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.ready()?.get(self.selected_entry_index)
    }
}
