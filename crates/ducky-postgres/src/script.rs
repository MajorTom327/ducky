use std::time::{Duration, Instant};

use ducky_core::{ScriptFailure, ScriptReport, StatementResult, TransactionMode};
use tokio_postgres::Client;

use crate::error::describe;
use crate::rows::run_statement;

pub(crate) async fn run_script(
    client: &Client,
    statements: &[String],
    mode: TransactionMode,
) -> ScriptReport {
    let started_at = Instant::now();
    let mut report = ScriptReport {
        mode,
        results: Vec::new(),
        failure: None,
        elapsed: Duration::ZERO,
    };
    if statements.is_empty() {
        return report;
    }
    let is_transaction = mode == TransactionMode::Transaction;
    if is_transaction && let Err(error) = client.batch_execute("BEGIN").await {
        report.failure = Some(ScriptFailure {
            statement_index: 0,
            sql: "BEGIN".to_owned(),
            message: describe(&error),
        });
        report.elapsed = started_at.elapsed();
        return report;
    }
    for (statement_index, sql) in statements.iter().enumerate() {
        let statement_started_at = Instant::now();
        match run_statement(client, sql).await {
            Ok(outcome) => report.results.push(StatementResult {
                sql: sql.clone(),
                outcome,
                elapsed: statement_started_at.elapsed(),
            }),
            Err(error) => {
                report.failure = Some(ScriptFailure {
                    statement_index,
                    sql: sql.clone(),
                    message: describe(&error),
                });
                break;
            }
        }
    }
    if is_transaction {
        finish_transaction(client, statements.len(), &mut report).await;
    }
    report.elapsed = started_at.elapsed();
    report
}

async fn finish_transaction(client: &Client, statement_count: usize, report: &mut ScriptReport) {
    match report.failure.as_mut() {
        Some(failure) => {
            if let Err(error) = client.batch_execute("ROLLBACK").await {
                failure.message.push_str("\nROLLBACK failed: ");
                failure.message.push_str(&describe(&error));
            }
        }
        None => {
            if let Err(error) = client.batch_execute("COMMIT").await {
                report.failure = Some(ScriptFailure {
                    statement_index: statement_count,
                    sql: "COMMIT".to_owned(),
                    message: describe(&error),
                });
            }
        }
    }
}
