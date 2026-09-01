use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{BackendOutput, MasteringOptions};
use crate::analysis;
use crate::config::Config;
use crate::dsp;
use crate::types::{AiProvider, MasteringParams, MasteringPlan};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LmStudioModel {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub size_gb: Option<f64>,
    #[serde(default)]
    pub quant: Option<String>,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub loaded: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AiBackend {
    provider: AiProvider,
    ollama_endpoint: String,
    ollama_model: String,
    lmstudio_endpoint: String,
    lmstudio_model: String,
    keyhanstudio_endpoint: String,
    keyhanstudio_api_key: String,
    openai_api_key: String,
    openai_model: String,
    anthropic_api_key: String,
    anthropic_model: String,
}

impl AiBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            provider: config.ai.default_provider,
            ollama_endpoint: config.ai.ollama.endpoint.clone(),
            ollama_model: config.ai.ollama.model.clone(),
            lmstudio_endpoint: config.ai.lmstudio.endpoint.clone(),
            lmstudio_model: config.ai.lmstudio.model.clone(),
            keyhanstudio_endpoint: if config.ai.keyhanstudio.endpoint.is_empty() {
                format!(
                    "{}/audiomaster/advice",
                    config.cloud.endpoint.trim_end_matches('/')
                )
            } else {
                config.ai.keyhanstudio.endpoint.clone()
            },
            keyhanstudio_api_key: config.ai.keyhanstudio.api_key.clone(),
            openai_api_key: config.ai.openai.api_key.clone(),
            openai_model: config.ai.openai.model.clone(),
            anthropic_api_key: config.ai.anthropic.api_key.clone(),
            anthropic_model: config.ai.anthropic.model.clone(),
        }
    }

    pub fn with_provider(mut self, provider: AiProvider) -> Self {
        self.provider = provider;
        self
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        info!("AI-assisted mastering using provider: {}", self.provider);

        // Step 1: Analyze the input audio
        let analysis = match &opts.pre_analysis {
            Some(analysis) => analysis.clone(),
            None => analysis::analyze_file(&opts.input_path).await?,
        };
        let mut advisor_analysis = analysis.clone();
        // Local paths are not acoustic information and must never cross a
        // remote-advisor boundary.
        advisor_analysis.metadata.path.clear();
        let analysis_json = serde_json::to_string_pretty(&advisor_analysis)?;
        debug!("Audio analysis:\n{analysis_json}");

        // Step 2: Ask the AI for mastering parameters
        let prompt = build_mastering_prompt(&analysis_json, opts);
        let ai_response = self.call_ai(&prompt).await?;
        debug!("AI response:\n{ai_response}");

        // Step 3: Parse mastering parameters from AI response
        let proposed = parse_mastering_params(&ai_response)?;
        let validated = dsp::validate_params(
            proposed,
            analysis.metadata.sample_rate,
            opts.target_lufs,
            opts.no_limiter,
        )?;
        let mut params = validated.params;

        // Step 4: Apply the validated plan with the deterministic native engine.
        let input = opts.input_path.clone();
        let output = opts.output_path.clone();
        let bit_depth = opts.bit_depth;
        let control = opts.control.clone();
        let mut warnings = validated.warnings;
        if matches!(
            opts.delivery_format,
            crate::types::AudioFormat::Mp3 | crate::types::AudioFormat::Aac
        ) && params.limiter.enabled
            && params.limiter.ceiling_db > -1.2
        {
            params.limiter.ceiling_db = -1.2;
            warnings.push("Limiter ceiling reduced to -1.2 dBTP for codec headroom".into());
        }
        let render_params = params.clone();
        warnings.extend(
            tokio::task::spawn_blocking(move || {
                dsp::render_wav_with_control(&input, &output, &render_params, bit_depth, &control)
            })
            .await
            .context("Native DSP worker failed")??,
        );

        info!("AI-assisted mastering completed");

        Ok(BackendOutput {
            output_path: opts.output_path.clone(),
            params_applied: Some(params),
            backend_name: format!("ai/{}", self.provider),
            message: if warnings.is_empty() {
                format!("Mastered using {} advisor and native DSP", self.provider)
            } else {
                format!(
                    "Mastered using {} advisor and native DSP ({} safety adjustments)",
                    self.provider,
                    warnings.len()
                )
            },
            warnings,
        })
    }

    async fn call_ai(&self, prompt: &str) -> Result<String> {
        match self.provider {
            AiProvider::Ollama => self.call_ollama(prompt).await,
            AiProvider::LmStudio => self.call_lmstudio(prompt).await,
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
        let response = parsed["response"].as_str().unwrap_or(&text).to_string();

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
            req = req.header(
                "Authorization",
                format!("Bearer {}", self.keyhanstudio_api_key),
            );
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

    async fn call_lmstudio(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/chat/completions",
            self.lmstudio_endpoint.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": self.lmstudio_model,
            "messages": [
                {"role": "system", "content": LMSTUDIO_SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.3,
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Calling LM Studio API — is LM Studio running and a model loaded?")?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("LM Studio API error ({status}): {text}");
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let content = parsed["choices"][0]["message"]["content"]
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
                Ok(matches!(resp, Ok(response) if response.status().is_success()))
            }
            AiProvider::LmStudio => {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(3))
                    .build()?;
                let url = self.lmstudio_endpoint.trim_end_matches('/');
                let resp = client.get(url).send().await;
                Ok(matches!(resp, Ok(response) if response.status().is_success()))
            }
            AiProvider::KeyhanStudio => Ok(!self.keyhanstudio_endpoint.is_empty()),
            AiProvider::OpenAi => Ok(!self.openai_api_key.is_empty()),
            AiProvider::Anthropic => Ok(!self.anthropic_api_key.is_empty()),
        }
    }

    pub async fn lmstudio_status(endpoint: &str) -> Result<bool> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;
        let url = endpoint.trim_end_matches('/');
        let resp = client.get(url).send().await;
        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn lmstudio_models(endpoint: &str) -> Result<Vec<LmStudioModel>> {
        // Try native LM Studio API first for richer metadata
        if let Ok(models) = Self::lmstudio_models_native(endpoint).await {
            if !models.is_empty() {
                return Ok(models);
            }
        }

        // Fallback to OpenAI-compatible endpoint
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let url = format!("{}/models", endpoint.trim_end_matches('/'));
        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to LM Studio. Is it running?")?;

        let parsed: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse LM Studio response")?;

        let models = parsed["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| {
                let id = m["id"].as_str()?.to_string();
                Some(LmStudioModel {
                    id,
                    display_name: None,
                    size_gb: None,
                    quant: None,
                    architecture: None,
                    loaded: None,
                })
            })
            .collect();

        Ok(models)
    }

    /// Fetch models from LM Studio's native REST API for richer metadata.
    async fn lmstudio_models_native(endpoint: &str) -> Result<Vec<LmStudioModel>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let base = endpoint.trim_end_matches("/v1").trim_end_matches('/');
        let url = format!("{base}/api/v1/models");

        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to reach LM Studio native API")?;
        if !resp.status().is_success() {
            anyhow::bail!("Native API returned {}", resp.status());
        }

        let parsed: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse LM Studio native response")?;

        let models = parsed["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| {
                let id = m["id"].as_str()?.to_string();
                let display_name = m["name"]
                    .as_str()
                    .or_else(|| m["display_name"].as_str())
                    .map(|s| s.to_string());
                let size_gb = m["size_gb"]
                    .as_f64()
                    .or_else(|| m["sizeBytes"].as_f64().map(|b| b / 1_073_741_824.0));
                let quant = m["quant"]
                    .as_str()
                    .or_else(|| m["quantization"].as_str())
                    .map(|s| s.to_string());
                let architecture = m["architecture"]
                    .as_str()
                    .or_else(|| m["arch"].as_str())
                    .map(|s| s.to_string());
                let loaded = m["loaded"].as_bool();

                Some(LmStudioModel {
                    id,
                    display_name,
                    size_gb,
                    quant,
                    architecture,
                    loaded,
                })
            })
            .collect();

        Ok(models)
    }

    /// Load a model in LM Studio via the native REST API.
    pub async fn lmstudio_load_model(endpoint: &str, model_id: &str) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let base = endpoint.trim_end_matches("/v1").trim_end_matches('/');

        // Try native API first: POST /api/v1/models/{identifier}/load
        let native_url = format!(
            "{base}/api/v1/models/{}/load",
            urlencoding::encode(model_id)
        );
        let resp = client.post(&native_url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                info!("Loaded model {model_id} via native API");
                return Ok(());
            }
            Ok(r) => {
                debug!("Native load API returned {}, falling back", r.status());
            }
            Err(e) => {
                debug!("Native load API unreachable: {e}, falling back");
            }
        }

        // Fallback: POST /api/v0/models/{identifier}/load (alternate path)
        let alt_url = format!(
            "{base}/api/v0/models/{}/load",
            urlencoding::encode(model_id)
        );
        let resp = client
            .post(&alt_url)
            .send()
            .await
            .context("Failed to reach LM Studio model loading endpoint")?;

        if resp.status().is_success() {
            info!("Loaded model {model_id} via fallback API");
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to load model {model_id}: {status} — {text}")
        }
    }

    /// Unload a model in LM Studio via the native REST API.
    pub async fn lmstudio_unload_model(endpoint: &str, model_id: &str) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let base = endpoint.trim_end_matches("/v1").trim_end_matches('/');

        let native_url = format!(
            "{base}/api/v1/models/{}/unload",
            urlencoding::encode(model_id)
        );
        let resp = client.post(&native_url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                info!("Unloaded model {model_id} via native API");
                return Ok(());
            }
            Ok(r) => {
                debug!("Native unload API returned {}, falling back", r.status());
            }
            Err(e) => {
                debug!("Native unload API unreachable: {e}, falling back");
            }
        }

        let alt_url = format!(
            "{base}/api/v0/models/{}/unload",
            urlencoding::encode(model_id)
        );
        let resp = client
            .post(&alt_url)
            .send()
            .await
            .context("Failed to reach LM Studio model unloading endpoint")?;

        if resp.status().is_success() {
            info!("Unloaded model {model_id} via fallback API");
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to unload model {model_id}: {status} — {text}")
        }
    }

    /// Get currently loaded models in LM Studio.
    pub async fn lmstudio_loaded_models(endpoint: &str) -> Result<Vec<LmStudioModel>> {
        let all = Self::lmstudio_models(endpoint).await?;
        Ok(all.into_iter().filter(|m| m.loaded == Some(true)).collect())
    }
}

