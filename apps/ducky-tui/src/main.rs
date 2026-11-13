use std::io::stdout;
use std::process::ExitCode;

use clap::Parser;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use ducky_app::{
    AppState, Msg, ProfilePicker, ProfileRow, ProfilesMsg, Screen, profile_kind_name, profile_rows,
};
use ducky_config::{Config, ConfigError, SearchContext, load, resolve_config_path};
use ducky_core::Profile;
use ducky_runtime::Runtime;
use ducky_tui::cli::Args;
use ducky_tui::event_loop;
use ducky_tui::tui_state::TuiState;

const EXAMPLE_CONFIG: &str = include_str!("../../../ducky.example.toml");

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();
    let config = match load_config(&args) {
        Ok(config) => config,
        Err(error) => {
            print_config_error(&error);
            return Ok(ExitCode::from(1));
        }
    };
    let startup_profile_index = match &args.profile {
        None => None,
        Some(requested_profile) => match find_profile(&config.profiles, requested_profile) {
            Some(profile_index) => Some(profile_index),
            None => {
                print_unknown_profile(requested_profile, &config);
                return Ok(ExitCode::from(2));
            }
        },
    };
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let runtime = Runtime::new(config.storage.clone(), sender);
    let mut state = AppState::new(config.profiles, config.storage).with_page_size(config.page_size);
    let startup_messages = match startup_profile_index {
        None => Vec::new(),
        Some(profile_index) => {
            let picker = ProfilePicker::new(&state.profiles, Some(profile_index));
            state.screen = Screen::Profiles(picker);
            vec![Msg::Profiles(ProfilesMsg::Connect)]
        }
    };
    let tui = TuiState::new(!args.no_effects);
    let mut terminal = ratatui::init();
    let outcome = match execute!(stdout(), EnableBracketedPaste) {
        Ok(()) => {
            let outcome = event_loop::run(
                &mut terminal,
                state,
                tui,
                runtime,
                receiver,
                startup_messages,
            )
            .await;
            let terminal_reset = execute!(
                stdout(),
                DisableBracketedPaste,
                SetCursorStyle::DefaultUserShape
            );
            outcome.and(terminal_reset.map_err(anyhow::Error::from))
        }
        Err(error) => Err(error.into()),
    };
    ratatui::restore();
    outcome.map(|()| ExitCode::SUCCESS)
}

fn load_config(args: &Args) -> Result<Config, ConfigError> {
    let context =
        SearchContext::from_process(args.config.clone()).map_err(|error| ConfigError::Read {
            path: args.config.clone().unwrap_or_default(),
            reason: error.to_string(),
        })?;
    let config_path = resolve_config_path(&context)?;
    load(&config_path)
}

fn print_config_error(error: &ConfigError) {
    eprintln!("ducky: {error}");
    if let ConfigError::NotFound { searched_paths } = error {
        eprintln!("\nSearched paths:");
        for searched_path in searched_paths {
            eprintln!("  {}", searched_path.display());
        }
    }
    eprintln!("\nExample configuration (ducky.example.toml):\n\n{EXAMPLE_CONFIG}");
}

fn find_profile(profiles: &[Profile], requested_profile: &str) -> Option<usize> {
    profiles
        .iter()
        .position(|profile| profile.label == requested_profile)
        .or_else(|| {
            let (project_name, label) = requested_profile.split_once('/')?;
            profiles.iter().position(|profile| {
                profile.label == label && profile.project.as_deref() == Some(project_name)
            })
        })
}

fn print_unknown_profile(requested_profile: &str, config: &Config) {
    eprintln!(
        "ducky: no profile `{requested_profile}` in {} (use LABEL or PROJECT/LABEL)",
        config.source.display()
    );
    eprintln!("Available profiles:");
    let rows = profile_rows(&config.profiles, "");
    let has_project_headers = rows.iter().any(|row| matches!(row, ProfileRow::Project(_)));
    for row in rows {
        match row {
            ProfileRow::Project(project_name) => eprintln!("  {project_name}"),
            ProfileRow::Profile(profile_index) => {
                let Some(profile) = config.profiles.get(profile_index) else {
                    continue;
                };
                let indent = if has_project_headers { "    " } else { "  " };
                eprintln!(
                    "{indent}{} ({}, {})",
                    profile.label,
                    profile_kind_name(profile.kind),
                    profile.target()
                );
            }
        }
    }
}
