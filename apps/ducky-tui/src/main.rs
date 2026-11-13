use std::io::stdout;
use std::process::ExitCode;

use clap::Parser;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, EventStream};
use crossterm::execute;
use ducky_app::{
    AppState, Msg, ProfilePicker, ProfileRow, ProfilesMsg, Screen, profile_kind_name, profile_rows,
};
use ducky_config::{Config, ConfigError, SearchContext, load, resolve_config_path};
use ducky_core::Profile;
use ducky_runtime::Runtime;
use ducky_tui::cli::Args;
use ducky_tui::event_loop::{self, TerminalIo};
use ducky_tui::profile_lookup::{ProfileLookupError, resolve_profile};
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
        Some(requested_profile) => match resolve_profile(&config.profiles, requested_profile) {
            Ok(profile_index) => Some(profile_index),
            Err(ProfileLookupError::NotFound) => {
                print_unknown_profile(requested_profile, &config);
                return Ok(ExitCode::from(2));
            }
            Err(ProfileLookupError::Ambiguous {
                matching_profile_indexes,
            }) => {
                eprintln!(
                    "ducky: `{requested_profile}` matches several profiles in {} (use PROJECT/LABEL or PROJECT/TYPE)",
                    config.source.display()
                );
                eprintln!("Matching profiles:");
                print_profile_list(&config.profiles, &matching_profile_indexes);
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
            let terminal_io = TerminalIo {
                terminal: &mut terminal,
                events: EventStream::new(),
                cursor_writer: stdout(),
            };
            let outcome =
                event_loop::run(terminal_io, state, tui, runtime, receiver, startup_messages).await;
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

fn print_unknown_profile(requested_profile: &str, config: &Config) {
    eprintln!(
        "ducky: no profile `{requested_profile}` in {} (use LABEL, PROJECT/LABEL or PROJECT/TYPE)",
        config.source.display()
    );
    eprintln!("Available profiles:");
    let all_profile_indexes: Vec<usize> = (0..config.profiles.len()).collect();
    print_profile_list(&config.profiles, &all_profile_indexes);
}

fn print_profile_list(profiles: &[Profile], listed_profile_indexes: &[usize]) {
    let rows = profile_rows(profiles, "");
    let has_project_headers = rows.iter().any(|row| matches!(row, ProfileRow::Project(_)));
    let mut pending_project_header = None;
    for row in rows {
        match row {
            ProfileRow::Project(project_name) => pending_project_header = Some(project_name),
            ProfileRow::Profile(profile_index) => {
                if !listed_profile_indexes.contains(&profile_index) {
                    continue;
                }
                let Some(profile) = profiles.get(profile_index) else {
                    continue;
                };
                if let Some(project_name) = pending_project_header.take() {
                    eprintln!("  {project_name}");
                }
                let indent = if has_project_headers { "    " } else { "  " };
                eprintln!(
                    "{indent}{} ({}, {})",
                    profile.display_name(),
                    profile_kind_name(profile.kind),
                    profile.target()
                );
            }
        }
    }
}
