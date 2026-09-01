mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mastering",
    about = "AI-powered music mastering CLI",
    version,
    author = "KeyhanStudio"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Master an audio track
    Master(commands::master::MasterArgs),

    /// Master an ordered group of tracks with album-level loudness continuity
    Album(commands::album::AlbumArgs),

    /// Analyze an audio file (loudness, spectrum, dynamics)
    Analyze(commands::analyze::AnalyzeArgs),

    /// Show or initialize configuration
    Config(commands::config::ConfigArgs),

    /// List available backends and check their status
    Backends,

    /// Qualify loudness and true-peak meters against approved vectors
    Qualify(commands::qualify::QualifyArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(false)
        .without_time()
        .init();

    match cli.command {
        Commands::Master(args) => commands::master::run(args).await,
        Commands::Album(args) => commands::album::run(args).await,
        Commands::Analyze(args) => commands::analyze::run(args).await,
        Commands::Config(args) => commands::config::run(args),
        Commands::Backends => commands::backends::run().await,
        Commands::Qualify(args) => commands::qualify::run(args).await,
    }
}
