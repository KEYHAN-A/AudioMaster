use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f64,
    pub bit_depth: Option<u16>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub metadata: AudioMetadata,
    pub lufs_integrated: f64,
    pub lufs_short_term_max: f64,
    pub rms_db: f64,
    pub peak_db: f64,
    pub true_peak_db: f64,
    pub dynamic_range_db: f64,
    pub stereo_width: f64,
    pub frequency_bands: FrequencyBands,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyBands {
    pub sub_bass: f64,
    pub bass: f64,
    pub low_mid: f64,
    pub mid: f64,
    pub upper_mid: f64,
    pub presence: f64,
    pub brilliance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteringParams {
    pub eq: Vec<EqBand>,
    pub compression: CompressionParams,
    pub limiter: LimiterParams,
    pub stereo: StereoParams,
    pub target_lufs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
    pub band_type: EqBandType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBandType {
    LowShelf,
    HighShelf,
    Peak,
    LowPass,
    HighPass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionParams {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_gain_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterParams {
    pub enabled: bool,
    pub ceiling_db: f64,
    pub release_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StereoParams {
    pub width: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteringResult {
    pub output_path: PathBuf,
    pub backend_used: String,
    pub pre_analysis: Option<AudioAnalysis>,
    pub post_analysis: Option<AudioAnalysis>,
    pub params_applied: Option<MasteringParams>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Auto,
    Matchering,
    Ai,
    LocalMl,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Auto => write!(f, "auto"),
            Backend::Matchering => write!(f, "matchering"),
            Backend::Ai => write!(f, "ai"),
            Backend::LocalMl => write!(f, "local-ml"),
        }
    }
}

impl std::str::FromStr for Backend {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Backend::Auto),
            "matchering" => Ok(Backend::Matchering),
            "ai" => Ok(Backend::Ai),
            "local-ml" | "local_ml" | "localml" => Ok(Backend::LocalMl),
            _ => anyhow::bail!("Unknown backend: {s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Ollama,
    KeyhanStudio,
    OpenAi,
    Anthropic,
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiProvider::Ollama => write!(f, "ollama"),
            AiProvider::KeyhanStudio => write!(f, "keyhanstudio"),
            AiProvider::OpenAi => write!(f, "openai"),
            AiProvider::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl std::str::FromStr for AiProvider {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(AiProvider::Ollama),
            "keyhanstudio" | "keyhan" => Ok(AiProvider::KeyhanStudio),
            "openai" => Ok(AiProvider::OpenAi),
            "anthropic" | "claude" => Ok(AiProvider::Anthropic),
            _ => anyhow::bail!("Unknown AI provider: {s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Wav => write!(f, "wav"),
            AudioFormat::Flac => write!(f, "flac"),
            AudioFormat::Mp3 => write!(f, "mp3"),
        }
    }
}

impl std::str::FromStr for AudioFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wav" => Ok(AudioFormat::Wav),
            "flac" => Ok(AudioFormat::Flac),
            "mp3" => Ok(AudioFormat::Mp3),
            _ => anyhow::bail!("Unknown audio format: {s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Streaming,
    Cd,
    Vinyl,
    Loud,
}

impl Preset {
    pub fn target_lufs(&self) -> f64 {
        match self {
            Preset::Streaming => -14.0,
            Preset::Cd => -9.0,
            Preset::Vinyl => -12.0,
            Preset::Loud => -6.0,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Preset::Streaming => "Optimized for streaming platforms (-14 LUFS)",
            Preset::Cd => "CD-level loudness (-9 LUFS)",
            Preset::Vinyl => "Vinyl-friendly dynamics (-12 LUFS)",
            Preset::Loud => "Maximum loudness (-6 LUFS)",
        }
    }
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Preset::Streaming => write!(f, "streaming"),
            Preset::Cd => write!(f, "cd"),
            Preset::Vinyl => write!(f, "vinyl"),
            Preset::Loud => write!(f, "loud"),
        }
    }
}

impl std::str::FromStr for Preset {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "streaming" => Ok(Preset::Streaming),
            "cd" => Ok(Preset::Cd),
            "vinyl" => Ok(Preset::Vinyl),
            "loud" => Ok(Preset::Loud),
            _ => anyhow::bail!("Unknown preset: {s}. Available: streaming, cd, vinyl, loud"),
        }
    }
}
