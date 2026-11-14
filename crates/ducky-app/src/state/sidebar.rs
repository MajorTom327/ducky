use ducky_core::{RelationKind, RelationRef};

pub const DEFAULT_SIDEBAR_WIDTH: u16 = 32;
pub const MIN_SIDEBAR_WIDTH: u16 = 20;
pub const MAX_SIDEBAR_WIDTH: u16 = 80;
pub const SIDEBAR_WIDTH_STEP: u16 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidebar {
    pub selected_item_index: usize,
    pub filter: String,
    pub width: u16,
    pub is_collapsed: bool,
}

impl Default for Sidebar {
    fn default() -> Sidebar {
        Sidebar {
            selected_item_index: 0,
            filter: String::new(),
            width: DEFAULT_SIDEBAR_WIDTH,
            is_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItem {
    pub relation: RelationRef,
    pub kind: RelationKind,
    pub estimated_row_count: Option<i64>,
}
