pub mod ai;
pub mod local_ml;
pub mod matchering;

use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::types::MasteringParams;

/// Options passed to any mastering backend.
#[derive(Debug, Clone)]
pub struct MasteringOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub reference_path: Option<PathBuf>,
    pub bit_depth: u16,
    pub target_lufs: f64,
    pub no_limiter: bool,
    pub preset: Option<crate::types::Preset>,
}

/// Result from a mastering backend.
#[derive(Debug, Clone)]
pub struct BackendOutput {
    pub output_path: PathBuf,
    pub params_applied: Option<MasteringParams>,
    pub backend_name: String,
    pub message: String,
}

/// Enum-dispatch mastering engine — avoids async trait objects.
pub enum MasteringEngine {
    Matchering(matchering::MatcheringBackend),
    Ai(ai::AiBackend),
    LocalMl(local_ml::LocalMlBackend),
}

impl MasteringEngine {
    pub fn from_config(backend: crate::types::Backend, config: &Config) -> Self {
        match backend {
            crate::types::Backend::Matchering => {
                MasteringEngine::Matchering(matchering::MatcheringBackend::new(config))
            }
            crate::types::Backend::Ai => MasteringEngine::Ai(ai::AiBackend::new(config)),
            crate::types::Backend::LocalMl => {
                MasteringEngine::LocalMl(local_ml::LocalMlBackend::new(config))
            }
            crate::types::Backend::Auto => {
                // Auto is resolved by the pipeline before reaching here; default to AI
                MasteringEngine::Ai(ai::AiBackend::new(config))
            }
        }
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        match self {
            MasteringEngine::Matchering(b) => b.process(opts).await,
            MasteringEngine::Ai(b) => b.process(opts).await,
            MasteringEngine::LocalMl(b) => b.process(opts).await,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            MasteringEngine::Matchering(_) => "matchering",
            MasteringEngine::Ai(_) => "ai",
            MasteringEngine::LocalMl(_) => "local-ml",
        }
    }

    pub async fn check_available(&self) -> Result<bool> {
        match self {
            MasteringEngine::Matchering(b) => b.check_available().await,
            MasteringEngine::Ai(b) => b.check_available().await,
            MasteringEngine::LocalMl(b) => b.check_available().await,
        }
    }
}
