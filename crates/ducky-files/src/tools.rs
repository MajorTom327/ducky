use std::ffi::OsStr;
use std::path::Path;

use ducky_core::{PgTools, ToolStatus};

pub fn detect_tools(path_variable: Option<&OsStr>) -> PgTools {
    PgTools {
        pg_dump: find_tool("pg_dump", path_variable),
        pg_restore: find_tool("pg_restore", path_variable),
    }
}

fn find_tool(tool_name: &str, path_variable: Option<&OsStr>) -> ToolStatus {
    path_variable
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(tool_name))
        .find(|candidate| is_executable(candidate))
        .map_or_else(
            || ToolStatus::Missing(format!("`{tool_name}` was not found in PATH")),
            ToolStatus::Available,
        )
}

#[cfg(unix)]
fn is_executable(candidate: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(candidate)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(candidate: &Path) -> bool {
    std::fs::metadata(candidate).is_ok_and(|metadata| metadata.is_file())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn detects_only_executable_present_tools() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pg_dump_path = temp_dir.path().join("pg_dump");
        std::fs::write(&pg_dump_path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&pg_dump_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(temp_dir.path().join("pg_restore"), "").unwrap();
        let path_variable = std::env::join_paths([temp_dir.path()]).unwrap();
        let tools = detect_tools(Some(&path_variable));
        assert_eq!(tools.pg_dump, ToolStatus::Available(pg_dump_path));
        assert_eq!(
            tools.pg_restore,
            ToolStatus::Missing("`pg_restore` was not found in PATH".to_owned())
        );
    }

    #[test]
    fn no_path_means_both_missing() {
        let tools = detect_tools(None);
        assert!(matches!(tools.pg_dump, ToolStatus::Missing(_)));
        assert!(matches!(tools.pg_restore, ToolStatus::Missing(_)));
    }
}
