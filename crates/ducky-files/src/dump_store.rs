use std::path::PathBuf;

use ducky_core::FileEntry;

use crate::FilesError;
use crate::names::{list_entries, normalize_file_name};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpStore {
    dump_dir: PathBuf,
}

impl DumpStore {
    pub fn new(dump_dir: PathBuf) -> DumpStore {
        DumpStore { dump_dir }
    }

    pub fn list(&self) -> Result<Vec<FileEntry>, FilesError> {
        let mut entries = list_entries(&self.dump_dir, "dump")?;
        entries.sort_by(|left, right| {
            right
                .modified_at
                .cmp(&left.modified_at)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(entries)
    }

    pub fn path_for(&self, file_name: &str) -> Result<PathBuf, FilesError> {
        Ok(self.dump_dir.join(normalize_file_name(file_name, "dump")?))
    }
}

pub fn default_dump_name(database: &str, now: chrono::DateTime<chrono::Local>) -> String {
    format!("{database}-{}.dump", now.format("%Y%m%d-%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::time::{Duration, SystemTime};

    #[test]
    fn lists_dumps_newest_first() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = DumpStore::new(temp_dir.path().to_path_buf());
        let now = SystemTime::now();
        for (name, age_seconds) in [("old.dump", 100), ("new.dump", 0), ("mid.dump", 50)] {
            let file = std::fs::File::create(temp_dir.path().join(name)).unwrap();
            file.set_modified(now - Duration::from_secs(age_seconds))
                .unwrap();
        }
        std::fs::write(temp_dir.path().join("x.sql"), "").unwrap();
        let names: Vec<String> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, vec!["new.dump", "mid.dump", "old.dump"]);
    }

    #[test]
    fn missing_directory_lists_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = DumpStore::new(temp_dir.path().join("absent"));
        assert_eq!(store.list().unwrap(), Vec::new());
    }

    #[test]
    fn path_for_normalizes_name() {
        let store = DumpStore::new(PathBuf::from("/d"));
        assert_eq!(store.path_for("x").unwrap(), PathBuf::from("/d/x.dump"));
        assert!(store.path_for("../x").is_err());
    }

    #[test]
    fn default_name_has_timestamp() {
        let now = chrono::Local.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(default_dump_name("shop", now), "shop-20260102-030405.dump");
    }
}
