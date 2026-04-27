//! Centralized error types for AudioMaster.
//!
//! All errors should implement `Into<MasteringError>` for consistent error handling
//! and user-friendly messaging.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for AudioMaster mastering operations.
///
/// Each variant includes enough context to display user-friendly error messages
/// with suggested recovery actions.
#[derive(Error, Debug)]
pub enum MasteringError {
    /// Network-related errors with retry capability
    #[error("Network operation failed: {message}")]
    NetworkTimeout {
        message: String,
        can_retry: bool,
        suggested_action: String,
    },

    /// Audio decoding and format errors
    #[error("Failed to decode audio file '{file}': {reason}")]
    AudioDecodeFailed {
        file: String,
        reason: String,
        suggested_action: String,
    },

    /// Python interpreter or dependency errors
    #[error("Python environment unavailable: {message}")]
    PythonUnavailable {
        message: String,
        suggested_action: String,
    },

    /// API quota or rate limit errors
    #[error("API quota exceeded for {provider}")]
    ApiQuotaExceeded {
        provider: String,
        reset_time: String,
        suggested_action: String,
    },

    /// Invalid user input or configuration
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        message: String,
        config_key: Option<String>,
    },

    /// File I/O errors
    #[error("File operation failed: {message}")]
    FileIo {
        message: String,
        path: Option<PathBuf>,
    },

    /// Backend-specific errors
    #[error("Backend '{backend}' failed: {message}")]
    BackendError {
        backend: String,
        message: String,
        can_fallback: bool,
    },

    /// Processing errors
    #[error("Audio processing failed: {message}")]
    ProcessingError {
        message: String,
        stage: String,
    },

    /// Validation errors
    #[error("Validation failed: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,
    },

    /// Generic errors with context
    #[error("Operation failed: {message}")]
    Generic {
        message: String,
        source: Option<anyhow::Error>,
    },
}

impl MasteringError {
    /// Returns a user-friendly error message.
    pub fn user_message(&self) -> String {
        match self {
            MasteringError::NetworkTimeout {
                suggested_action, ..
            } => {
                format!("{}\n\nSuggestion: {}", self, suggested_action)
            }
            MasteringError::AudioDecodeFailed {
                suggested_action, ..
            } => {
                format!("{}\n\nSuggestion: {}", self, suggested_action)
            }
            MasteringError::PythonUnavailable {
                suggested_action, ..
            } => {
                format!("{}\n\nSuggestion: {}", self, suggested_action)
            }
            MasteringError::ApiQuotaExceeded {
                suggested_action, ..
            } => {
                format!("{}\n\nSuggestion: {}", self, suggested_action)
            }
            _ => self.to_string(),
        }
    }

    /// Returns whether this error is recoverable (can retry).
    pub fn can_retry(&self) -> bool {
        match self {
            MasteringError::NetworkTimeout { can_retry, .. } => *can_retry,
            MasteringError::ApiQuotaExceeded { .. } => false,
            MasteringError::AudioDecodeFailed { .. } => false,
            MasteringError::PythonUnavailable { .. } => false,
            MasteringError::InvalidConfig { .. } => false,
            MasteringError::FileIo { .. } => false,
            MasteringError::BackendError { can_fallback, .. } => *can_fallback,
            MasteringError::ProcessingError { .. } => true,
            MasteringError::ValidationError { .. } => true,
            MasteringError::Generic { .. } => false,
        }
    }

    /// Returns whether this error supports fallback to an alternative backend.
    pub fn can_fallback(&self) -> bool {
        matches!(
            self,
            MasteringError::BackendError {
                can_fallback: true,
                ..
            } | MasteringError::NetworkTimeout { .. }
                | MasteringError::ApiQuotaExceeded { .. }
        )
    }

    /// Creates a network timeout error.
    pub fn network_timeout(message: impl Into<String>, can_retry: bool) -> Self {
        let message = message.into();
        let suggested_action = if can_retry {
            "Check your internet connection and try again.".to_string()
        } else {
            "The service may be temporarily unavailable. Please try again later.".to_string()
        };
        MasteringError::NetworkTimeout {
            message,
            can_retry,
            suggested_action,
        }
    }

