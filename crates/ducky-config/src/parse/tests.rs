use std::fs;

use super::*;

fn load_text(text: &str) -> (tempfile::TempDir, Result<Config, ConfigError>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ducky.toml");
    fs::write(&path, text).unwrap();
    let result = load(&path);
    (dir, result)
}

fn profile(label: &str, url: &str, kind: &str) -> String {
    format!("[[profiles]]\nlabel = \"{label}\"\nurl = \"{url}\"\ntype = \"{kind}\"\n")
}

const URL: &str = "postgres://u@h/db";

#[test]
fn valid_file_with_three_kinds() {
    let text = [
        profile("A", URL, "local"),
        profile("B", "postgresql://u:${PW}@h/db", "staging"),
        profile("C", URL, "prod"),
    ]
    .concat();
    let (dir, result) = load_text(&text);
    let config = result.unwrap();
    let kinds: Vec<_> = config.profiles.iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        vec![ProfileKind::Local, ProfileKind::Staging, ProfileKind::Prod]
    );
    assert_eq!(config.profiles[1].url, "postgresql://u:${PW}@h/db");
    assert_eq!(config.source, dir.path().join("ducky.toml"));
    assert_eq!(config.storage.sql_dir, dir.path().join("sql"));
    assert_eq!(config.storage.dump_dir, dir.path().join("dumps"));
    assert_eq!(config.page_size, 50);
}

#[test]
fn page_size_is_read_from_the_file() {
    let (_dir, result) = load_text(&format!("page_size = 200\n{}", profile("A", URL, "local")));
    assert_eq!(result.unwrap().page_size, 200);
}

#[test]
fn page_size_outside_bounds_is_rejected() {
    for page_size in [0, 5_001] {
        let text = format!("page_size = {page_size}\n{}", profile("A", URL, "local"));
        let (_dir, result) = load_text(&text);
        assert_eq!(result, Err(ConfigError::InvalidPageSize { page_size }));
    }
}

#[test]
fn unknown_type_is_parse_error() {
    let (_dir, result) = load_text(&profile("A", URL, "dev"));
    assert!(matches!(result, Err(ConfigError::Parse { .. })));
}

#[test]
fn unknown_key_is_parse_error() {
    let (_dir, result) = load_text(&format!("color = 1\n{}", profile("A", URL, "local")));
    assert!(matches!(result, Err(ConfigError::Parse { .. })));
}

#[test]
fn missing_file_is_read_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        load(&dir.path().join("x.toml")),
        Err(ConfigError::Read { .. })
    ));
}

#[test]
fn no_profiles() {
    let (dir, result) = load_text("sql_dir = \"q\"\n");
    assert_eq!(
        result.unwrap_err(),
        ConfigError::NoProfiles {
            path: dir.path().join("ducky.toml")
        }
    );
}

#[test]
fn empty_label() {
    let text = [profile("A", URL, "local"), profile("  ", URL, "local")].concat();
    let (_dir, result) = load_text(&text);
    assert_eq!(
        result.unwrap_err(),
        ConfigError::EmptyLabel {
            profile_position: 2
        }
    );
}

#[test]
fn duplicate_label() {
    let text = [profile("A", URL, "local"), profile(" A ", URL, "prod")].concat();
    let (_dir, result) = load_text(&text);
    assert_eq!(
        result.unwrap_err(),
        ConfigError::DuplicateLabel {
            label: "A".to_owned()
        }
    );
}

#[test]
fn invalid_scheme() {
    let (_dir, result) = load_text(&profile("A", "mysql://h/db", "local"));
    assert!(matches!(result, Err(ConfigError::InvalidUrl { label, .. }) if label == "A"));
}

#[test]
fn unterminated_variable() {
    let (_dir, result) = load_text(&profile("A", "postgres://u:${PW@h/db", "local"));
    assert!(matches!(result, Err(ConfigError::InvalidUrl { .. })));
}

#[test]
fn invalid_variable_name() {
    let (_dir, result) = load_text(&profile("A", "postgres://u:${1PW}@h/db", "local"));
    assert!(matches!(result, Err(ConfigError::InvalidUrl { .. })));
}

#[test]
fn relative_and_absolute_storage_dirs() {
    let absolute = tempfile::tempdir().unwrap();
    let text = format!(
        "sql_dir = \"../queries\"\ndump_dir = \"{}\"\n{}",
        absolute.path().display(),
        profile("A", URL, "local")
    );
    let (dir, result) = load_text(&text);
    let config = result.unwrap();
    assert_eq!(config.storage.sql_dir, dir.path().join("../queries"));
    assert_eq!(config.storage.dump_dir, absolute.path());
}

#[test]
fn project_and_tags_are_normalised() {
    let text = format!(
        "{}project = \"  soiltrackers \"\ntags = [\" EU \", \"readonly\", \"eu\", \"ReadOnly\"]\n",
        profile("A", URL, "prod")
    );
    let (_dir, result) = load_text(&text);
    let config = result.unwrap();
    assert_eq!(config.profiles[0].project.as_deref(), Some("soiltrackers"));
    assert_eq!(config.profiles[0].tags, ["eu", "readonly"]);
}

#[test]
fn profiles_without_project_or_tags() {
    let text = [
        profile("A", URL, "local"),
        "project = \"   \"\n".to_owned(),
        profile("B", URL, "local"),
    ]
    .concat();
    let (_dir, result) = load_text(&text);
    let config = result.unwrap();
    for loaded_profile in &config.profiles {
        assert_eq!(loaded_profile.project, None);
        assert!(loaded_profile.tags.is_empty());
    }
}

#[test]
fn empty_tag_is_rejected() {
    let text = format!("{}tags = [\"eu\", \"  \"]\n", profile("A", URL, "local"));
    let (_dir, result) = load_text(&text);
    assert_eq!(
        result.unwrap_err(),
        ConfigError::EmptyTag {
            label: "A".to_owned()
        }
    );
}
