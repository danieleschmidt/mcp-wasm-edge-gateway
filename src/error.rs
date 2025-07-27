// Error handling for MCP WASM Edge Gateway

use thiserror::Error;

/// Gateway result type
pub type Result<T> = std::result::Result<T, GatewayError>;

/// Main error type for the gateway
#[derive(Error, Debug)]
pub enum GatewayError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Model loading error
    #[error("Model error: {0}")]
    Model(String),
    
    /// Network error
    #[error("Network error: {0}")]
    Network(String),
    
    /// Security error
    #[error("Security error: {0}")]
    Security(String),
    
    /// Resource exhausted
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
}

impl GatewayError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            GatewayError::Network(_) | GatewayError::ResourceExhausted(_)
        )
    }
}