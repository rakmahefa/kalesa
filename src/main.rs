use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Game directory and launcher initializer", long_about = None)]
struct Args {
    /// Path to target executable (.exe, Linux binary or .AppImage)
    #[arg(required = true)]
    target: PathBuf,

    /// Display name of the game
    #[arg(short, long)]
    name: Option<String>,

    /// Developer or studio name
    #[arg(long)]
    developer: Option<String>,

    /// Game version
    #[arg(long)]
    version: Option<String>,

    /// Short game description
    #[arg(long)]
    description: Option<String>,

    /// Desktop category; can be supplied multiple times
    #[arg(long = "category")]
    categories: Vec<String>,

    /// Explicit icon path
    #[arg(long)]
    icon: Option<PathBuf>,

    /// Runner backend: auto, native, wine or proton
    #[arg(long, default_value = "auto")]
    runner: kalesa::RunnerBackend,

    /// Wine prefix used by Wine/Proton
    #[arg(long)]
    wine_prefix: Option<PathBuf>,

    /// Proton executable used by the Proton runner
    #[arg(long)]
    proton_path: Option<PathBuf>,

    /// Additional argument baked into the generated launcher; repeatable
    #[arg(long = "arg")]
    args: Vec<String>,

    /// Environment assignment (KEY=VALUE); repeatable
    #[arg(long = "env")]
    env: Vec<String>,

    /// Overwrite existing game.desktop / .directory files
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// Increase log verbosity
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn parse_env(entries: Vec<String>) -> kalesa::error::Result<Vec<(String, String)>> {
    entries
        .into_iter()
        .map(|entry| {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| kalesa::error::KalesaError::InvalidEnvironmentKey(entry.clone()))?;
            Ok((key.to_string(), value.to_string()))
        })
        .collect()
}

fn main() -> kalesa::error::Result<()> {
    let args = Args::parse();
    let default_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_target(false)
        .format_timestamp(None)
        .init();

    let setup_options = kalesa::pipeline::SetupOptions {
        name: args.name,
        developer: args.developer,
        version: args.version,
        description: args.description,
        categories: args.categories,
        icon: args.icon,
        runner: args.runner,
        wine_prefix: args.wine_prefix,
        proton_path: args.proton_path,
        launch: kalesa::LaunchOptions {
            args: args.args,
            env: parse_env(args.env)?,
        },
        force: args.force,
    };

    kalesa::run_setup_with_options(&args.target, setup_options)?;
    Ok(())
}
