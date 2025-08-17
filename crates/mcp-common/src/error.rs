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
    Serialization(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Generic error: {0}")]
    Generic(String),
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
            Error::Internal(_) => "internal",
            Error::Generic(_) => "generic",
        }
    }

    /// Get severity level for error (1-5, where 5 is most severe)
    pub fn severity(&self) -> u8 {
        match self {
            Error::Security(_) => 5,
            Error::Internal(_) => 5,
            Error::Configuration(_) => 4,
            Error::Model(_) => 4,
            Error::ResourceExhausted(_) => 3,
            Error::Queue(_) => 3,
            Error::Routing(_) => 3,
            Error::Network(_) => 2,
            Error::Timeout(_) => 2,
            Error::Telemetry(_) => 1,
            Error::Memory(_) => 4,
            Error::InvalidRequest(_) => 2,
            Error::Serialization(_) => 2,
            Error::Generic(_) => 3,
        }
    }

    /// Check if error should trigger alerts
    pub fn should_alert(&self) -> bool {
        self.severity() >= 4
    }

    /// Get suggested retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> Option<u64> {
        if !self.is_retryable() {
            return None;
        }

        match self {
            Error::Network(_) => Some(1000), // 1 second
            Error::Timeout(_) => Some(2000), // 2 seconds
            Error::ResourceExhausted(_) => Some(5000), // 5 seconds
            _ => None,
        }
    }

    /// Get maximum retry attempts for this error type
    pub fn max_retries(&self) -> u32 {
        match self {
            Error::Network(_) => 3,
            Error::Timeout(_) => 2,
            Error::ResourceExhausted(_) => 5,
            _ => 0,
        }
    }

    /// Create error context with additional information
    pub fn with_context(self, context: &str) -> Self {
        match self {
            Error::Configuration(msg) => Error::Configuration(format!("{}: {}", context, msg)),
            Error::Network(msg) => Error::Network(format!("{}: {}", context, msg)),
            Error::Model(msg) => Error::Model(format!("{}: {}", context, msg)),
            Error::Security(msg) => Error::Security(format!("{}: {}", context, msg)),
            Error::Queue(msg) => Error::Queue(format!("{}: {}", context, msg)),
            Error::Routing(msg) => Error::Routing(format!("{}: {}", context, msg)),
            Error::Telemetry(msg) => Error::Telemetry(format!("{}: {}", context, msg)),
            Error::ResourceExhausted(msg) => Error::ResourceExhausted(format!("{}: {}", context, msg)),
            Error::InvalidRequest(msg) => Error::InvalidRequest(format!("{}: {}", context, msg)),
            Error::Timeout(msg) => Error::Timeout(format!("{}: {}", context, msg)),
            Error::Memory(msg) => Error::Memory(format!("{}: {}", context, msg)),
            Error::Internal(msg) => Error::Internal(format!("{}: {}", context, msg)),
            other => other, // Cannot add context to these types
        }
    }
}

/// Specialized result types for different operations
pub type ConfigResult<T> = std::result::Result<T, Error>;
pub type NetworkResult<T> = std::result::Result<T, Error>;
pub type ModelResult<T> = std::result::Result<T, Error>;
pub type SecurityResult<T> = std::result::Result<T, Error>;
pub type QueueResult<T> = std::result::Result<T, Error>;

/// Error recovery strategy
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry { max_attempts: u32, base_delay_ms: u64 },
    /// Fallback to alternative method
    Fallback(String),
    /// Circuit breaker - fail fast for some time
    CircuitBreaker { timeout_ms: u64 },
    /// Graceful degradation
    Degrade(String),
    /// No recovery possible
    NoRecovery,
}

impl Error {
    /// Get recommended recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Error::Network(_) => RecoveryStrategy::Retry { 
                max_attempts: 3, 
                base_delay_ms: 1000 
            },
            Error::Timeout(_) => RecoveryStrategy::Retry { 
                max_attempts: 2, 
                base_delay_ms: 2000 
            },
            Error::ResourceExhausted(_) => RecoveryStrategy::CircuitBreaker { 
                timeout_ms: 30000 
            },
            Error::Model(_) => RecoveryStrategy::Fallback("cloud".to_string()),
            Error::Routing(_) => RecoveryStrategy::Fallback("queue".to_string()),
            Error::Security(_) => RecoveryStrategy::NoRecovery,
            Error::Configuration(_) => RecoveryStrategy::NoRecovery,
            Error::Queue(_) => RecoveryStrategy::Degrade("skip_offline_queue".to_string()),
            _ => RecoveryStrategy::Retry { 
                max_attempts: 1, 
                base_delay_ms: 500 
            },
        }
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Configuration(s) => Error::Configuration(s.clone()),
            Error::Network(s) => Error::Network(s.clone()),
            Error::Model(s) => Error::Model(s.clone()),
            Error::Security(s) => Error::Security(s.clone()),
            Error::Queue(s) => Error::Queue(s.clone()),
            Error::Routing(s) => Error::Routing(s.clone()),
            Error::Telemetry(s) => Error::Telemetry(s.clone()),
            Error::ResourceExhausted(s) => Error::ResourceExhausted(s.clone()),
            Error::InvalidRequest(s) => Error::InvalidRequest(s.clone()),
            Error::Timeout(s) => Error::Timeout(s.clone()),
            Error::Serialization(s) => Error::Serialization(s.clone()),
            Error::Memory(s) => Error::Memory(s.clone()),
            Error::Internal(s) => Error::Internal(s.clone()),
            Error::Generic(s) => Error::Generic(s.clone()),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Generic(err.to_string())
    }
}
