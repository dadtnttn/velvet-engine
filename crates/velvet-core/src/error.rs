//! Core error types.

use thiserror::Error;

use crate::plugin::PluginError;

/// Result alias for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Top-level recoverable engine error.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Plugin system failure.
    #[error("plugin error: {0}")]
    Plugin(#[from] PluginError),

    /// Configuration problem.
    #[error("configuration error: {0}")]
    Config(String),

    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization / deserialization.
    #[error("serialization error: {0}")]
    Serde(String),

    /// Resource or path not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Invalid state for the requested operation.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// Generic message for boundary mapping.
    #[error("{0}")]
    Message(String),
}

impl CoreError {
    /// Config error helper.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Message helper.
    pub fn msg(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}

impl From<ron::error::SpannedError> for CoreError {
    fn from(value: ron::error::SpannedError) -> Self {
        Self::Serde(value.to_string())
    }
}

impl From<ron::Error> for CoreError {
    fn from(value: ron::Error) -> Self {
        Self::Serde(value.to_string())
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value.to_string())
    }
}
