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

impl TableView {
    pub(crate) fn first_page(relation: RelationRef, page_size: usize) -> TableView {
        TableView {
            request: PageRequest {
                relation: relation.clone(),
                page_index: 0,
                page_size,
                sort: None,
                filter: None,
            },
            relation,
            tab: TableTab::Data,
            page: Loadable::Loading,
            cursor: GridCursor::default(),
            details: Loadable::Idle,
        }
    }
}
