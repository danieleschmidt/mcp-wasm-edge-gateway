//! MCP Security - Security and cryptography for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::metrics::ComponentHealth;
use mcp_common::{Config, MCPRequest, Result};
use std::sync::Arc;

/// Security manager trait for request validation and cryptographic operations
#[async_trait]
pub trait SecurityManager {
    /// Validate an incoming request
    async fn validate_request(&self, request: &MCPRequest) -> Result<()>;

    /// Encrypt data
    async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt data
    async fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>>;

    /// Get health status
    async fn health_check(&self) -> Result<ComponentHealth>;

    /// Shutdown the security manager
    async fn shutdown(&self) -> Result<()>;
}

mod standard_security;

pub use standard_security::StandardSecurityManager;

/// Create a new security manager instance
pub async fn create_security_manager(
    config: Arc<Config>,
) -> Result<Arc<dyn SecurityManager + Send + Sync>> {
    let manager = StandardSecurityManager::new(config).await?;
    Ok(Arc::new(manager))
}
