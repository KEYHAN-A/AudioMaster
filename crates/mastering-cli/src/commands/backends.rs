use anyhow::Result;
use colored::Colorize;

use mastering_core::backends::MasteringEngine;
use mastering_core::config::Config;
use mastering_core::types::Backend;

pub async fn run() -> Result<()> {
    let config = Config::load()?;

    println!("\n{}", "Available Backends".bold().cyan());

    let backends = [
        (Backend::Matchering, "Reference-based mastering (matches EQ, loudness, stereo width)"),
        (Backend::Ai, "AI-assisted mastering (LLM suggests DSP parameters)"),
        (Backend::LocalMl, "Local ML models (DeepAFx-ST, HuggingFace)"),
    ];

    for (backend, description) in &backends {
        let engine = MasteringEngine::from_config(*backend, &config);
        let available = engine.check_available().await.unwrap_or(false);

        let status = if available {
            "READY".bold().green()
        } else {
            "NOT AVAILABLE".bold().red()
        };

        println!("\n  {} [{}]", backend.to_string().bold().white(), status);
        println!("    {description}");
    }

    // Show AI provider details
    println!("\n{}", "AI Providers".bold().cyan());

    let providers = [
        ("Ollama (local)", !config.ai.ollama.endpoint.is_empty()),
        ("KeyhanStudio", !config.ai.keyhanstudio.endpoint.is_empty() && !config.ai.keyhanstudio.api_key.is_empty()),
        ("OpenAI", !config.ai.openai.api_key.is_empty()),
        ("Anthropic", !config.ai.anthropic.api_key.is_empty()),
    ];

    for (name, configured) in &providers {
        let status = if *configured {
            "configured".green()
        } else {
            "not configured".dimmed()
        };
        println!("  {name}: {status}");
    }

    println!(
        "\n  Default: {}",
        config.ai.default_provider.to_string().cyan()
    );

    println!();
    Ok(())
}
