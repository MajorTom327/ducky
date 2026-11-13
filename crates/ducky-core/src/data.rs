use crate::catalog::RelationRef;

pub type Cell = Option<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sort {
    pub column: String,
    pub direction: SortDirection,
}

pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const MAX_PAGE_SIZE: usize = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    pub relation: RelationRef,
    pub page_index: usize,
    pub page_size: usize,
    pub sort: Option<Sort>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub result_set: ResultSet,
    pub has_next_page: bool,
}
