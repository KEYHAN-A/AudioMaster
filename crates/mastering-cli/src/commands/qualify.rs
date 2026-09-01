use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct QualifyArgs {
    /// Approved-vector manifest with hashes and expected measurements.
    pub manifest: PathBuf,
    /// JSON evidence output. Defaults beside the manifest.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub async fn run(args: QualifyArgs) -> Result<()> {
    let report = mastering_core::qualification::run_manifest(&args.manifest).await?;
    let output = args
        .output
        .unwrap_or_else(|| args.manifest.with_extension("report.json"));
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("Writing qualification report {}", output.display()))?;
    println!("Qualification report: {}", output.display());
    anyhow::ensure!(report.passed, "One or more qualification vectors failed");
    Ok(())
}
