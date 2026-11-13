use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "ducky",
    version,
    about = "Explore PostgreSQL databases from the terminal"
)]
pub struct Args {
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub no_effects: bool,
}
