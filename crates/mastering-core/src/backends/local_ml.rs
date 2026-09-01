use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, info};

use super::{BackendOutput, MasteringOptions};
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct LocalMlBackend {
    python_path: String,
    default_model: String,
    scripts_dir: std::path::PathBuf,
}

impl LocalMlBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            python_path: config.backends.local_ml.python_path.clone(),
            default_model: config.backends.local_ml.default_model.clone(),
            scripts_dir: Config::python_scripts_dir(),
        }
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        let script = self.scripts_dir.join("ml_inference.py");
        anyhow::ensure!(
            script.exists(),
            "ML inference script not found at: {}",
            script.display()
        );

        info!(
            "Running local ML model '{}' on: {}",
            self.default_model,
            opts.input_path.display()
        );

        let request = serde_json::json!({
            "input": opts.input_path.to_string_lossy(),
            "output": opts.output_path.to_string_lossy(),
            "model": self.default_model,
            "reference": opts.reference_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "bit_depth": opts.bit_depth,
            "target_lufs": opts.target_lufs,
            "no_limiter": opts.no_limiter,
        });

        let output = Command::new(&self.python_path)
            .arg(&script)
            .arg(request.to_string())
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to run ML inference script. Is Python installed at '{}'?",
                    self.python_path
                )
            })?;

        debug!(
            "ML inference stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ML inference failed:\n{stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: serde_json::Value = serde_json::from_str(stdout.trim())
            .with_context(|| format!("Parsing ML inference output: {stdout}"))?;

        let result_path = response["output"]
            .as_str()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| opts.output_path.clone());

        let message = response["message"]
            .as_str()
            .unwrap_or("ML model processing completed")
            .to_string();

        Ok(BackendOutput {
            output_path: result_path,
            params_applied: None,
            backend_name: format!("local-ml/{}", self.default_model),
            message,
            warnings: Vec::new(),
        })
    }

    pub async fn check_available(&self) -> Result<bool> {
        let script = self.scripts_dir.join("ml_inference.py");
        if !script.exists() {
            return Ok(false);
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            Command::new(&self.python_path)
                .arg("-c")
                .arg("import soundfile; print('ok')")
                .output(),
        )
        .await;
        Ok(matches!(result, Ok(Ok(output)) if output.status.success()))
    }
}
