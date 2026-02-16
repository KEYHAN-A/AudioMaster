use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

use super::{BackendOutput, MasteringOptions};
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct MatcheringBackend {
    python_path: String,
    scripts_dir: std::path::PathBuf,
}

impl MatcheringBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            python_path: config.backends.matchering.python_path.clone(),
            scripts_dir: Config::python_scripts_dir(),
        }
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        let reference = opts
            .reference_path
            .as_ref()
            .context("Matchering backend requires a reference track (--reference)")?;

        let script = self.scripts_dir.join("matchering_bridge.py");
        anyhow::ensure!(
            script.exists(),
            "Matchering bridge script not found at: {}",
            script.display()
        );

        info!(
            "Running Matchering: target={}, reference={}",
            opts.input_path.display(),
            reference.display()
        );

        let request = serde_json::json!({
            "target": opts.input_path.to_string_lossy(),
            "reference": reference.to_string_lossy(),
            "output": opts.output_path.to_string_lossy(),
            "bit_depth": opts.bit_depth,
            "no_limiter": opts.no_limiter,
        });

        let output = Command::new(&self.python_path)
            .arg(&script)
            .arg(request.to_string())
            .output()
            .with_context(|| {
                format!(
                    "Failed to run matchering bridge script. Is Python installed at '{}'?",
                    self.python_path
                )
            })?;

        debug!("Matchering stdout: {}", String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Matchering failed:\n{stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: serde_json::Value = serde_json::from_str(stdout.trim())
            .with_context(|| format!("Parsing matchering output: {stdout}"))?;

        let result_path = response["output"]
            .as_str()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| opts.output_path.clone());

        let message = response["message"]
            .as_str()
            .unwrap_or("Matchering completed successfully")
            .to_string();

        Ok(BackendOutput {
            output_path: result_path,
            params_applied: None,
            backend_name: "matchering".into(),
            message,
        })
    }

    pub async fn check_available(&self) -> Result<bool> {
        let script = self.scripts_dir.join("matchering_bridge.py");
        if !script.exists() {
            return Ok(false);
        }

        let python = self.python_path.clone();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::task::spawn_blocking(move || {
                Command::new(&python)
                    .arg("-c")
                    .arg("import matchering; print('ok')")
                    .output()
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(o))) => Ok(o.status.success()),
            _ => Ok(false),
        }
    }
}
