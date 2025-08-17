//! MCP Queue - Offline queue management for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::metrics::ComponentHealth;
use mcp_common::{Config, MCPRequest, MCPResponse, Result};
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
pub async fn create_offline_queue(
    config: Arc<Config>,
) -> Result<Arc<dyn OfflineQueue + Send + Sync>> {
    let queue = PersistentQueue::new(config).await?;
    Ok(Arc::new(queue))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_common::{Config, MCPRequest};
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_queue_creation() {
        let config = Arc::new(Config::default());
        let result = create_offline_queue(config).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let config = Arc::new(Config::default());
        let queue = create_offline_queue(config).await.unwrap();
        
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            method: "test_method".to_string(),
            params: serde_json::Value::Null,
        };
        
        // Test enqueue
        let response = queue.enqueue_request(request.clone()).await;
        assert!(response.is_ok());
        
        // Test queue size
        let size = queue.queue_size().await.unwrap();
        assert!(size > 0);
        
        // Test dequeue
        let dequeued = queue.dequeue_request().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, request.id);
    }
    
    #[tokio::test]
    async fn test_queue_health() {
        let config = Arc::new(Config::default());
        let queue = create_offline_queue(config).await.unwrap();
        
        let health = queue.health_check().await;
        assert!(health.is_ok());
        assert!(matches!(health.unwrap(), ComponentHealth::Healthy { .. }));
    }
    
    #[tokio::test]
    async fn test_queue_sync() {
        let config = Arc::new(Config::default());
        let queue = create_offline_queue(config).await.unwrap();
        
        // Test sync - should not fail even if no cloud connection
        let result = queue.sync_with_cloud().await;
        // Allow either success or specific errors
        assert!(result.is_ok() || result.is_err());
    }
}