const SYSTEM_PROMPT: &str = r#"You are a professional audio mastering engineer AI. Given audio analysis data, you provide precise mastering parameters as JSON. You respond ONLY with valid JSON, no explanations.

The JSON must have this exact versioned structure:
{
  "schema_version": 1,
  "params": {
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
}

band_type must be one of: low_shelf, high_shelf, peak, low_pass, high_pass
Provide musically appropriate values based on the analysis. Be subtle with EQ (usually +/- 3dB max)."#;

const LMSTUDIO_SYSTEM_PROMPT: &str = r#"You are a professional audio mastering engineer. You receive audio analysis data and output precise mastering parameters as a JSON object. You output ONLY valid JSON — no explanations, no markdown, no commentary.

STEP 1 - ANALYZE the audio:
- Compare loudness to target: if LUFS is much louder than target, reduce gain; if much quieter, plan gain boost
- Check dynamic range: wide range (>15dB) suggests gentle compression; narrow range (<6dB) suggests minimal compression
- Examine frequency bands: identify if sub-bass is excessive, midrange is muddy, or brilliance is lacking
- Note stereo width: values far from 1.0 may need correction

STEP 2 - RECOMMEND parameters:
- EQ: Apply corrective EQ first (cut problematic frequencies), then gentle enhancement (usually +/- 2dB max)
  - Use low_shelf for bass adjustments (60-120Hz), peak for midrange (250-4000Hz), high_shelf for air (8000-12000Hz)
  - Q values: 0.5-1.0 for broad shaping, 1.0-3.0 for surgical corrections
- Compression: Match to genre and dynamic range
  - Gentle: ratio 1.5-2.5, slow attack (15-30ms), auto release
  - Moderate: ratio 2.5-4.0, medium attack (5-15ms)
  - Never exceed ratio 6.0 for mastering
- Limiter: Set ceiling at -1.0 to -0.5 dB, moderate release (30-100ms)
- Stereo: Width 0.9-1.1 is safe. Adjust only if analysis shows problems.
- Target LUFS: Match the specified target precisely.

STEP 3 - OUTPUT this exact versioned JSON structure:
{
  "schema_version": 1,
  "params": {
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
}

band_type must be one of: low_shelf, high_shelf, peak, low_pass, high_pass
Value ranges: EQ gain -6 to +6 dB, Q 0.3 to 5.0, compression ratio 1.0 to 6.0, stereo width 0.5 to 1.5.
IMPORTANT: Return ONLY the JSON object. No other text."#;

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

Provide a mastering plan with schema_version 1 and a params object containing: eq, compression, limiter, stereo, target_lufs."#,
        target_lufs = opts.target_lufs,
        no_limiter = opts.no_limiter,
    )
}

