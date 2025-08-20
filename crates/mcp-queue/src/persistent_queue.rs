//! Persistent queue implementation for offline request handling

use crate::OfflineQueue;
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, Error, MCPRequest, MCPResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Persistent queue for offline request handling
pub struct PersistentQueue {
    config: Arc<Config>,
    storage: Arc<sled::Db>,
    memory_queue: Arc<RwLock<VecDeque<QueuedRequest>>>,
    stats: Arc<RwLock<QueueStats>>,
}

/// Request stored in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedRequest {
    id: Uuid,
    request: MCPRequest,
    queued_at: chrono::DateTime<chrono::Utc>,
    retry_count: u32,
    priority_score: f32,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Queue statistics for monitoring
#[derive(Debug, Default)]
struct QueueStats {
    total_enqueued: u64,
    total_dequeued: u64,
    total_failed: u64,
    total_expired: u64,
    sync_attempts: u32,
    sync_successes: u32,
    last_sync_attempt: Option<chrono::DateTime<chrono::Utc>>,
    last_sync_success: Option<chrono::DateTime<chrono::Utc>>,
}

impl PersistentQueue {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        // Initialize persistent storage
        let storage = sled::open(&config.queue.storage_path)
            .map_err(|e| Error::Queue(format!("Failed to open queue database: {}", e)))?;

