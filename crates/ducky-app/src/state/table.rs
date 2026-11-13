use std::collections::BTreeSet;

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
    pub marked_row_indexes: BTreeSet<usize>,
    pub visual_anchor_row_index: Option<usize>,
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
            marked_row_indexes: BTreeSet::new(),
            visual_anchor_row_index: None,
        }
    }

    pub fn selected_row_indexes(&self) -> Vec<usize> {
        let mut selected_row_indexes = self.marked_row_indexes.clone();
        if let Some(anchor_row_index) = self.visual_anchor_row_index {
            let first_row_index = anchor_row_index.min(self.cursor.row_index);
            let last_row_index = anchor_row_index.max(self.cursor.row_index);
            selected_row_indexes.extend(first_row_index..=last_row_index);
        }
        selected_row_indexes.into_iter().collect()
    }

    pub fn has_selection(&self) -> bool {
        !self.marked_row_indexes.is_empty() || self.visual_anchor_row_index.is_some()
    }

    pub(crate) fn clear_selection(&mut self) {
        self.marked_row_indexes.clear();
        self.visual_anchor_row_index = None;
    }
}