    /// Creates an audio decode failed error.
    pub fn audio_decode_failed(file: impl Into<String>, reason: impl Into<String>) -> Self {
        let file = file.into();
        let reason = reason.into();
        let suggested_action = if reason.contains("format") || reason.contains("codec") {
            format!(
                "Ensure the file is a supported format (WAV, FLAC, MP3, OGG, M4A)."
            )
        } else if reason.contains("corrupt") || reason.contains("invalid") {
            "The file may be corrupted. Try opening it in another audio application to verify.".to_string()
        } else {
            "Try re-exporting the file from your audio software.".to_string()
        };
        MasteringError::AudioDecodeFailed {
            file,
            reason,
            suggested_action,
        }
    }

    /// Creates a Python unavailable error.
    pub fn python_unavailable(message: impl Into<String>) -> Self {
        let message = message.into();
        let suggested_action = if message.contains("not found") {
            "Install Python 3.8+ and ensure it's in your PATH.".to_string()
        } else if message.contains("module") || message.contains("package") {
            "Install required Python packages: pip install -r python/requirements.txt".to_string()
        } else {
            "Ensure Python 3.8+ is installed and accessible.".to_string()
        };
        MasteringError::PythonUnavailable {
            message,
            suggested_action,
        }
    }

    /// Creates an API quota exceeded error.
    pub fn api_quota_exceeded(provider: impl Into<String>, reset_time: impl Into<String>) -> Self {
        let provider = provider.into();
        let reset_time = reset_time.into();
        let suggested_action = format!(
            "Wait until {} or switch to a different backend/provider.",
            reset_time
        );
        MasteringError::ApiQuotaExceeded {
            provider,
            reset_time,
            suggested_action,
        }
    }

    /// Creates a backend error.
    pub fn backend_error(backend: impl Into<String>, message: impl Into<String>) -> Self {
        MasteringError::BackendError {
            backend: backend.into(),
            message: message.into(),
            can_fallback: true,
        }
    }

    /// Creates a validation error.
    pub fn validation_error(message: impl Into<String>, field: Option<String>) -> Self {
        MasteringError::ValidationError {
            message: message.into(),
            field,
        }
    }
}

/// Conversion from anyhow::Error to MasteringError.
impl From<anyhow::Error> for MasteringError {
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();

        // Detect common error patterns and provide specific error types
        if msg.contains("timeout") || msg.contains("timed out") {
            return MasteringError::network_timeout(msg, true);
        }
        if msg.contains("Python") || msg.contains("python") {
            return MasteringError::python_unavailable(msg);
        }
        if msg.contains("quota") || msg.contains("rate limit") {
            return MasteringError::api_quota_exceeded("API", "unknown");
        }
        if msg.contains("decode") || msg.contains("codec") || msg.contains("format") {
            return MasteringError::audio_decode_failed("unknown", msg);
        }

        MasteringError::Generic {
            message: msg,
            source: Some(err),
        }
    }
}

/// Conversion from std::io::Error to MasteringError.
impl From<std::io::Error> for MasteringError {
    fn from(err: std::io::Error) -> Self {
        MasteringError::FileIo {
            message: err.to_string(),
            path: None,
        }
    }
}

/// Conversion from serde_json::Error to MasteringError.
impl From<serde_json::Error> for MasteringError {
    fn from(err: serde_json::Error) -> Self {
        MasteringError::Generic {
            message: format!("JSON error: {}", err),
            source: None,
        }
    }
}

/// Result type alias for MasteringError.
pub type Result<T> = std::result::Result<T, MasteringError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_user_message() {
        let err = MasteringError::network_timeout("Connection failed", true);
        let msg = err.user_message();
        assert!(msg.contains("Connection failed"));
        assert!(msg.contains("Suggestion:"));
    }

    #[test]
    fn test_can_retry() {
        assert!(MasteringError::network_timeout("test", true).can_retry());
        assert!(!MasteringError::api_quota_exceeded("test", "now").can_retry());
    }

    #[test]
    fn test_can_fallback() {
        assert!(MasteringError::backend_error("test", "failed").can_fallback());
        assert!(MasteringError::network_timeout("test", true).can_fallback());
        assert!(!MasteringError::audio_decode_failed("test", "failed").can_fallback());
    }
}
