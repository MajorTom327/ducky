use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_ducky");

fn write_config(config_dir: &TempDir, content: &str) -> std::path::PathBuf {
    let config_path = config_dir.path().join("ducky.toml");
    std::fs::write(&config_path, content).expect("writes the config file");
    config_path
}

fn run_ducky(working_dir: &Path, arguments: &[&str]) -> Output {
    Command::new(BINARY)
        .args(arguments)
        .current_dir(working_dir)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", working_dir)
        .output()
        .expect("runs the ducky binary")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

const PROJECT_CONFIG: &str = r#"
[[profiles]]
label = "Shop local"
project = "shop"
type = "local"
url = "postgres://postgres@localhost:5432/shop"

[[profiles]]
project = "shop"
type = "prod"
url = "postgres://app@prod.example.com:5432/shop"

[[profiles]]
label = "Analytics local"
project = "analytics"
type = "local"
url = "postgres://postgres@localhost:5432/analytics"
"#;

#[test]
fn missing_explicit_config_exits_1_with_searched_path_and_example() {
    let working_dir = tempfile::tempdir().expect("creates a temporary directory");
    let missing_path = working_dir.path().join("absent.toml");
    let output = run_ducky(
        working_dir.path(),
        &["--config", missing_path.to_str().expect("utf-8 path")],
    );
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr.starts_with("ducky: "));
    assert!(stderr.contains("Searched paths:"));
    assert!(stderr.contains(&format!("  {}", missing_path.display())));
    assert!(stderr.contains("Example configuration (ducky.example.toml):"));
    assert!(stderr.contains(include_str!("../../../ducky.example.toml")));
}

#[test]
fn no_config_anywhere_lists_every_searched_location() {
    let working_dir = tempfile::tempdir().expect("creates a temporary directory");
    let output = run_ducky(working_dir.path(), &[]);
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(1));
    let local_position = stderr
        .find(&working_dir.path().join("ducky.toml").display().to_string())
        .expect("lists ./ducky.toml");
    let home_position = stderr
        .find(
            &working_dir
                .path()
                .join(".config/ducky/config.toml")
                .display()
                .to_string(),
        )
        .expect("lists the home config path");
    assert!(local_position < home_position);
}

#[test]
fn invalid_config_exits_1_without_searched_paths() {
    let config_dir = tempfile::tempdir().expect("creates a temporary directory");
    let config_path = write_config(&config_dir, "profiles = []\n");
    let output = run_ducky(
        config_dir.path(),
        &["--config", config_path.to_str().expect("utf-8 path")],
    );
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains(&config_path.display().to_string()));
    assert!(!stderr.contains("Searched paths:"));
    assert!(stderr.contains("Example configuration (ducky.example.toml):"));
}

#[test]
fn unknown_profile_exits_2_listing_every_profile_by_project() {
    let config_dir = tempfile::tempdir().expect("creates a temporary directory");
    let config_path = write_config(&config_dir, PROJECT_CONFIG);
    let output = run_ducky(
        config_dir.path(),
        &[
            "--config",
            config_path.to_str().expect("utf-8 path"),
            "--profile",
            "nope",
        ],
    );
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains(&format!(
        "ducky: no profile `nope` in {}",
        config_path.display()
    )));
    assert!(stderr.contains("Available profiles:"));
    assert!(stderr.ends_with(
        "Available profiles:\n  shop\n    Shop local (local, localhost:5432/shop)\n    shop prod (prod, prod.example.com:5432/shop)\n  analytics\n    Analytics local (local, localhost:5432/analytics)\n"
    ));
    assert!(!stderr.contains("Example configuration"));
}

#[test]
fn ambiguous_profile_exits_2_listing_only_the_matches() {
    let config_dir = tempfile::tempdir().expect("creates a temporary directory");
    let config_path = write_config(
        &config_dir,
        r#"
[[profiles]]
label = "Dev"
type = "local"
url = "postgres://postgres@localhost:5432/one"

[[profiles]]
label = "Dev"
type = "staging"
url = "postgres://postgres@localhost:5432/two"

[[profiles]]
label = "Other"
type = "prod"
url = "postgres://postgres@localhost:5432/three"
"#,
    );
    let output = run_ducky(
        config_dir.path(),
        &[
            "--config",
            config_path.to_str().expect("utf-8 path"),
            "--profile",
            "Dev",
        ],
    );
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains(&format!(
        "ducky: `Dev` matches several profiles in {} (use PROJECT/LABEL or PROJECT/TYPE)",
        config_path.display()
    )));
    assert!(stderr.ends_with(
        "Matching profiles:\n  Dev (local, localhost:5432/one)\n  Dev (staging, localhost:5432/two)\n"
    ));
}

#[test]
fn ambiguous_project_type_lists_only_the_matching_project() {
    let config_dir = tempfile::tempdir().expect("creates a temporary directory");
    let config_path = write_config(
        &config_dir,
        &format!(
            "{PROJECT_CONFIG}{}",
            r#"
[[profiles]]
label = "Shop replica"
project = "shop"
type = "local"
url = "postgres://postgres@localhost:5433/shop"
"#
        ),
    );
    let output = run_ducky(
        config_dir.path(),
        &[
            "--config",
            config_path.to_str().expect("utf-8 path"),
            "--profile",
            "shop/local",
        ],
    );
    let stderr = stderr_text(&output);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.ends_with(
        "Matching profiles:\n  shop\n    Shop local (local, localhost:5432/shop)\n    Shop replica (local, localhost:5433/shop)\n"
    ));
    assert!(!stderr.contains("analytics"));
}
