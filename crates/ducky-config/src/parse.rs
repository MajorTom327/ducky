use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ducky_core::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, Profile, ProfileKind, StoragePaths};
use serde::Deserialize;

use crate::ConfigError;
use crate::env::split_url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub source: PathBuf,
    pub profiles: Vec<Profile>,
    pub storage: StoragePaths,
    pub page_size: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    sql_dir: Option<PathBuf>,
    dump_dir: Option<PathBuf>,
    page_size: Option<usize>,
    #[serde(default)]
    profiles: Vec<RawProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    label: String,
    url: String,
    #[serde(rename = "type")]
    kind: RawProfileKind,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawProfileKind {
    Local,
    Staging,
    Prod,
}

impl From<RawProfileKind> for ProfileKind {
    fn from(raw_kind: RawProfileKind) -> ProfileKind {
        match raw_kind {
            RawProfileKind::Local => ProfileKind::Local,
            RawProfileKind::Staging => ProfileKind::Staging,
            RawProfileKind::Prod => ProfileKind::Prod,
        }
    }
}

pub fn load(config_path: &Path) -> Result<Config, ConfigError> {
    let file_text = std::fs::read_to_string(config_path).map_err(|error| ConfigError::Read {
        path: config_path.to_path_buf(),
        reason: error.to_string(),
    })?;
    let raw_config: RawConfig = toml::from_str(&file_text).map_err(|error| ConfigError::Parse {
        path: config_path.to_path_buf(),
        reason: error.to_string(),
    })?;
    if raw_config.profiles.is_empty() {
        return Err(ConfigError::NoProfiles {
            path: config_path.to_path_buf(),
        });
    }
    let mut seen_labels = HashSet::new();
    let mut profiles = Vec::with_capacity(raw_config.profiles.len());
    for (index, raw_profile) in raw_config.profiles.into_iter().enumerate() {
        let label = raw_profile.label.trim().to_owned();
        if label.is_empty() {
            return Err(ConfigError::EmptyLabel {
                profile_position: index + 1,
            });
        }
        if !seen_labels.insert(label.clone()) {
            return Err(ConfigError::DuplicateLabel { label });
        }
        validate_url(&label, &raw_profile.url)?;
        profiles.push(Profile {
            label,
            url: raw_profile.url,
            kind: raw_profile.kind.into(),
        });
    }
    let page_size = raw_config.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(ConfigError::InvalidPageSize { page_size });
    }
    let base_dir = config_path.parent().unwrap_or(Path::new(""));
    let storage = StoragePaths {
        sql_dir: base_dir.join(raw_config.sql_dir.unwrap_or_else(|| PathBuf::from("sql"))),
        dump_dir: base_dir.join(
            raw_config
                .dump_dir
                .unwrap_or_else(|| PathBuf::from("dumps")),
        ),
    };
    Ok(Config {
        source: config_path.to_path_buf(),
        profiles,
        storage,
        page_size,
    })
}

fn validate_url(label: &str, url: &str) -> Result<(), ConfigError> {
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        return Err(ConfigError::InvalidUrl {
            label: label.to_owned(),
            reason: "must start with postgres:// or postgresql://".to_owned(),
        });
    }
    split_url(url)
        .map(|_| ())
        .map_err(|reason| ConfigError::InvalidUrl {
            label: label.to_owned(),
            reason,
        })
}

#[cfg(test)]
mod tests;
