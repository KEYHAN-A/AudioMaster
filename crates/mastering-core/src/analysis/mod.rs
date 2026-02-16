mod decode;
mod metrics;

pub use decode::decode_audio;
pub use metrics::analyze;

use crate::types::AudioAnalysis;
use anyhow::Result;
use std::path::Path;

/// Full analysis pipeline: decode file then compute all metrics.
pub async fn analyze_file(path: &Path) -> Result<AudioAnalysis> {
    let decoded = decode::decode_audio(path)?;
    let analysis = metrics::analyze(path, &decoded)?;
    Ok(analysis)
}
