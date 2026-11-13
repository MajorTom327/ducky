use std::error::Error as _;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgError {
    #[error("{0}")]
    InvalidUrl(String),
    #[error("{0}")]
    Connect(String),
    #[error("{0}")]
    Query(String),
}

pub(crate) fn describe(error: &tokio_postgres::Error) -> String {
    match error.as_db_error() {
        Some(database_error) => {
            let mut description = format!(
                "{}: {}",
                database_error.severity(),
                database_error.message()
            );
            if let Some(detail) = database_error.detail() {
                description.push_str("\nDETAIL: ");
                description.push_str(detail);
            }
            if let Some(hint) = database_error.hint() {
                description.push_str("\nHINT: ");
                description.push_str(hint);
            }
            description
        }
        None => {
            let mut description = error.to_string();
            let mut source = error.source();
            while let Some(cause) = source {
                description.push_str(": ");
                description.push_str(&cause.to_string());
                source = cause.source();
            }
            description
        }
    }
}

pub(crate) fn query_error(error: tokio_postgres::Error) -> PgError {
    PgError::Query(describe(&error))
}
