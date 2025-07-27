//! MCP Models - Model execution engine for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::{Result, Error, MCPRequest, MCPResponse, ComponentHealth, Config, ModelId};
use std::sync::Arc;

/// Model engine trait for executing AI models
#[async_trait]
pub trait ModelEngine {
    /// Process a request using the specified model
    async fn process_request(&self, request: &MCPRequest, model_id: &ModelId) -> Result<MCPResponse>;
    
    /// Load a model into memory
    async fn load_model(&self, model_id: &ModelId) -> Result<()>;
    
    /// Unload a model from memory
    async fn unload_model(&self, model_id: &ModelId) -> Result<()>;
    
    /// Get health status
    async fn health_check(&self) -> Result<ComponentHealth>;
    
    /// Shutdown the model engine
    async fn shutdown(&self) -> Result<()>;
}

mod engine;
mod cache;
mod loaders;

pub use engine::StandardModelEngine;

/// Create a new model engine instance
pub async fn create_model_engine(config: Arc<Config>) -> Result<Arc<dyn ModelEngine + Send + Sync>> {
    let engine = StandardModelEngine::new(config).await?;
    Ok(Arc::new(engine))
}