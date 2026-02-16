use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

use super::{BackendOutput, MasteringOptions};
use crate::analysis;
use crate::config::Config;
use crate::types::{AiProvider, MasteringParams};

#[derive(Debug, Clone)]
pub struct AiBackend {
    provider: AiProvider,
    ollama_endpoint: String,
    ollama_model: String,
    keyhanstudio_endpoint: String,
    keyhanstudio_api_key: String,
    openai_api_key: String,
    openai_model: String,
    anthropic_api_key: String,
    anthropic_model: String,
    python_path: String,
    scripts_dir: std::path::PathBuf,
}

impl AiBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            provider: config.ai.default_provider,
            ollama_endpoint: config.ai.ollama.endpoint.clone(),
            ollama_model: config.ai.ollama.model.clone(),
            keyhanstudio_endpoint: config.ai.keyhanstudio.endpoint.clone(),
            keyhanstudio_api_key: config.ai.keyhanstudio.api_key.clone(),
            openai_api_key: config.ai.openai.api_key.clone(),
            openai_model: config.ai.openai.model.clone(),
            anthropic_api_key: config.ai.anthropic.api_key.clone(),
            anthropic_model: config.ai.anthropic.model.clone(),
            python_path: config.backends.matchering.python_path.clone(),
            scripts_dir: Config::python_scripts_dir(),
        }
    }

    pub fn with_provider(mut self, provider: AiProvider) -> Self {
        self.provider = provider;
        self
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        info!("AI-assisted mastering using provider: {}", self.provider);

        // Step 1: Analyze the input audio
        let analysis = analysis::analyze_file(&opts.input_path).await?;
        let analysis_json = serde_json::to_string_pretty(&analysis)?;
        debug!("Audio analysis:\n{analysis_json}");

        // Step 2: Ask the AI for mastering parameters
        let prompt = build_mastering_prompt(&analysis_json, opts);
        let ai_response = self.call_ai(&prompt).await?;
        debug!("AI response:\n{ai_response}");

        // Step 3: Parse mastering parameters from AI response
        let params = parse_mastering_params(&ai_response)?;
        let _params_json = serde_json::to_string(&params)?;

        // Step 4: Apply parameters via Python DSP bridge
        let script = self.scripts_dir.join("apply_fx.py");
        anyhow::ensure!(
            script.exists(),
            "DSP bridge script not found at: {}",
            script.display()
        );

        let request = serde_json::json!({
            "input": opts.input_path.to_string_lossy(),
            "output": opts.output_path.to_string_lossy(),
            "params": params,
            "bit_depth": opts.bit_depth,
        });

        let output = Command::new(&self.python_path)
            .arg(&script)
            .arg(request.to_string())
            .output()
            .with_context(|| {
                format!(
                    "Failed to run DSP bridge. Is Python installed at '{}'?",
                    self.python_path
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("DSP processing failed:\n{stderr}");
        }

        info!("AI-assisted mastering completed");

        Ok(BackendOutput {
            output_path: opts.output_path.clone(),
            params_applied: Some(params),
            backend_name: format!("ai/{}", self.provider),
            message: format!(
                "Mastered using {} AI provider with custom EQ, compression, and limiting",
                self.provider
            ),
        })
    }

    async fn call_ai(&self, prompt: &str) -> Result<String> {
        match self.provider {
            AiProvider::Ollama => self.call_ollama(prompt).await,
            AiProvider::KeyhanStudio => self.call_keyhanstudio(prompt).await,
            AiProvider::OpenAi => self.call_openai(prompt).await,
            AiProvider::Anthropic => self.call_anthropic(prompt).await,
        }
    }

    async fn call_ollama(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", self.ollama_endpoint);

        let body = serde_json::json!({
            "model": self.ollama_model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Calling Ollama API")?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Ollama API error ({status}): {text}");
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let response = parsed["response"]
            .as_str()
            .unwrap_or(&text)
            .to_string();

        Ok(response)
    }

    async fn call_keyhanstudio(&self, prompt: &str) -> Result<String> {
        anyhow::ensure!(
            !self.keyhanstudio_endpoint.is_empty(),
            "KeyhanStudio endpoint not configured. Set it in ~/.config/mastering/config.toml"
        );

        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ],
            "response_format": { "type": "json_object" },
        });

        let mut req = client.post(&self.keyhanstudio_endpoint).json(&body);

        if !self.keyhanstudio_api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.keyhanstudio_api_key));
        }

        let resp = req.send().await.context("Calling KeyhanStudio API")?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("KeyhanStudio API error ({status}): {text}");
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| parsed["response"].as_str())
            .unwrap_or(&text)
            .to_string();

        Ok(content)
    }

    async fn call_openai(&self, prompt: &str) -> Result<String> {
        anyhow::ensure!(
            !self.openai_api_key.is_empty(),
            "OpenAI API key not configured. Set it in ~/.config/mastering/config.toml"
        );

        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.openai_model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ],
            "response_format": { "type": "json_object" },
        });

        let resp = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.openai_api_key))
            .json(&body)
            .send()
            .await
            .context("Calling OpenAI API")?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("OpenAI API error ({status}): {text}");
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(&text)
            .to_string();

        Ok(content)
    }

    async fn call_anthropic(&self, prompt: &str) -> Result<String> {
        anyhow::ensure!(
            !self.anthropic_api_key.is_empty(),
            "Anthropic API key not configured. Set it in ~/.config/mastering/config.toml"
        );

        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.anthropic_model,
            "max_tokens": 4096,
            "system": SYSTEM_PROMPT,
            "messages": [
                {"role": "user", "content": prompt}
            ],
        });

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.anthropic_api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Calling Anthropic API")?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Anthropic API error ({status}): {text}");
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let content = parsed["content"][0]["text"]
            .as_str()
            .unwrap_or(&text)
            .to_string();

        Ok(content)
    }

    pub async fn check_available(&self) -> Result<bool> {
        match self.provider {
            AiProvider::Ollama => {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(3))
                    .build()?;
                let resp = client.get(&self.ollama_endpoint).send().await;
                Ok(resp.is_ok())
            }
            AiProvider::KeyhanStudio => {
                Ok(!self.keyhanstudio_endpoint.is_empty())
            }
            AiProvider::OpenAi => Ok(!self.openai_api_key.is_empty()),
            AiProvider::Anthropic => Ok(!self.anthropic_api_key.is_empty()),
        }
    }
}

