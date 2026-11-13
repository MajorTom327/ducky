use ducky_core::{Cell, ResultSet, StatementOutcome};
use futures::{TryStreamExt, pin_mut};
use tokio_postgres::types::FromSql;
use tokio_postgres::{Client, Row, SimpleQueryMessage};

use crate::error::{PgError, query_error};

pub const MAX_RESULT_ROWS: usize = 10_000;

pub(crate) async fn run_statement(
    client: &Client,
    sql: &str,
) -> Result<StatementOutcome, tokio_postgres::Error> {
    let message_stream = client.simple_query_raw(sql).await?;
    pin_mut!(message_stream);
    let mut result_set: Option<ResultSet> = None;
    let mut affected_row_count = 0;
    while let Some(message) = message_stream.try_next().await? {
        match message {
            SimpleQueryMessage::RowDescription(columns) => {
                result_set = Some(ResultSet {
                    columns: columns
                        .iter()
                        .map(|column| column.name().to_owned())
                        .collect(),
                    rows: Vec::new(),
                    is_truncated: false,
                });
            }
            SimpleQueryMessage::Row(row) => {
                if let Some(result_set) = result_set.as_mut() {
                    if result_set.rows.len() < MAX_RESULT_ROWS {
                        let cells: Vec<Cell> = (0..row.len())
                            .map(|column_index| row.get(column_index).map(str::to_owned))
                            .collect();
                        result_set.rows.push(cells);
                    } else {
                        result_set.is_truncated = true;
                    }
                }
            }
            SimpleQueryMessage::CommandComplete(row_count) => affected_row_count = row_count,
            _ => {}
        }
    }
    Ok(match result_set {
        Some(result_set) => StatementOutcome::Rows(result_set),
        None => StatementOutcome::Affected(affected_row_count),
    })
}

pub(crate) fn column_value<'a, T: FromSql<'a>>(
    row: &'a Row,
    column_index: usize,
) -> Result<T, PgError> {
    row.try_get(column_index).map_err(query_error)
}
