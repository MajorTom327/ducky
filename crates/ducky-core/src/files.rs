use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Available(PathBuf),
    Missing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgTools {
    pub pg_dump: ToolStatus,
    pub pg_restore: ToolStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestoreOptions {
    pub clean_before_restore: bool,
    pub skip_ownership: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePaths {
    pub sql_dir: PathBuf,
    pub dump_dir: PathBuf,
}
