use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use mastering_core::config::Config;
use mastering_core::pipeline::{self, MasteringJob};
use mastering_core::types::{AiProvider, AudioFormat, Backend, Preset};

#[derive(Args)]
pub struct MasterArgs {
    /// Input audio file to master
    pub input: PathBuf,

    /// Reference track (triggers Matchering mode)
    #[arg(short, long)]
    pub reference: Option<PathBuf>,

    /// Mastering backend: auto, matchering, ai, local-ml
    #[arg(short, long, default_value = "auto")]
    pub backend: String,

    /// AI provider: ollama, keyhanstudio, openai, anthropic
    #[arg(long)]
    pub ai_provider: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output bit depth: 16, 24, or 32
    #[arg(long)]
    pub bit_depth: Option<u16>,

    /// Output format: wav, flac, mp3
    #[arg(short, long)]
    pub format: Option<String>,

    /// Target loudness in LUFS
    #[arg(long)]
    pub target_lufs: Option<f64>,

    /// Mastering preset: streaming, cd, vinyl, loud
    #[arg(short, long)]
    pub preset: Option<String>,

    /// Skip the final limiter
    #[arg(long)]
    pub no_limiter: bool,

    /// Analyze only, don't process
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: MasterArgs) -> Result<()> {
    let config = Config::load().context("Loading configuration")?;

    anyhow::ensure!(
        args.input.exists(),
        "Input file not found: {}",
        args.input.display()
    );

    let backend: Backend = args.backend.parse()?;
    let ai_provider: Option<AiProvider> = args
        .ai_provider
        .map(|s| s.parse())
        .transpose()?;
    let format: Option<AudioFormat> = args.format.map(|s| s.parse()).transpose()?;
    let preset: Option<Preset> = args.preset.map(|s| s.parse()).transpose()?;

    if let Some(bd) = args.bit_depth {
        anyhow::ensure!(
            bd == 16 || bd == 24 || bd == 32,
            "Bit depth must be 16, 24, or 32 (got {bd})"
        );
    }

    let job = MasteringJob {
        input_path: args.input.clone(),
        output_path: args.output,
        reference_path: args.reference,
        backend,
        ai_provider,
        bit_depth: args.bit_depth,
        format,
        target_lufs: args.target_lufs,
        no_limiter: args.no_limiter,
        preset,
        dry_run: args.dry_run,
    };

    println!(
        "\n{}  {}",
        "MASTERING".bold().cyan(),
        args.input.display().to_string().white()
    );

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Processing...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let result = pipeline::run(&job, &config).await?;

    spinner.finish_and_clear();

    // Print results
    println!("\n{}", "Results".bold().green());
    println!("  Backend:  {}", result.backend_used.cyan());
    println!("  Output:   {}", result.output_path.display().to_string().white());

    if let Some(ref pre) = result.pre_analysis {
        println!("\n{}", "Input Analysis".bold().yellow());
        println!("  LUFS:         {:.1}", pre.lufs_integrated);
        println!("  Peak:         {:.1} dB", pre.peak_db);
        println!("  RMS:          {:.1} dB", pre.rms_db);
        println!("  Dynamic Range:{:.1} dB", pre.dynamic_range_db);
        println!("  Stereo Width: {:.2}", pre.stereo_width);
        println!("  Duration:     {:.1}s", pre.metadata.duration_secs);
    }

    if let Some(ref post) = result.post_analysis {
        println!("\n{}", "Output Analysis".bold().green());
        println!("  LUFS:         {:.1}", post.lufs_integrated);
        println!("  Peak:         {:.1} dB", post.peak_db);
        println!("  RMS:          {:.1} dB", post.rms_db);
        println!("  Dynamic Range:{:.1} dB", post.dynamic_range_db);
        println!("  Stereo Width: {:.2}", post.stereo_width);
    }

    if let Some(ref params) = result.params_applied {
        println!("\n{}", "Applied Parameters".bold().blue());
        println!("  EQ Bands:     {}", params.eq.len());
        println!(
            "  Compression:  ratio {:.1}:1, threshold {:.1} dB",
            params.compression.ratio, params.compression.threshold_db
        );
        if params.limiter.enabled {
            println!("  Limiter:      ceiling {:.1} dB", params.limiter.ceiling_db);
        } else {
            println!("  Limiter:      disabled");
        }
        println!("  Target LUFS:  {:.1}", params.target_lufs);
    }

    println!();
    Ok(())
}