const SYSTEM_PROMPT: &str = r#"You are a professional audio mastering engineer AI. Given audio analysis data, you provide precise mastering parameters as JSON. You respond ONLY with valid JSON, no explanations.

The JSON must have this exact structure:
{
  "eq": [
    {"frequency": 80.0, "gain_db": 1.5, "q": 0.7, "band_type": "low_shelf"},
    {"frequency": 3000.0, "gain_db": -0.5, "q": 1.0, "band_type": "peak"},
    {"frequency": 12000.0, "gain_db": 2.0, "q": 0.7, "band_type": "high_shelf"}
  ],
  "compression": {
    "threshold_db": -18.0,
    "ratio": 2.5,
    "attack_ms": 10.0,
    "release_ms": 100.0,
    "knee_db": 6.0,
    "makeup_gain_db": 2.0
  },
  "limiter": {
    "enabled": true,
    "ceiling_db": -1.0,
    "release_ms": 50.0
  },
  "stereo": {
    "width": 1.0,
    "balance": 0.0
  },
  "target_lufs": -14.0
}

band_type must be one of: low_shelf, high_shelf, peak, low_pass, high_pass
Provide musically appropriate values based on the analysis. Be subtle with EQ (usually +/- 3dB max)."#;

fn build_mastering_prompt(analysis_json: &str, opts: &MasteringOptions) -> String {
    let preset_info = opts
        .preset
        .map(|p| format!("\nPreset: {} — {}", p, p.description()))
        .unwrap_or_default();

    format!(
        r#"Analyze this audio and provide mastering parameters as JSON.

Audio Analysis:
{analysis_json}

Target LUFS: {target_lufs}
No Limiter: {no_limiter}{preset_info}

Provide your mastering parameters as a JSON object with keys: eq, compression, limiter, stereo, target_lufs."#,
        target_lufs = opts.target_lufs,
        no_limiter = opts.no_limiter,
    )
}

fn parse_mastering_params(response: &str) -> Result<MasteringParams> {
    // Try parsing the response directly
    if let Ok(params) = serde_json::from_str::<MasteringParams>(response) {
        return Ok(params);
    }

    // Try extracting JSON from markdown code blocks
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        response
    };

    serde_json::from_str::<MasteringParams>(json_str)
        .context("Failed to parse AI response as mastering parameters. The AI may have returned an unexpected format.")
}
