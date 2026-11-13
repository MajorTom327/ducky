use crate::FilesError;

pub fn normalize_file_name(raw_name: &str, extension: &str) -> Result<String, FilesError> {
    let trimmed_name = raw_name.trim();
    if trimmed_name.is_empty()
        || trimmed_name.starts_with('.')
        || trimmed_name.contains(['/', '\\', '\0'])
    {
        return Err(FilesError::InvalidName(raw_name.to_owned()));
    }
    let suffix = format!(".{extension}");
    if trimmed_name.ends_with(&suffix) {
        Ok(trimmed_name.to_owned())
    } else {
        Ok(format!("{trimmed_name}{suffix}"))
    }
}

pub(crate) fn list_entries(
    dir: &std::path::Path,
    extension: &str,
) -> Result<Vec<ducky_core::FileEntry>, FilesError> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut entries = Vec::new();
    for dir_entry in read_dir {
        let dir_entry = dir_entry?;
        let metadata = dir_entry.metadata()?;
        let path = dir_entry.path();
        if !metadata.is_file()
            || path.extension().and_then(|value| value.to_str()) != Some(extension)
        {
            continue;
        }
        entries.push(ducky_core::FileEntry {
            name: dir_entry.file_name().to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
            modified_at: metadata.modified().ok(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_missing_extension_and_trims() {
        assert_eq!(
            normalize_file_name("  report ", "sql"),
            Ok("report.sql".to_owned())
        );
        assert_eq!(
            normalize_file_name("report.sql", "sql"),
            Ok("report.sql".to_owned())
        );
        assert_eq!(
            normalize_file_name("db.dump", "dump"),
            Ok("db.dump".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_names() {
        for raw_name in ["", "   ", "a/b", "a\\b", "a\0b", ".hidden", "../x"] {
            assert!(matches!(
                normalize_file_name(raw_name, "sql"),
                Err(FilesError::InvalidName(_))
            ));
        }
    }
}
