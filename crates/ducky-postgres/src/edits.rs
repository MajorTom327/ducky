use tokio_postgres::{Client, SimpleQueryMessage};

use crate::error::{PgError, query_error};

pub(crate) async fn apply_edits(client: &Client, sql: &str) -> Result<Vec<u64>, PgError> {
    let messages = client.simple_query(sql).await.map_err(query_error)?;
    Ok(messages
        .into_iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::CommandComplete(updated_row_count) => Some(updated_row_count),
            _ => None,
        })
        .collect())
}
