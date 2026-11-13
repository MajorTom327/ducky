use std::io::stdout;
use std::process::ExitCode;

use clap::Parser;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use ducky_app::{AppState, Msg, ProfilesMsg};
use ducky_config::{Config, ConfigError, SearchContext, load, resolve_config_path};
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
    let startup_messages = match &args.profile {
        None => Vec::new(),
        Some(profile_label) => {
            let Some(profile_index) = config
                .profiles
                .iter()
                .position(|profile| &profile.label == profile_label)
            else {
                print_unknown_profile(profile_label, &config);
                return Ok(ExitCode::from(2));
            };
            vec![
                Msg::Profiles(ProfilesMsg::Select(profile_index)),
                Msg::Profiles(ProfilesMsg::Connect),
            ]
        }
    };
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let runtime = Runtime::new(config.storage.clone(), sender);
    let state = AppState::new(config.profiles, config.storage).with_page_size(config.page_size);
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

fn print_unknown_profile(profile_label: &str, config: &Config) {
    eprintln!(
        "ducky: no profile labelled `{profile_label}` in {}",
        config.source.display()
    );
    eprintln!("Available profiles:");
    for profile in &config.profiles {
        eprintln!("  {}", profile.label);
    }
}
