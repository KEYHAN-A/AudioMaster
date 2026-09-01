use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use mastering_core::album::{self, AlbumJob};
use mastering_core::config::Config;
use mastering_core::types::{AiProvider, AudioFormat, Backend, Preset};

#[derive(Args)]
pub struct AlbumArgs {
    /// Ordered input tracks. Quote paths containing spaces.
    #[arg(required = true, num_args = 2..)]
    pub inputs: Vec<PathBuf>,

    /// Directory for mastered album tracks.
    #[arg(short, long)]
    pub output_directory: PathBuf,

    #[arg(short, long, default_value = "native")]
    pub backend: String,

    #[arg(long)]
    pub ai_provider: Option<String>,

    #[arg(short, long)]
    pub reference: Option<PathBuf>,

    #[arg(long)]
    pub bit_depth: Option<u16>,

    #[arg(short, long)]
    pub format: Option<String>,

    #[arg(long)]
    pub target_lufs: Option<f64>,

    #[arg(short, long)]
    pub preset: Option<String>,

    #[arg(long)]
    pub no_limiter: bool,

    /// Maximum intentional source loudness difference preserved per track.
    #[arg(long, default_value_t = 1.5)]
    pub max_relative_offset_lu: f64,

    /// Per-track LU adjustments in input order (repeat once per track).
    #[arg(long, value_delimiter = ',')]
    pub track_offsets_lu: Vec<f64>,
}

pub async fn run(args: AlbumArgs) -> Result<()> {
    let config = Config::load().context("Loading configuration")?;
    let backend: Backend = args.backend.parse()?;
    let ai_provider: Option<AiProvider> =
        args.ai_provider.map(|value| value.parse()).transpose()?;
    let format: Option<AudioFormat> = args.format.map(|value| value.parse()).transpose()?;
    let preset: Option<Preset> = args.preset.map(|value| value.parse()).transpose()?;

    let job = AlbumJob {
        input_paths: args.inputs,
        output_directory: args.output_directory,
        reference_path: args.reference,
        backend,
        ai_provider,
        bit_depth: args.bit_depth,
        format,
        target_lufs: args.target_lufs,
        no_limiter: args.no_limiter,
        preset,
        max_relative_offset_lu: args.max_relative_offset_lu,
        track_offsets_lu: args.track_offsets_lu,
    };

    println!(
        "\n{}  {} tracks",
        "ALBUM MASTERING".bold().cyan(),
        job.input_paths.len()
    );
    let result = album::run(&job, &config).await?;

    println!("\n{}", "Album Results".bold().green());
    println!("  Album target:      {:.1} LUFS", result.album_target_lufs);
    println!("  Source median:     {:.1} LUFS", result.source_median_lufs);
    println!(
        "  Delivered spread:  {:.1} LU",
        result.delivered_loudness_spread_lu
    );
    println!("  Verification report: {}", result.report_path.display());
    for (index, track) in result.tracks.iter().enumerate() {
        let delivered = track
            .result
            .post_analysis
            .as_ref()
            .map(|analysis| format!("{:.1} LUFS", analysis.lufs_integrated))
            .unwrap_or_else(|| "unverified".into());
        println!(
            "  {:02}. {} → {} (target {:.1}, {})",
            index + 1,
            track.input_path.display(),
            track.result.output_path.display(),
            track.assigned_target_lufs,
            delivered
        );
    }
    println!();
    Ok(())
}
