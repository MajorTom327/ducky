use ducky_core::FileEntry;

use crate::loadable::Loadable;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilesView {
    pub entries: Loadable<Vec<FileEntry>>,
    pub selected_entry_index: usize,
    pub preview: Loadable<String>,
}

impl FilesView {
    pub(crate) fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.ready()?.get(self.selected_entry_index)
    }
}
