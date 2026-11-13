use std::path::PathBuf;

use crate::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchContext {
    pub explicit_path: Option<PathBuf>,
    pub working_dir: PathBuf,
    pub xdg_config_home: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
}

impl SearchContext {
    pub fn from_process(explicit_path: Option<PathBuf>) -> std::io::Result<SearchContext> {
        Ok(SearchContext {
            explicit_path,
            working_dir: std::env::current_dir()?,
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            home_dir: std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
        })
    }
}

pub fn resolve_config_path(context: &SearchContext) -> Result<PathBuf, ConfigError> {
    if let Some(explicit_path) = &context.explicit_path {
        return if explicit_path.is_file() {
            Ok(explicit_path.clone())
        } else {
            Err(ConfigError::NotFound {
                searched_paths: vec![explicit_path.clone()],
            })
        };
    }
    let mut candidate_paths = vec![context.working_dir.join("ducky.toml")];
    match (&context.xdg_config_home, &context.home_dir) {
        (Some(xdg_config_home), _) => {
            candidate_paths.push(xdg_config_home.join("ducky").join("config.toml"))
        }
        (None, Some(home_dir)) => {
            candidate_paths.push(home_dir.join(".config").join("ducky").join("config.toml"))
        }
        (None, None) => {}
    }
    match candidate_paths.iter().find(|path| path.is_file()) {
        Some(found_path) => Ok(found_path.clone()),
        None => Err(ConfigError::NotFound {
            searched_paths: candidate_paths,
        }),
    }
}

#[cfg(test)]
mod tests;
