//! Reproducible meter qualification against approved reference vectors.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct QualificationManifest {
    pub suite: String,
    pub vectors: Vec<QualificationVector>,
}

#[derive(Debug, Deserialize)]
pub struct QualificationVector {
    pub file: PathBuf,
    pub sha256: String,
    pub expected_lufs: f64,
    pub expected_lra_lu: Option<f64>,
    pub expected_true_peak_dbtp: Option<f64>,
    #[serde(default = "default_loudness_tolerance")]
    pub loudness_tolerance_lu: f64,
    #[serde(default = "default_lra_tolerance")]
    pub lra_tolerance_lu: f64,
    #[serde(default = "default_peak_tolerance")]
    pub true_peak_tolerance_db: f64,
}

#[derive(Debug, Serialize)]
pub struct QualificationReport {
    pub schema_version: u16,
    pub engine_version: String,
    pub suite: String,
    pub passed: bool,
    pub results: Vec<QualificationResult>,
}

#[derive(Debug, Serialize)]
pub struct QualificationResult {
    pub file: PathBuf,
    pub sha256: String,
    pub hash_matches: bool,
    pub expected_lufs: f64,
    pub actual_lufs: f64,
    pub expected_lra_lu: Option<f64>,
    pub actual_lra_lu: f64,
    pub expected_true_peak_dbtp: Option<f64>,
    pub actual_true_peak_dbtp: f64,
    pub passed: bool,
}

fn default_loudness_tolerance() -> f64 {
    0.1
}
fn default_lra_tolerance() -> f64 {
    0.5
}
fn default_peak_tolerance() -> f64 {
    0.1
}

pub async fn run_manifest(manifest_path: &Path) -> Result<QualificationReport> {
    let contents = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("Reading qualification manifest {}", manifest_path.display()))?;
    let manifest: QualificationManifest = serde_json::from_str(&contents)
        .with_context(|| format!("Parsing qualification manifest {}", manifest_path.display()))?;
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut results = Vec::with_capacity(manifest.vectors.len());
    for vector in manifest.vectors {
        let path = if vector.file.is_absolute() {
            vector.file.clone()
        } else {
            base.join(&vector.file)
        };
        let actual_hash = sha256_file(&path)?;
        let hash_matches = actual_hash.eq_ignore_ascii_case(&vector.sha256);
        let analysis = crate::analysis::analyze_file(&path).await?;
        let loudness_pass =
            (analysis.lufs_integrated - vector.expected_lufs).abs() <= vector.loudness_tolerance_lu;
        let lra_pass = vector.expected_lra_lu.is_none_or(|expected| {
            (analysis.loudness_range_lu - expected).abs() <= vector.lra_tolerance_lu
        });
        let peak_pass = vector.expected_true_peak_dbtp.is_none_or(|expected| {
            (analysis.true_peak_db - expected).abs() <= vector.true_peak_tolerance_db
        });
        results.push(QualificationResult {
            file: vector.file,
            sha256: actual_hash,
            hash_matches,
            expected_lufs: vector.expected_lufs,
            actual_lufs: analysis.lufs_integrated,
            expected_lra_lu: vector.expected_lra_lu,
            actual_lra_lu: analysis.loudness_range_lu,
            expected_true_peak_dbtp: vector.expected_true_peak_dbtp,
            actual_true_peak_dbtp: analysis.true_peak_db,
            passed: hash_matches && loudness_pass && lra_pass && peak_pass,
        });
    }
    Ok(QualificationReport {
        schema_version: 1,
        engine_version: env!("CARGO_PKG_VERSION").into(),
        suite: manifest.suite,
        passed: results.iter().all(|result| result.passed),
        results,
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
