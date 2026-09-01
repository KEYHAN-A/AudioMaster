pub mod decode;
mod metrics;

pub use decode::decode_audio;
pub use metrics::analyze;

use crate::types::AudioAnalysis;
use anyhow::{Context, Result};
use std::path::Path;

/// Full analysis pipeline: decode file then compute all metrics.
pub async fn analyze_file(path: &Path) -> Result<AudioAnalysis> {
    let path = path.to_path_buf();
    if let Some(cached) = crate::cache::global_cache().get(&path).await {
        return Ok(cached);
    }
    let decoded = tokio::task::spawn_blocking({
        let path = path.clone();
        move || decode::decode_audio(&path)
    })
    .await
    .map_err(|error| anyhow::anyhow!("Audio analysis worker failed: {error}"))?;
    let analysis = match decoded {
        Ok(decoded) => metrics::analyze(&path, &decoded)?,
        Err(error) if error.to_string().contains("256 MiB processing limit") => {
            analyze_large_file(&path).await?
        }
        Err(error) => return Err(error),
    };
    crate::cache::global_cache().put(path, &analysis).await;
    Ok(analysis)
}

async fn analyze_large_file(path: &Path) -> Result<AudioAnalysis> {
    // Full-duration EBU measurements are streamed by FFmpeg. Secondary tonal
    // and spatial metrics use a bounded prefix and are marked by the same
    // report schema; release qualification validates the authoritative fields.
    let prefix_path = path.to_path_buf();
    let mut report = tokio::task::spawn_blocking(move || {
        let decoded = decode::decode_audio_prefix(&prefix_path, 8 * 1024 * 1024)?;
        metrics::analyze(&prefix_path, &decoded)
    })
    .await
    .map_err(|error| anyhow::anyhow!("Bounded analysis worker failed: {error}"))??;

    let output = tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(path)
        .args([
            "-af",
            "loudnorm=I=-23:TP=-1:LRA=7:print_format=json",
            "-f",
            "null",
            "-",
        ])
        .output()
        .await
        .context("Running bounded full-duration loudness analysis with FFmpeg")?;
    anyhow::ensure!(output.status.success(), "FFmpeg loudness analysis failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let start = stderr
        .rfind('{')
        .context("FFmpeg loudness report was missing")?;
    let end = stderr
        .rfind('}')
        .context("FFmpeg loudness report was incomplete")?;
    let loudness: serde_json::Value = serde_json::from_str(&stderr[start..=end])?;
    let number = |field: &str| -> Result<f64> {
        loudness[field]
            .as_str()
            .context(format!("FFmpeg report omitted {field}"))?
            .parse::<f64>()
            .context(format!("FFmpeg returned invalid {field}"))
    };
    report.lufs_integrated = number("input_i")?;
    report.true_peak_db = number("input_tp")?;
    report.loudness_range_lu = number("input_lra")?;
    report.peak_to_loudness_ratio = (report.true_peak_db - report.lufs_integrated).max(0.0);

    let probe = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration,format_name",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .await
        .context("Reading long-file metadata with ffprobe")?;
    if probe.status.success() {
        let value: serde_json::Value = serde_json::from_slice(&probe.stdout)?;
        if let Some(duration) = value["format"]["duration"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            report.metadata.duration_secs = duration;
        }
        if let Some(format) = value["format"]["format_name"].as_str() {
            report.metadata.format = format.to_uppercase();
        }
    }
    Ok(report)
}
