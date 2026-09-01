pub mod ai;
pub mod local_ml;
pub mod matchering;
pub mod native;

use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::MasteringError;
use crate::types::{AudioAnalysis, Backend, MasteringParams};

/// Options passed to any mastering backend.
#[derive(Debug, Clone)]
pub struct MasteringOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub reference_path: Option<PathBuf>,
    pub bit_depth: u16,
    pub delivery_format: crate::types::AudioFormat,
    pub target_lufs: f64,
    pub no_limiter: bool,
    pub preset: Option<crate::types::Preset>,
    pub pre_analysis: Option<AudioAnalysis>,
    pub reference_analysis: Option<AudioAnalysis>,
    pub control: crate::control::ProcessingControl,
}

/// Result from a mastering backend.
#[derive(Debug, Clone)]
pub struct BackendOutput {
    pub output_path: PathBuf,
    pub params_applied: Option<MasteringParams>,
    pub backend_name: String,
    pub message: String,
    pub warnings: Vec<String>,
}

/// Enum-dispatch mastering engine — avoids async trait objects.
pub enum MasteringEngine {
    Native(native::NativeBackend),
    Matchering(matchering::MatcheringBackend),
    Ai(ai::AiBackend),
    LocalMl(local_ml::LocalMlBackend),
}

impl MasteringEngine {
    pub fn from_config(backend: crate::types::Backend, config: &Config) -> Self {
        match backend {
            crate::types::Backend::Native => MasteringEngine::Native(native::NativeBackend::new()),
            crate::types::Backend::Matchering => {
                MasteringEngine::Matchering(matchering::MatcheringBackend::new(config))
            }
            crate::types::Backend::Ai => MasteringEngine::Ai(ai::AiBackend::new(config)),
            crate::types::Backend::LocalMl => {
                MasteringEngine::LocalMl(local_ml::LocalMlBackend::new(config))
            }
            crate::types::Backend::Auto => MasteringEngine::Native(native::NativeBackend::new()),
        }
    }

    pub async fn process(&self, opts: &MasteringOptions) -> Result<BackendOutput> {
        match self {
            MasteringEngine::Native(b) => b.process(opts).await,
            MasteringEngine::Matchering(b) => b.process(opts).await,
            MasteringEngine::Ai(b) => b.process(opts).await,
            MasteringEngine::LocalMl(b) => b.process(opts).await,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            MasteringEngine::Native(_) => "native",
            MasteringEngine::Matchering(_) => "matchering",
            MasteringEngine::Ai(_) => "ai",
            MasteringEngine::LocalMl(_) => "local-ml",
        }
    }

    pub async fn check_available(&self) -> Result<bool> {
        match self {
            MasteringEngine::Native(b) => b.check_available().await,
            MasteringEngine::Matchering(b) => b.check_available().await,
            MasteringEngine::Ai(b) => b.check_available().await,
            MasteringEngine::LocalMl(b) => b.check_available().await,
        }
    }

    /// Get the next fallback backend in the chain.
    ///
    /// Fallback order: AI → Matchering → LocalMl
    pub fn fallback(&self, config: &Config) -> Option<Self> {
        match self {
            MasteringEngine::Native(_) => None,
            MasteringEngine::Ai(_) => Some(MasteringEngine::Matchering(
                matchering::MatcheringBackend::new(config),
            )),
            MasteringEngine::Matchering(_) => Some(MasteringEngine::LocalMl(
                local_ml::LocalMlBackend::new(config),
            )),
            MasteringEngine::LocalMl(_) => None, // No more fallbacks
        }
    }

    /// Process with automatic fallback on failure.
    ///
    /// Attempts the current backend, and if it fails, tries fallback backends
    /// in order: AI → Matchering → LocalMl.
    pub async fn process_with_fallback(
        &self,
        opts: &MasteringOptions,
        config: &Config,
    ) -> Result<BackendOutput, MasteringError> {
        // Try current backend first
        match self.process(opts).await {
            Ok(output) => Ok(output),
            Err(e) => {
                let error: MasteringError = e.into();

                // Check if we can fallback
                if let Some(fallback_engine) = self.fallback(config) {
                    tracing::warn!(
                        "Backend {} failed, trying fallback: {}",
                        self.name(),
                        fallback_engine.name()
                    );

                    // Try fallback
                    fallback_engine.process(opts).await.map_err(|e| {
                        MasteringError::BackendError {
                            backend: fallback_engine.name().to_string(),
                            message: e.to_string(),
                            can_fallback: false, // Already tried fallback
                        }
                    })
                } else {
                    // No fallback available
                    Err(error)
                }
            }
        }
    }

    /// Create a new engine from a Backend type with fallback chain support.
    pub fn with_fallback(backend: Backend, config: &Config) -> Self {
        match backend {
            Backend::Auto | Backend::Native => Self::Native(native::NativeBackend::new()),
            Backend::Matchering => Self::Matchering(matchering::MatcheringBackend::new(config)),
            Backend::Ai => Self::Ai(ai::AiBackend::new(config)),
            Backend::LocalMl => Self::LocalMl(local_ml::LocalMlBackend::new(config)),
        }
    }
}
