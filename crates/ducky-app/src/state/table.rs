use ducky_core::{Page, PageRequest, RelationDetails, RelationRef};

use crate::loadable::Loadable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridCursor {
    pub row_index: usize,
    pub column_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableTab {
    Data,
    Structure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableView {
    pub relation: RelationRef,
    pub tab: TableTab,
    pub request: PageRequest,
    pub page: Loadable<Page>,
    pub cursor: GridCursor,
    pub details: Loadable<RelationDetails>,
}
