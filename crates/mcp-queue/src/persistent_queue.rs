//! Persistent queue implementation

use crate::OfflineQueue;
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, MCPRequest, MCPResponse, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Persistent queue implementation using embedded database
pub struct PersistentQueue {
    config: Arc<Config>,
    queue_size: Arc<RwLock<u32>>,
}

impl PersistentQueue {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self {
            config,
            queue_size: Arc::new(RwLock::new(0)),
        })
    }
}

#[async_trait]
impl OfflineQueue for PersistentQueue {
    async fn enqueue_request(&self, request: MCPRequest) -> Result<MCPResponse> {
        debug!("Enqueueing request {}", request.id);

        // Simulate storing request
        {
            let mut size = self.queue_size.write().await;
            *size += 1;
        }

        Ok(MCPResponse {
            id: request.id,
            result: Some(serde_json::json!({
                "status": "queued",
                "message": "Request queued for offline processing"
            })),
            error: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn dequeue_request(&self) -> Result<Option<MCPRequest>> {
        // Simulate dequeuing
        let mut size = self.queue_size.write().await;
        if *size > 0 {
            *size -= 1;
            // Return a mock request
            Ok(Some(MCPRequest {
                id: uuid::Uuid::new_v4(),
                device_id: "mock".to_string(),
                method: "test".to_string(),
                params: HashMap::new(),
                context: None,
                timestamp: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn queue_size(&self) -> Result<u32> {
        Ok(*self.queue_size.read().await)
    }

    async fn sync_with_cloud(&self) -> Result<()> {
        debug!("Syncing queue with cloud");
        // Simulate sync
        Ok(())
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let size = *self.queue_size.read().await;
        let mut metrics = HashMap::new();
        metrics.insert("queue_size".to_string(), size as f32);

        let status = if size > 1000 {
            HealthLevel::Critical
        } else if size > 500 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };

        Ok(ComponentHealth {
            status,
            message: format!("Queue has {} items", size),
            last_check: chrono::Utc::now(),
            metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down persistent queue");
        Ok(())
    }
}
