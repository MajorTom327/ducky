use ducky_core::{ScriptReport, StatementResult, TransactionMode};

use crate::state::table::GridCursor;
use crate::state::tabs::TabId;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub name: Option<String>,
    pub text: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryPane {
    #[default]
    Editor,
    Results,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RunState {
    #[default]
    Idle,
    Running,
    Finished(ScriptReport),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSave {
    pub tab_id: TabId,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryView {
    pub number: usize,
    pub document: Document,
    pub transaction_mode: TransactionMode,
    pub run: RunState,
    pub selected_result_index: usize,
    pub cursor: GridCursor,
    pub pane: QueryPane,
}

impl QueryView {
    pub fn new(number: usize) -> QueryView {
        QueryView {
            number,
            document: Document::default(),
            transaction_mode: TransactionMode::Transaction,
            run: RunState::Idle,
            selected_result_index: 0,
            cursor: GridCursor::default(),
            pane: QueryPane::Editor,
        }
    }

    pub fn selected_result(&self) -> Option<&StatementResult> {
        match &self.run {
            RunState::Finished(report) => report.results.get(self.selected_result_index),
            RunState::Idle | RunState::Running => None,
        }
    }
}
