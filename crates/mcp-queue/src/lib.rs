//! MCP Queue - Offline queue management for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::{Result, Error, MCPRequest, MCPResponse, ComponentHealth, Config};
use std::sync::Arc;

/// Offline queue trait for managing queued requests
#[async_trait]
pub trait OfflineQueue {
    /// Enqueue a request for later processing
    async fn enqueue_request(&self, request: MCPRequest) -> Result<MCPResponse>;
    
    /// Dequeue a request for processing
    async fn dequeue_request(&self) -> Result<Option<MCPRequest>>;
    
    /// Get queue size
    async fn queue_size(&self) -> Result<u32>;
    
    /// Sync queued requests with cloud
    async fn sync_with_cloud(&self) -> Result<()>;
    
    /// Get health status
    async fn health_check(&self) -> Result<ComponentHealth>;
    
    /// Shutdown the queue
    async fn shutdown(&self) -> Result<()>;
}

mod persistent_queue;

pub use persistent_queue::PersistentQueue;

/// Create a new offline queue instance
pub async fn create_offline_queue(config: Arc<Config>) -> Result<Arc<dyn OfflineQueue + Send + Sync>> {
    let queue = PersistentQueue::new(config).await?;
    Ok(Arc::new(queue))
}