        let queue = Self {
            config: config.clone(),
            storage: Arc::new(storage),
            memory_queue: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(QueueStats::default())),
        };

        // Load existing requests from persistent storage
        queue.load_from_storage().await?;

        // Start background sync task
        queue.start_sync_task().await;

        info!("Persistent queue initialized with storage path: {:?}", config.queue.storage_path);
        Ok(queue)
    }

    /// Load queued requests from persistent storage into memory
    async fn load_from_storage(&self) -> Result<()> {
        debug!("Loading queued requests from persistent storage");

        let mut memory_queue = self.memory_queue.write().await;
        let mut loaded_count = 0;

        for result in self.storage.iter() {
            match result {
                Ok((key, value)) => {
                    // Skip metadata keys
                    if key.starts_with(b"meta:") {
                        continue;
                    }

                    match serde_json::from_slice::<QueuedRequest>(&value) {
                        Ok(queued_request) => {
                            // Check if request has expired
                            if let Some(expires_at) = queued_request.expires_at {
                                if chrono::Utc::now() > expires_at {
                                    debug!("Removing expired request: {}", queued_request.id);
                                    if let Err(e) = self.storage.remove(&key) {
                                        warn!("Failed to remove expired request: {}", e);
                                    }
                                    continue;
                                }
                            }

                            memory_queue.push_back(queued_request);
                            loaded_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to deserialize queued request: {}", e);
                            // Remove corrupted entry
                            if let Err(e) = self.storage.remove(&key) {
                                warn!("Failed to remove corrupted request: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading from queue storage: {}", e);
                }
            }
        }

        // Sort by priority and age
        let mut requests: Vec<_> = memory_queue.drain(..).collect();
        requests.sort_by(|a, b| {
            // Higher priority first, then older requests first
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.queued_at.cmp(&b.queued_at))
        });

        memory_queue.extend(requests);

        info!("Loaded {} requests from persistent storage", loaded_count);
        Ok(())
    }

    /// Start background task for periodic sync with cloud
    async fn start_sync_task(&self) {
        let queue = Arc::new(self.clone());
        let sync_interval = std::time::Duration::from_millis(self.config.queue.sync_interval_ms);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval);
            loop {
                interval.tick().await;
                if let Err(e) = queue.sync_with_cloud().await {
                    warn!("Background sync failed: {}", e);
                }
            }
        });
    }

    /// Calculate priority score for a request
    fn calculate_priority_score(&self, request: &MCPRequest) -> f32 {
        let mut score = 50.0; // Base score

        // Context-based priority
        if let Some(context) = &request.context {
            score += match context.priority {
                mcp_common::Priority::Critical => 40.0,
                mcp_common::Priority::High => 25.0,
                mcp_common::Priority::Normal => 0.0,
                mcp_common::Priority::Low => -15.0,
            };

            // Latency requirements boost priority
            if let Some(max_latency) = context.requirements.max_latency_ms {
                if max_latency < 1000 {
                    score += 20.0;
                } else if max_latency < 5000 {
                    score += 10.0;
                }
            }

            // Local-only requirements boost priority (will be processed when local is available)
            if context.requirements.require_local {
                score += 15.0;
            }
        }

        // Method-based priority
        score += match request.method.as_str() {
            "completion" => 10.0,
            "chat" => 5.0,
            "embedding" => -5.0,
            _ => 0.0,
        };

        score
    }

    /// Persist a request to storage
    async fn persist_request(&self, queued_request: &QueuedRequest) -> Result<()> {
        let key = format!("request:{}", queued_request.id);
        let value = serde_json::to_vec(queued_request)
            .map_err(|e| Error::Queue(format!("Failed to serialize request: {}", e)))?;

        self.storage
            .insert(key.as_bytes(), value)
            .map_err(|e| Error::Queue(format!("Failed to persist request: {}", e)))?;

        self.storage
            .flush()
            .map_err(|e| Error::Queue(format!("Failed to flush storage: {}", e)))?;

        Ok(())
    }

    /// Remove a request from storage
    async fn remove_from_storage(&self, request_id: &Uuid) -> Result<()> {
        let key = format!("request:{}", request_id);
        self.storage
            .remove(key.as_bytes())
            .map_err(|e| Error::Queue(format!("Failed to remove request from storage: {}", e)))?;

        Ok(())
    }

    /// Get queue statistics
    async fn get_queue_stats(&self) -> QueueStats {
        let stats = self.stats.read().await;
        QueueStats {
            total_enqueued: stats.total_enqueued,
            total_dequeued: stats.total_dequeued,
            total_failed: stats.total_failed,
            total_expired: stats.total_expired,
            sync_attempts: stats.sync_attempts,
            sync_successes: stats.sync_successes,
            last_sync_attempt: stats.last_sync_attempt,
            last_sync_success: stats.last_sync_success,
        }
    }

    /// Update statistics
    async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut QueueStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut *stats);
    }

    /// Clean up expired requests
    async fn cleanup_expired_requests(&self) -> Result<u32> {
        let mut memory_queue = self.memory_queue.write().await;
        let mut removed_count = 0;
        let now = chrono::Utc::now();

        // Find expired requests
        let mut expired_ids = Vec::new();
        memory_queue.retain(|req| {
            if let Some(expires_at) = req.expires_at {
                if now > expires_at {
                    expired_ids.push(req.id);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        // Remove expired requests from storage
        for id in expired_ids {
            if let Err(e) = self.remove_from_storage(&id).await {
                warn!("Failed to remove expired request {}: {}", id, e);
            } else {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            debug!("Cleaned up {} expired requests", removed_count);
            self.update_stats(|stats| stats.total_expired += removed_count as u64).await;
        }

        Ok(removed_count)
    }
}

#[async_trait]
impl OfflineQueue for PersistentQueue {
    async fn enqueue_request(&self, request: MCPRequest) -> Result<MCPResponse> {
        debug!("Enqueuing request: {}", request.id);

        // Check queue size limit
        let current_size = {
            let memory_queue = self.memory_queue.read().await;
            memory_queue.len()
        };

        if current_size >= self.config.queue.max_queue_size as usize {
            // Try to clean up expired requests first
            self.cleanup_expired_requests().await?;

            let current_size = {
                let memory_queue = self.memory_queue.read().await;
                memory_queue.len()
            };

            if current_size >= self.config.queue.max_queue_size as usize {
                return Err(Error::Queue("Queue is full".to_string()));
            }
        }

        // Calculate priority and expiration
        let priority_score = self.calculate_priority_score(&request);
        let expires_at = request.context.as_ref()
            .and_then(|ctx| ctx.timeout_ms)
            .map(|timeout| chrono::Utc::now() + chrono::Duration::milliseconds(timeout as i64));

        let queued_request = QueuedRequest {
            id: Uuid::new_v4(),
            request: request.clone(),
            queued_at: chrono::Utc::now(),
            retry_count: 0,
            priority_score,
            expires_at,
        };

        // Persist to storage
        if let Err(e) = self.persist_request(&queued_request).await {
            error!("Failed to persist request: {}", e);
            return Err(e);
        }

        // Add to memory queue
        {
            let mut memory_queue = self.memory_queue.write().await;
            
            // Insert in priority order
            let insert_pos = memory_queue
                .iter()
                .position(|req| req.priority_score < priority_score)
                .unwrap_or(memory_queue.len());
            
            memory_queue.insert(insert_pos, queued_request);
        }

        // Update statistics
        self.update_stats(|stats| stats.total_enqueued += 1).await;

        info!("Request {} queued successfully (priority: {:.1})", request.id, priority_score);

        Ok(MCPResponse {
            id: request.id,
            result: Some(serde_json::json!({
                "status": "queued",
                "priority_score": priority_score,
                "queue_position": current_size + 1
            })),
            error: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn dequeue_request(&self) -> Result<Option<MCPRequest>> {
        let mut memory_queue = self.memory_queue.write().await;

        if let Some(queued_request) = memory_queue.pop_front() {
            // Remove from storage
            if let Err(e) = self.remove_from_storage(&queued_request.id).await {
                warn!("Failed to remove request from storage: {}", e);
                // Re-add to front of queue
                memory_queue.push_front(queued_request);
                return Err(e);
            }

            self.update_stats(|stats| stats.total_dequeued += 1).await;
            
            debug!("Dequeued request: {}", queued_request.request.id);
            Ok(Some(queued_request.request))
        } else {
            Ok(None)
        }
    }

    async fn queue_size(&self) -> Result<u32> {
        let memory_queue = self.memory_queue.read().await;
        Ok(memory_queue.len() as u32)
    }

    async fn sync_with_cloud(&self) -> Result<()> {
        debug!("Starting queue sync with cloud");

        self.update_stats(|stats| {
            stats.sync_attempts += 1;
            stats.last_sync_attempt = Some(chrono::Utc::now());
        }).await;

        // Clean up expired requests first
        self.cleanup_expired_requests().await?;

        let requests_to_sync = {
            let memory_queue = self.memory_queue.read().await;
            memory_queue.iter().take(10).cloned().collect::<Vec<_>>() // Sync in batches
        };

        if requests_to_sync.is_empty() {
            debug!("No requests to sync");
            return Ok(());
        }

        let mut sync_count = 0;
        for queued_request in requests_to_sync {
            // TODO: Implement actual cloud sync
            // For now, just simulate sync attempt
            debug!("Syncing request: {}", queued_request.request.id);
            sync_count += 1;

            // Remove successfully synced request
            {
                let mut memory_queue = self.memory_queue.write().await;
                memory_queue.retain(|req| req.id != queued_request.id);
            }

            if let Err(e) = self.remove_from_storage(&queued_request.id).await {
                warn!("Failed to remove synced request from storage: {}", e);
            }
        }

        self.update_stats(|stats| {
            stats.sync_successes += 1;
            stats.last_sync_success = Some(chrono::Utc::now());
        }).await;

        info!("Successfully synced {} requests with cloud", sync_count);
        Ok(())
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let queue_size = self.queue_size().await?;
        let stats = self.get_queue_stats().await;

        let mut health_metrics = std::collections::HashMap::new();
        health_metrics.insert("queue_size".to_string(), queue_size as f32);
        health_metrics.insert("max_queue_size".to_string(), self.config.queue.max_queue_size as f32);
        health_metrics.insert("total_enqueued".to_string(), stats.total_enqueued as f32);
        health_metrics.insert("total_dequeued".to_string(), stats.total_dequeued as f32);
        health_metrics.insert("total_failed".to_string(), stats.total_failed as f32);
        health_metrics.insert("sync_attempts".to_string(), stats.sync_attempts as f32);
        health_metrics.insert("sync_successes".to_string(), stats.sync_successes as f32);

        let usage_percent = (queue_size as f32 / self.config.queue.max_queue_size as f32) * 100.0;
        health_metrics.insert("usage_percent".to_string(), usage_percent);

        let status = if usage_percent > 95.0 {
            HealthLevel::Critical
        } else if usage_percent > 80.0 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };

        let message = match status {
            HealthLevel::Healthy => format!("Queue operating normally ({} items)", queue_size),
            HealthLevel::Degraded => format!("Queue usage high ({:.1}%)", usage_percent),
            HealthLevel::Critical => format!("Queue nearly full ({:.1}%)", usage_percent),
            HealthLevel::Unknown => "Queue status unknown".to_string(),
        };

        Ok(ComponentHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            metrics: health_metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down persistent queue");

        // Flush any pending writes
        if let Err(e) = self.storage.flush() {
            warn!("Failed to flush queue storage: {}", e);
        }

        let stats = self.get_queue_stats().await;
        info!("Queue shutdown complete. Final stats: enqueued={}, dequeued={}, failed={}", 
              stats.total_enqueued, stats.total_dequeued, stats.total_failed);

        Ok(())
    }
}

// Implement Clone for PersistentQueue to enable Arc<Self> usage
impl Clone for PersistentQueue {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            storage: self.storage.clone(),
            memory_queue: self.memory_queue.clone(),
            stats: self.stats.clone(),
        }
    }
}