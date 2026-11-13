#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FilesError {
    #[error("Invalid file name: {0}")]
    InvalidName(String),
    #[error("{0} already exists")]
    AlreadyExists(String),
    #[error("{0} was not found")]
    NotFound(String),
    #[error("{name} is too large ({size_bytes} bytes)")]
    TooLarge { name: String, size_bytes: u64 },
    #[error("{0}")]
    Io(String),
    #[error("Invalid database URL: {0}")]
    InvalidUrl(String),
    #[error("Tool failed (exit code {}): {last_line}", exit_code.map_or_else(|| "none".to_owned(), |code| code.to_string()))]
    ToolFailed {
        exit_code: Option<i32>,
        last_line: String,
    },
}

impl From<std::io::Error> for FilesError {
    fn from(error: std::io::Error) -> FilesError {
        FilesError::Io(error.to_string())
    }
}
