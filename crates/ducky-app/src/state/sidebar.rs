use ducky_core::{RelationKind, RelationRef};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sidebar {
    pub selected_item_index: usize,
    pub filter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItem {
    pub relation: RelationRef,
    pub kind: RelationKind,
    pub estimated_row_count: Option<i64>,
}
