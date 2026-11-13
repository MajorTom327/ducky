use std::path::PathBuf;

use ducky_core::FileEntry;

use crate::FilesError;
use crate::names::{list_entries, normalize_file_name};

pub const MAX_EDITOR_FILE_BYTES: u64 = 5 * 1024 * 1024;
pub const PREVIEW_LINE_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlStore {
    sql_dir: PathBuf,
}

impl SqlStore {
    pub fn new(sql_dir: PathBuf) -> SqlStore {
        SqlStore { sql_dir }
    }

    pub fn list(&self) -> Result<Vec<FileEntry>, FilesError> {
        let mut entries = list_entries(&self.sql_dir, "sql")?;
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    pub fn load(&self, name: &str) -> Result<String, FilesError> {
        let file_name = normalize_file_name(name, "sql")?;
        let path = self.sql_dir.join(&file_name);
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => return Err(FilesError::NotFound(file_name)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(FilesError::NotFound(file_name));
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.len() > MAX_EDITOR_FILE_BYTES {
            return Err(FilesError::TooLarge {
                name: file_name,
                size_bytes: metadata.len(),
            });
        }
        Ok(std::fs::read_to_string(path)?)
    }

    pub fn preview(&self, name: &str) -> Result<String, FilesError> {
        let content = self.load(name)?;
        Ok(content
            .lines()
            .take(PREVIEW_LINE_LIMIT)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    pub fn save(
        &self,
        name: &str,
        content: &str,
        overwrite: bool,
    ) -> Result<FileEntry, FilesError> {
        let file_name = normalize_file_name(name, "sql")?;
        std::fs::create_dir_all(&self.sql_dir)?;
        let path = self.sql_dir.join(&file_name);
        if !overwrite && path.exists() {
            return Err(FilesError::AlreadyExists(file_name));
        }
        std::fs::write(&path, content)?;
        let metadata = std::fs::metadata(&path)?;
        Ok(FileEntry {
            name: file_name,
            size_bytes: metadata.len(),
            modified_at: metadata.modified().ok(),
        })
    }
}

#[cfg(test)]
mod tests;
