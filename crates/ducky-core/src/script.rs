use std::time::Duration;

use crate::data::ResultSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransactionMode {
    #[default]
    Transaction,
    AutoCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementOutcome {
    Rows(ResultSet),
    Affected(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementResult {
    pub sql: String,
    pub outcome: StatementOutcome,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFailure {
    pub statement_index: usize,
    pub sql: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptReport {
    pub mode: TransactionMode,
    pub results: Vec<StatementResult>,
    pub failure: Option<ScriptFailure>,
    pub elapsed: Duration,
}
