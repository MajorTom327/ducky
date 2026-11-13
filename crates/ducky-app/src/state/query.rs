use ducky_core::{ScriptReport, StatementResult, TransactionMode};

use crate::state::table::GridCursor;

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
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryView {
    pub document: Document,
    pub transaction_mode: TransactionMode,
    pub run: RunState,
    pub selected_result_index: usize,
    pub cursor: GridCursor,
    pub pane: QueryPane,
    pub pending_save: Option<PendingSave>,
}

impl QueryView {
    pub fn selected_result(&self) -> Option<&StatementResult> {
        match &self.run {
            RunState::Finished(report) => report.results.get(self.selected_result_index),
            RunState::Idle | RunState::Running => None,
        }
    }
}
