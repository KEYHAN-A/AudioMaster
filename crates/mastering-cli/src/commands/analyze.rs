use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use mastering_core::analysis;

#[derive(Args)]
pub struct AnalyzeArgs {
    /// Audio file to analyze
    pub input: PathBuf,

    /// Output analysis as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: AnalyzeArgs) -> Result<()> {
    anyhow::ensure!(
        args.input.exists(),
        "Input file not found: {}",
        args.input.display()
    );

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Analyzing audio...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let analysis = analysis::analyze_file(&args.input)
        .await
        .context("Audio analysis failed")?;

    spinner.finish_and_clear();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
        return Ok(());
    }

    println!(
        "\n{}  {}",
        "ANALYSIS".bold().cyan(),
        args.input.display().to_string().white()
    );

    println!("\n{}", "Metadata".bold().yellow());
    println!("  Format:       {}", analysis.metadata.format);
    println!("  Sample Rate:  {} Hz", analysis.metadata.sample_rate);
    println!("  Channels:     {}", analysis.metadata.channels);
    println!("  Duration:     {:.1}s", analysis.metadata.duration_secs);

    println!("\n{}", "Loudness".bold().yellow());
    println!("  Integrated LUFS:   {:.1}", analysis.lufs_integrated);
    println!("  Short-term Max:    {:.1} LUFS", analysis.lufs_short_term_max);
    println!("  RMS:               {:.1} dB", analysis.rms_db);

    println!("\n{}", "Dynamics".bold().yellow());
    println!("  Peak:              {:.1} dB", analysis.peak_db);
    println!("  True Peak:         {:.1} dB", analysis.true_peak_db);
    println!("  Dynamic Range:     {:.1} dB", analysis.dynamic_range_db);

    println!("\n{}", "Stereo".bold().yellow());
    let width_desc = if analysis.stereo_width < 0.1 {
        "Mono"
    } else if analysis.stereo_width < 0.5 {
        "Narrow"
    } else if analysis.stereo_width < 0.8 {
        "Normal"
    } else if analysis.stereo_width < 1.2 {
        "Wide"
    } else {
        "Very Wide (possible phase issues)"
    };
    println!(
        "  Stereo Width:      {:.2} ({})",
        analysis.stereo_width, width_desc
    );

    println!("\n{}", "Frequency Balance".bold().yellow());
    let bands = &analysis.frequency_bands;
    print_band("Sub-bass  (20-60 Hz)   ", bands.sub_bass);
    print_band("Bass      (60-250 Hz)  ", bands.bass);
    print_band("Low-mid   (250-500 Hz) ", bands.low_mid);
    print_band("Mid       (500-2k Hz)  ", bands.mid);
    print_band("Upper-mid (2k-4k Hz)   ", bands.upper_mid);
    print_band("Presence  (4k-6k Hz)   ", bands.presence);
    print_band("Brilliance(6k-20k Hz)  ", bands.brilliance);

    println!();
    Ok(())
}

fn print_band(label: &str, db: f64) {
    let bar_len = ((db + 10.0) * 3.0).max(0.0).min(40.0) as usize;
    let bar: String = "#".repeat(bar_len);
    let color_bar = if db > -3.0 {
        bar.green()
    } else if db > -7.0 {
        bar.yellow()
    } else {
        bar.red()
    };
    println!("  {label} {:>6.1} dB  {color_bar}", db);
}
