use anyhow::Result;
use clap::Args;
use colored::Colorize;

use mastering_core::config::Config;

#[derive(Args)]
pub struct ConfigArgs {
    /// Initialize a new config file with defaults
    #[arg(long)]
    pub init: bool,

    /// Show the config file path
    #[arg(long)]
    pub path: bool,
}

pub fn run(args: ConfigArgs) -> Result<()> {
    if args.path {
        let path = Config::config_path()?;
        println!("{}", path.display());
        return Ok(());
    }

    if args.init {
        let config = Config::default();
        config.save()?;
        let path = Config::config_path()?;
        println!(
            "{} Config initialized at: {}",
            "OK".bold().green(),
            path.display()
        );
        return Ok(());
    }

    // Show current config
    let config = Config::load()?;
    let path = Config::config_path()?;

    println!(
        "\n{}  {}",
        "CONFIG".bold().cyan(),
        path.display().to_string().dimmed()
    );

    if !path.exists() {
        println!(
            "\n  {} No config file found. Run {} to create one.",
            "!".bold().yellow(),
            "mastering config --init".cyan()
        );
    }

    println!("\n{}", "General".bold().yellow());
    println!("  Default Backend:   {}", config.general.default_backend);
    println!("  Default Bit Depth: {}", config.general.default_bit_depth);
    println!("  Default Format:    {}", config.general.default_format);
    println!("  Target LUFS:       {:.1}", config.general.target_lufs);

    println!("\n{}", "AI".bold().yellow());
    println!("  Default Provider:  {}", config.ai.default_provider);
    println!("  Ollama Endpoint:   {}", config.ai.ollama.endpoint);
    println!("  Ollama Model:      {}", config.ai.ollama.model);
    println!(
        "  KeyhanStudio:      {}",
        if config.ai.keyhanstudio.endpoint.is_empty() {
            "not configured".dimmed().to_string()
        } else {
            config.ai.keyhanstudio.endpoint.clone()
        }
    );
    println!(
        "  OpenAI:            {}",
        if config.ai.openai.api_key.is_empty() {
            "not configured".dimmed().to_string()
        } else {
            format!("{} ({})", "configured".green(), config.ai.openai.model)
        }
    );
    println!(
        "  Anthropic:         {}",
        if config.ai.anthropic.api_key.is_empty() {
            "not configured".dimmed().to_string()
        } else {
            format!(
                "{} ({})",
                "configured".green(),
                config.ai.anthropic.model
            )
        }
    );

    println!("\n{}", "Backends".bold().yellow());
    println!(
        "  Matchering Python: {}",
        config.backends.matchering.python_path
    );
    println!(
        "  Local ML Python:   {}",
        config.backends.local_ml.python_path
    );
    println!(
        "  Local ML Model:    {}",
        config.backends.local_ml.default_model
    );

    println!();
    Ok(())
}
