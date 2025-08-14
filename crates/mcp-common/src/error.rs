//! Error types and result handling for the MCP Edge Gateway

use thiserror::Error;

/// Result type alias for MCP operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for MCP Edge Gateway operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Routing error: {0}")]
    Routing(String),

    #[error("Telemetry error: {0}")]
    Telemetry(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl Error {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(_) | Error::Timeout(_) | Error::ResourceExhausted(_)
        )
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            Error::Configuration(_) => "configuration",
            Error::Network(_) => "network",
            Error::Model(_) => "model",
            Error::Security(_) => "security",
            Error::Queue(_) => "queue",
            Error::Routing(_) => "routing",
            Error::Telemetry(_) => "telemetry",
            Error::ResourceExhausted(_) => "resource",
            Error::InvalidRequest(_) => "request",
            Error::Timeout(_) => "timeout",
            Error::Serialization(_) => "serialization",
            Error::Memory(_) => "memory",
            Error::Generic(_) => "generic",
        }
    }
}
