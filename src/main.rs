use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Game directory and launcher initializer", long_about = None)]
struct Args {
    /// Path to target executable (.exe or Linux binary)
    #[arg(required = true)]
    target: PathBuf,

    /// Display name of the game (optional)
    #[arg(short, long)]
    name: Option<String>,

    /// Overwrite existing game.desktop / .directory files if present
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// Increase log verbosity (debug level)
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let default_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_target(false)
        .format_timestamp(None)
        .init();

    kalesa::run_setup(&args.target, args.name, args.force)?;

    Ok(())
}
