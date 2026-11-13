use ducky_core::{RelationKind, RelationRef};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sidebar {
    pub selected_item_index: usize,
    pub filter: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    SqlCommand,
    SqlFiles,
    Dump,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarItem {
    Action(SidebarAction),
    Relation {
        relation: RelationRef,
        kind: RelationKind,
    },
}
