//! Cooperative job cancellation and observable progress.

use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub stage: String,
    pub fraction: f64,
    pub processed_frames: u64,
    pub total_frames: Option<u64>,
    pub message: String,
}

type ProgressCallback = dyn Fn(JobProgress) + Send + Sync + 'static;

#[derive(Clone, Default)]
pub struct ProcessingControl {
    cancelled: Arc<AtomicBool>,
    callback: Option<Arc<ProgressCallback>>,
}

impl std::fmt::Debug for ProcessingControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessingControl")
            .field("cancelled", &self.is_cancelled())
            .field("has_callback", &self.callback.is_some())
            .finish()
    }
}

impl ProcessingControl {
    pub fn with_callback(callback: impl Fn(JobProgress) + Send + Sync + 'static) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            callback: Some(Arc::new(callback)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check_cancelled(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.is_cancelled(), "Mastering job was cancelled");
        Ok(())
    }

    pub fn report(
        &self,
        stage: impl Into<String>,
        fraction: f64,
        processed_frames: u64,
        total_frames: Option<u64>,
        message: impl Into<String>,
    ) {
        if let Some(callback) = &self.callback {
            callback(JobProgress {
                stage: stage.into(),
                fraction: fraction.clamp(0.0, 1.0),
                processed_frames,
                total_frames,
                message: message.into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_shared_between_clones() {
        let first = ProcessingControl::default();
        let second = first.clone();
        first.cancel();
        assert!(second.check_cancelled().is_err());
    }
}
