use std::fs;
use std::path::{Path, PathBuf};

use super::*;

fn context(root: &Path, explicit_path: Option<PathBuf>, has_xdg: bool) -> SearchContext {
    SearchContext {
        explicit_path,
        working_dir: root.join("work"),
        xdg_config_home: has_xdg.then(|| root.join("xdg")),
        home_dir: Some(root.join("home")),
    }
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn explicit_path_wins() {
    let dir = tempfile::tempdir().unwrap();
    let explicit = dir.path().join("custom.toml");
    touch(&explicit);
    touch(&dir.path().join("work/ducky.toml"));
    let found = resolve_config_path(&context(dir.path(), Some(explicit.clone()), true)).unwrap();
    assert_eq!(found, explicit);
}

#[test]
fn missing_explicit_path_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let explicit = dir.path().join("missing.toml");
    touch(&dir.path().join("work/ducky.toml"));
    let error =
        resolve_config_path(&context(dir.path(), Some(explicit.clone()), true)).unwrap_err();
    assert_eq!(
        error,
        ConfigError::NotFound {
            searched_paths: vec![explicit]
        }
    );
}

#[test]
fn working_dir_first() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("work/ducky.toml"));
    touch(&dir.path().join("xdg/ducky/config.toml"));
    let found = resolve_config_path(&context(dir.path(), None, true)).unwrap();
    assert_eq!(found, dir.path().join("work/ducky.toml"));
}

#[test]
fn xdg_config_home_second() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("xdg/ducky/config.toml"));
    touch(&dir.path().join("home/.config/ducky/config.toml"));
    let found = resolve_config_path(&context(dir.path(), None, true)).unwrap();
    assert_eq!(found, dir.path().join("xdg/ducky/config.toml"));
}

#[test]
fn home_config_when_xdg_unset() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("home/.config/ducky/config.toml"));
    let found = resolve_config_path(&context(dir.path(), None, false)).unwrap();
    assert_eq!(found, dir.path().join("home/.config/ducky/config.toml"));
}

#[test]
fn not_found_lists_paths_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let with_xdg = resolve_config_path(&context(dir.path(), None, true)).unwrap_err();
    assert_eq!(
        with_xdg,
        ConfigError::NotFound {
            searched_paths: vec![
                dir.path().join("work/ducky.toml"),
                dir.path().join("xdg/ducky/config.toml"),
            ]
        }
    );
    let without_xdg = resolve_config_path(&context(dir.path(), None, false)).unwrap_err();
    assert_eq!(
        without_xdg,
        ConfigError::NotFound {
            searched_paths: vec![
                dir.path().join("work/ducky.toml"),
                dir.path().join("home/.config/ducky/config.toml"),
            ]
        }
    );
}
