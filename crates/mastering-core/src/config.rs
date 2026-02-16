use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::types::{AiProvider, AudioFormat, Backend};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub backends: BackendsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_backend")]
    pub default_backend: Backend,
    #[serde(default = "default_bit_depth")]
    pub default_bit_depth: u16,
    #[serde(default = "default_format")]
    pub default_format: AudioFormat,
    #[serde(default = "default_target_lufs")]
    pub target_lufs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_provider")]
    pub default_provider: AiProvider,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub keyhanstudio: KeyhanStudioConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyhanStudioConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_openai_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_anthropic_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendsConfig {
    #[serde(default)]
    pub matchering: MatcheringConfig,
    #[serde(default)]
    pub local_ml: LocalMlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcheringConfig {
    #[serde(default = "default_python_path")]
    pub python_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMlConfig {
    #[serde(default = "default_python_path")]
    pub python_path: String,
    #[serde(default = "default_ml_model")]
    pub default_model: String,
}

// --- Default value functions ---

fn default_backend() -> Backend {
    Backend::Auto
}
fn default_bit_depth() -> u16 {
    24
}
fn default_format() -> AudioFormat {
    AudioFormat::Wav
}
fn default_target_lufs() -> f64 {
    -14.0
}
fn default_ai_provider() -> AiProvider {
    AiProvider::Ollama
}
fn default_ollama_endpoint() -> String {
    "http://localhost:11434".into()
}
fn default_ollama_model() -> String {
    "llama3".into()
}
fn default_openai_model() -> String {
    "gpt-4o".into()
}
fn default_anthropic_model() -> String {
    "claude-sonnet-4-20250514".into()
}
fn default_python_path() -> String {
    "python3".into()
}
fn default_ml_model() -> String {
    "deepafx-st".into()
}

// --- Default trait impls ---

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ai: AiConfig::default(),
            backends: BackendsConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_backend: default_backend(),
            default_bit_depth: default_bit_depth(),
            default_format: default_format(),
            target_lufs: default_target_lufs(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: default_ai_provider(),
            ollama: OllamaConfig::default(),
            keyhanstudio: KeyhanStudioConfig::default(),
            openai: OpenAiConfig::default(),
            anthropic: AnthropicConfig::default(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: default_ollama_endpoint(),
            model: default_ollama_model(),
        }
    }
}

impl Default for KeyhanStudioConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: String::new(),
        }
    }
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_openai_model(),
        }
    }
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_anthropic_model(),
        }
    }
}

impl Default for BackendsConfig {
    fn default() -> Self {
        Self {
            matchering: MatcheringConfig::default(),
            local_ml: LocalMlConfig::default(),
        }
    }
}

impl Default for MatcheringConfig {
    fn default() -> Self {
        Self {
            python_path: default_python_path(),
        }
    }
}

impl Default for LocalMlConfig {
    fn default() -> Self {
        Self {
            python_path: default_python_path(),
            default_model: default_ml_model(),
        }
    }
}

// --- Config operations ---

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("mastering");
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("Reading config: {}", path.display()))?;
        let config: Config =
            toml::from_str(&contents).with_context(|| format!("Parsing config: {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Creating config directory: {}", parent.display()))?;
        }
        let contents =
            toml::to_string_pretty(self).context("Serializing config")?;
        std::fs::write(path, &contents)
            .with_context(|| format!("Writing config: {}", path.display()))?;
        Ok(())
    }

    pub fn python_scripts_dir() -> PathBuf {
        // 1. Explicit env var (set by Tauri app or user)
        if let Ok(dir) = std::env::var("MASTERING_PROJECT_DIR") {
            let base = PathBuf::from(&dir);
            // Direct python/ subfolder
            let p = base.join("python");
            if p.exists() {
                return p;
            }
            // Tauri bundles ../python as _up_/python
            let up = base.join("_up_").join("python");
            if up.exists() {
                return up;
            }
        }

        let exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe.parent().unwrap_or(Path::new("."));

        // 2. macOS .app bundle: Contents/Resources/python
        let resources = exe_dir.join("../Resources/python");
        if resources.exists() {
            return resources;
        }

        // 3. Next to the binary
        let beside_exe = exe_dir.join("python");
        if beside_exe.exists() {
            return beside_exe;
        }

        // 4. Workspace dev layout (cargo build from root)
        for depth in &["../../..", "../.."] {
            let workspace = exe_dir.join(depth).join("python");
            if workspace.exists() {
                return workspace;
            }
        }

        // 5. Current working directory
        let cwd = std::env::current_dir().unwrap_or_default().join("python");
        if cwd.exists() {
            return cwd;
        }

        PathBuf::from("python")
    }
}
