use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "no configuration file found (searched: {})",
        display_paths(searched_paths)
    )]
    NotFound { searched_paths: Vec<PathBuf> },
    #[error("cannot read {}: {reason}", path.display())]
    Read { path: PathBuf, reason: String },
    #[error("cannot parse {}: {reason}", path.display())]
    Parse { path: PathBuf, reason: String },
    #[error("{} defines no profiles", path.display())]
    NoProfiles { path: PathBuf },
    #[error("profile #{profile_position} has an empty label")]
    EmptyLabel { profile_position: usize },
    #[error("profile label `{label}` is used more than once")]
    DuplicateLabel { label: String },
    #[error("profile `{label}` has an invalid url: {reason}")]
    InvalidUrl { label: String, reason: String },
    #[error("environment variable `{variable}` is not set")]
    MissingEnvVar { variable: String },
}

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