fn parse_mastering_params(response: &str) -> Result<MasteringParams> {
    // Versioned plans are the production contract. Legacy bare parameters are
    // accepted during migration so older local models remain usable.
    if let Ok(plan) = serde_json::from_str::<MasteringPlan>(response) {
        return plan.validate_version();
    }
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

    if let Ok(plan) = serde_json::from_str::<MasteringPlan>(json_str) {
        return plan.validate_version();
    }
    serde_json::from_str::<MasteringParams>(json_str).context(
        "Failed to parse AI response as a mastering plan. The advisor returned an incompatible format.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mastering_params_valid_json() {
        let response = r#"{
  "eq": [],
  "compression": {"threshold_db": -20.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 100.0, "knee_db": 2.0, "makeup_gain_db": 0.0},
  "limiter": {"enabled": true, "ceiling_db": -1.0, "release_ms": 100.0},
  "stereo": {"width": 1.0, "balance": 0.0},
  "target_lufs": -14.0
}"#;
        let result = parse_mastering_params(response);
        assert!(result.is_ok(), "Should parse valid JSON");
        let params = result.unwrap();
        assert_eq!(params.target_lufs, -14.0);
    }

    #[test]
    fn test_parse_mastering_params_with_markdown() {
        let response = r#"Here are the parameters:

```json
{
  "eq": [],
  "compression": {"threshold_db": -20.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 100.0, "knee_db": 2.0, "makeup_gain_db": 0.0},
  "limiter": {"enabled": true, "ceiling_db": -1.0, "release_ms": 100.0},
  "stereo": {"width": 1.0, "balance": 0.0},
  "target_lufs": -14.0
}
```

I recommend using these settings."#;
        let result = parse_mastering_params(response);
        assert!(result.is_ok(), "Should parse JSON from markdown");
        let params = result.unwrap();
        assert_eq!(params.target_lufs, -14.0);
    }

    #[test]
    fn test_parse_mastering_params_invalid() {
        let response = "I couldn't process that audio file.";
        let result = parse_mastering_params(response);
        assert!(result.is_err(), "Should fail on non-JSON response");
    }

    #[test]
    fn rejects_unknown_mastering_plan_version() {
        let response = r#"{"schema_version":99,"params":{"eq":[],"compression":{"threshold_db":-20.0,"ratio":2.0,"attack_ms":10.0,"release_ms":100.0,"knee_db":2.0,"makeup_gain_db":0.0},"limiter":{"enabled":true,"ceiling_db":-1.0,"release_ms":100.0},"stereo":{"width":1.0,"balance":0.0},"target_lufs":-14.0}}"#;
        assert!(parse_mastering_params(response).is_err());
    }

    #[test]
    fn test_build_mastering_prompt_basic() {
        let opts = MasteringOptions {
            input_path: std::path::PathBuf::from("/test/input.wav"),
            output_path: std::path::PathBuf::from("/test/output.wav"),
            reference_path: None,
            bit_depth: 24,
            delivery_format: crate::types::AudioFormat::Wav,
            target_lufs: -16.0,
            no_limiter: false,
            preset: None,
            pre_analysis: None,
            reference_analysis: None,
            control: crate::control::ProcessingControl::default(),
        };

        let prompt = build_mastering_prompt("{}", &opts);
        assert!(prompt.contains("16"), "Should contain LUFS value");
        assert!(prompt.contains("false"), "Should contain no_limiter flag");
        assert!(
            !prompt.contains("Preset"),
            "Should not contain preset when None"
        );
    }

    #[test]
    fn test_build_mastering_prompt_with_preset() {
        let opts = MasteringOptions {
            input_path: std::path::PathBuf::from("/test/input.wav"),
            output_path: std::path::PathBuf::from("/test/output.wav"),
            reference_path: None,
            bit_depth: 24,
            delivery_format: crate::types::AudioFormat::Wav,
            target_lufs: -14.0,
            no_limiter: true,
            preset: Some(crate::types::Preset::Streaming),
            pre_analysis: None,
            reference_analysis: None,
            control: crate::control::ProcessingControl::default(),
        };

        let prompt = build_mastering_prompt("{}", &opts);
        // Preset Display is lowercase (streaming)
        assert!(prompt.contains("streaming"), "Should contain preset name");
        assert!(prompt.contains("14"), "Should contain LUFS value");
        assert!(prompt.contains("true"), "Should contain no_limiter flag");
    }

    #[test]
    fn test_backend_with_provider() {
        let backend = AiBackend::new(&Config::default());
        let ollama_backend = backend.clone().with_provider(AiProvider::Ollama);
        assert_eq!(ollama_backend.provider, AiProvider::Ollama);
    }

    #[test]
    fn test_backend_check_available() {
        let backend = AiBackend::new(&Config::default());
        // This test just verifies the function doesn't panic
        // In a real scenario, we'd mock the Python process
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(backend.check_available());
        // Result depends on whether Python is installed, so we just check it returns
        drop(result);
    }
}
