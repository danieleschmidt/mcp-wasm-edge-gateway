//! Persistent queue implementation

use crate::OfflineQueue;
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, MCPRequest, MCPResponse, Result, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error};

/// Queue entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueEntry {
    id: String,
    request: MCPRequest,
    priority: u8,
    retry_count: u32,
    max_retries: u32,
    created_at: chrono::DateTime<chrono::Utc>,
    last_retry: Option<chrono::DateTime<chrono::Utc>>,
    next_retry: Option<chrono::DateTime<chrono::Utc>>,
}

/// Persistent queue implementation using Sled embedded database
pub struct PersistentQueue {
    config: Arc<Config>,
    db: sled::Db,
    requests_tree: sled::Tree,
    priority_index: sled::Tree,
    next_sequence: AtomicU64,
    queue_stats: Arc<RwLock<QueueStats>>,
    sync_client: Arc<Mutex<Option<reqwest::Client>>>,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
struct QueueStats {
    total_enqueued: u64,
    total_dequeued: u64,
    total_synced: u64,
    failed_syncs: u64,
    current_size: u64,
    oldest_entry: Option<chrono::DateTime<chrono::Utc>>,
    newest_entry: Option<chrono::DateTime<chrono::Utc>>,
}

impl PersistentQueue {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        info!("Initializing persistent queue with storage path: {}", config.queue.storage_path.display());
        
        // Create storage directory if it doesn't exist
        let storage_path = config.queue.storage_path.clone();
        if let Some(parent) = storage_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| Error::Queue(format!("Failed to create queue storage directory: {}", e)))?;
        }
        
        // Open Sled database
        let db = sled::open(&storage_path)
            .map_err(|e| Error::Queue(format!("Failed to open queue database: {}", e)))?;
        
        // Create/open trees for different data types
        let requests_tree = db.open_tree("requests")
            .map_err(|e| Error::Queue(format!("Failed to open requests tree: {}", e)))?;
        let priority_index = db.open_tree("priority_index")
            .map_err(|e| Error::Queue(format!("Failed to open priority index: {}", e)))?;
        
        // Initialize sequence counter from existing data
        let mut max_sequence = 0u64;
        for entry in requests_tree.iter() {
            if let Ok((key, _)) = entry {
                if let Ok(key_str) = std::str::from_utf8(&key) {
                    if let Ok(seq) = key_str.parse::<u64>() {
                        max_sequence = max_sequence.max(seq);
                    }
                }
            }
        }
        
        // Calculate initial statistics
        let mut stats = QueueStats::default();
        stats.current_size = requests_tree.len() as u64;
        
        // Find oldest and newest entries
        if stats.current_size > 0 {
            if let Ok(Some((_, value))) = requests_tree.first() {
                if let Ok(entry) = bincode::deserialize::<QueueEntry>(&value) {
                    stats.oldest_entry = Some(entry.created_at);
                }
            }
            if let Ok(Some((_, value))) = requests_tree.last() {
                if let Ok(entry) = bincode::deserialize::<QueueEntry>(&value) {
                    stats.newest_entry = Some(entry.created_at);
                }
            }
        }
        
        // Initialize HTTP client for cloud sync
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Queue(format!("Failed to create HTTP client: {}", e)))?;
        
        info!("Persistent queue initialized with {} existing entries", stats.current_size);
        
        Ok(Self {
            config,
            db,
            requests_tree,
            priority_index,
            next_sequence: AtomicU64::new(max_sequence + 1),
            queue_stats: Arc::new(RwLock::new(stats)),
            sync_client: Arc::new(Mutex::new(Some(client))),
        })
    }
    
    /// Generate next sequence number for queue entries
    fn next_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Create queue entry key
    fn entry_key(&self, sequence: u64, priority: u8) -> String {
        // Format: {priority:03}{sequence:016} to ensure proper ordering
        format!("{:03}{:016}", 255 - priority, sequence)
    }
    
    /// Parse sequence from key
    fn parse_sequence_from_key(&self, key: &str) -> Option<u64> {
        if key.len() >= 19 {
            key[3..19].parse().ok()
        } else {
            None
        }
    }
    
    /// Calculate exponential backoff delay
    fn calculate_retry_delay(&self, retry_count: u32) -> chrono::Duration {
        let base_delay = 1000; // 1 second base
        let max_delay = 3600000; // 1 hour max
        let delay_ms = (base_delay * 2_u64.pow(retry_count)).min(max_delay as u64);
        chrono::Duration::milliseconds(delay_ms as i64)
    }
    
    /// Clean up old processed entries
    async fn cleanup_old_entries(&self) -> Result<u64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24); // Keep 24 hours of history
        let mut removed_count = 0u64;
        
        let mut keys_to_remove = Vec::new();
        
        for entry in self.requests_tree.iter() {
            if let Ok((key, value)) = entry {
                if let Ok(queue_entry) = bincode::deserialize::<QueueEntry>(&value) {
                    // Remove entries that are old and have been successfully processed
                    if queue_entry.created_at < cutoff {
                        keys_to_remove.push(key.to_vec());
                    }
                }
            }
        }
        
        for key in keys_to_remove {
            if let Ok(_) = self.requests_tree.remove(&key) {
                removed_count += 1;
            }
        }
        
        if removed_count > 0 {
            debug!("Cleaned up {} old queue entries", removed_count);
        }
        
        Ok(removed_count)
    }
    
    /// Sync a single entry to the cloud
    async fn sync_entry_to_cloud(&self, client: &reqwest::Client, entry: &QueueEntry) -> Result<bool> {
        let cloud_url = "https://api.example.com/v1/mcp/completions".to_string();
        
        // Prepare request payload
        let payload = serde_json::json!({
            "id": entry.request.id,
            "device_id": entry.request.device_id,
            "method": entry.request.method,
            "params": entry.request.params,
            "context": entry.request.context,
            "timestamp": entry.request.timestamp,
            "queued_at": entry.created_at,
            "retry_count": entry.retry_count
        });
        
        // Send request to cloud
        let response = client
            .post(&cloud_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "MCP-Edge-Gateway/1.0")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Queue(format!("Failed to send request to cloud: {}", e)))?;
        
        if response.status().is_success() {
            debug!("Successfully sent entry {} to cloud", entry.id);
            Ok(true)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!(
                "Cloud sync failed for entry {} with status {}: {}", 
                entry.id, status, error_text
            );
            Ok(false)
        }
    }
}

#[async_trait]
impl OfflineQueue for PersistentQueue {
    async fn enqueue_request(&self, request: MCPRequest) -> Result<MCPResponse> {
        debug!("Enqueueing request {} (method: {})", request.id, request.method);
        
        // Check queue size limits
        let current_size = self.requests_tree.len();
        if current_size >= self.config.queue.max_queue_size as usize {
            warn!("Queue at capacity ({} items), rejecting request", current_size);
            return Ok(MCPResponse {
                id: request.id,
                result: None,
                error: Some(mcp_common::MCPError {
                    code: -32000,
                    message: "Queue at capacity".to_string(),
                    data: Some(serde_json::json!({
                        "queue_size": current_size,
                        "max_size": self.config.queue.max_queue_size
                    })),
                }),
                timestamp: chrono::Utc::now(),
            });
        }
        
        // Determine priority based on method
        let priority = match request.method.as_str() {
            "completion" | "chat" => 1, // High priority
            "embedding" => 2,           // Medium priority  
            "summarization" => 3,       // Lower priority
            _ => 5,                      // Default priority
        };
        
        let sequence = self.next_sequence();
        let entry_id = format!("{}_{}", sequence, request.id);
        
        let queue_entry = QueueEntry {
            id: entry_id.clone(),
            request: request.clone(),
            priority,
            retry_count: 0,
            max_retries: 3,
            created_at: chrono::Utc::now(),
            last_retry: None,
            next_retry: None,
        };
        
        // Serialize and store the entry
        let serialized = bincode::serialize(&queue_entry)
            .map_err(|e| Error::Queue(format!("Failed to serialize queue entry: {}", e)))?;
        
        let key = self.entry_key(sequence, priority);
        
        self.requests_tree.insert(key.as_bytes(), serialized)
            .map_err(|e| Error::Queue(format!("Failed to store queue entry: {}", e)))?;
        
        // Update statistics
        {
            let mut stats = self.queue_stats.write().await;
            stats.total_enqueued += 1;
            stats.current_size += 1;
            stats.newest_entry = Some(queue_entry.created_at);
            if stats.oldest_entry.is_none() {
                stats.oldest_entry = Some(queue_entry.created_at);
            }
        }
        
        // Flush to disk
        self.db.flush_async().await
            .map_err(|e| Error::Queue(format!("Failed to flush queue to disk: {}", e)))?;
        
        info!("Request {} queued successfully (priority: {}, sequence: {})", request.id, priority, sequence);
        
        Ok(MCPResponse {
            id: request.id,
            result: Some(serde_json::json!({
                "status": "queued",
                "message": "Request queued for offline processing",
                "queue_id": entry_id,
                "priority": priority,
                "queue_position": current_size + 1
            })),
            error: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn dequeue_request(&self) -> Result<Option<MCPRequest>> {
        // Get the first (highest priority) entry that's ready for processing
        let mut entry_to_process = None;
        let mut key_to_remove = None;
        
        let now = chrono::Utc::now();
        
        // Iterate through entries in priority order
        for result in self.requests_tree.iter() {
            if let Ok((key, value)) = result {
                if let Ok(mut queue_entry) = bincode::deserialize::<QueueEntry>(&value) {
                    // Check if entry is ready for processing (not waiting for retry)
                    let ready_for_processing = queue_entry.next_retry.is_none() 
                        || queue_entry.next_retry.unwrap() <= now;
                    
                    if ready_for_processing {
                        // Check if we should still retry this entry
                        if queue_entry.retry_count < queue_entry.max_retries {
                            debug!(
                                "Dequeuing request {} (attempt {}/{}, priority: {})", 
                                queue_entry.request.id,
                                queue_entry.retry_count + 1,
                                queue_entry.max_retries,
                                queue_entry.priority
                            );
                            
                            // Update retry information
                            queue_entry.retry_count += 1;
                            queue_entry.last_retry = Some(now);
                            
                            if queue_entry.retry_count < queue_entry.max_retries {
                                // Schedule next retry
                                let delay = self.calculate_retry_delay(queue_entry.retry_count);
                                queue_entry.next_retry = Some(now + delay);
                                
                                // Update the entry in the database with new retry info
                                let updated_serialized = bincode::serialize(&queue_entry)
                                    .map_err(|e| Error::Queue(format!("Failed to serialize updated entry: {}", e)))?;
                                
                                self.requests_tree.insert(&key, updated_serialized)
                                    .map_err(|e| Error::Queue(format!("Failed to update queue entry: {}", e)))?;
                            } else {
                                // Max retries reached, remove from queue
                                warn!("Request {} exceeded max retries, removing from queue", queue_entry.request.id);
                                key_to_remove = Some(key.to_vec());
                            }
                            
                            entry_to_process = Some(queue_entry.request);
                            break;
                        } else {
                            // Entry has exceeded max retries, remove it
                            warn!("Removing expired entry {} from queue", queue_entry.request.id);
                            key_to_remove = Some(key.to_vec());
                            break;
                        }
                    }
                }
            }
        }
        
        // Remove expired entry if needed
        if let Some(key) = key_to_remove {
            self.requests_tree.remove(&key)
                .map_err(|e| Error::Queue(format!("Failed to remove expired entry: {}", e)))?;
            
            // Update statistics
            {
                let mut stats = self.queue_stats.write().await;
                stats.current_size = stats.current_size.saturating_sub(1);
            }
        }
        
        // Update statistics for dequeue
        if entry_to_process.is_some() {
            let mut stats = self.queue_stats.write().await;
            stats.total_dequeued += 1;
        }
        
        Ok(entry_to_process)
    }

    async fn queue_size(&self) -> Result<u32> {
        Ok(self.requests_tree.len() as u32)
    }

    async fn sync_with_cloud(&self) -> Result<()> {
        let cloud_endpoint = "https://api.example.com".to_string();
        if cloud_endpoint.is_empty() {
            debug!("No cloud endpoint configured, skipping sync");
            return Ok(());
        }
        
        let sync_start = std::time::Instant::now();
        let mut synced_count = 0u64;
        let mut failed_count = 0u64;
        
        info!("Starting sync with cloud endpoint: {}", cloud_endpoint);
        
        // Get HTTP client
        let client = {
            let client_guard = self.sync_client.lock().await;
            client_guard.as_ref().cloned()
        };
        
        let client = match client {
            Some(c) => c,
            None => {
                error!("HTTP client not available for sync");
                return Err(Error::Queue("HTTP client not available".to_string()));
            }
        };
        
        // Process entries ready for sync (completed or failed locally)
        let mut keys_to_remove = Vec::new();
        
        for result in self.requests_tree.iter() {
            if let Ok((key, value)) = result {
                if let Ok(queue_entry) = bincode::deserialize::<QueueEntry>(&value) {
                    // Try to sync this entry
                    match self.sync_entry_to_cloud(&client, &queue_entry).await {
                        Ok(success) => {
                            if success {
                                synced_count += 1;
                                keys_to_remove.push(key.to_vec());
                                debug!("Successfully synced entry {}", queue_entry.id);
                            }
                        },
                        Err(e) => {
                            failed_count += 1;
                            warn!("Failed to sync entry {}: {}", queue_entry.id, e);
                        }
                    }
                }
            }
        }
        
        // Remove successfully synced entries
        for key in keys_to_remove {
            if let Err(e) = self.requests_tree.remove(&key) {
                warn!("Failed to remove synced entry: {}", e);
            }
        }
        
        // Update statistics
        {
            let mut stats = self.queue_stats.write().await;
            stats.total_synced += synced_count;
            stats.failed_syncs += failed_count;
            stats.current_size = self.requests_tree.len() as u64;
        }
        
        // Flush database
        self.db.flush_async().await
            .map_err(|e| Error::Queue(format!("Failed to flush after sync: {}", e)))?;
        
        let sync_duration = sync_start.elapsed();
        info!(
            "Sync completed: {} entries synced, {} failed in {:?}", 
            synced_count, failed_count, sync_duration
        );
        
        // Run cleanup
        self.cleanup_old_entries().await?;
        
        Ok(())
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let stats = self.queue_stats.read().await;
        let current_size = self.requests_tree.len() as u32;
        
        let mut metrics = HashMap::new();
        metrics.insert("queue_size".to_string(), current_size as f32);
        metrics.insert("total_enqueued".to_string(), stats.total_enqueued as f32);
        metrics.insert("total_dequeued".to_string(), stats.total_dequeued as f32);
        metrics.insert("total_synced".to_string(), stats.total_synced as f32);
        metrics.insert("failed_syncs".to_string(), stats.failed_syncs as f32);
        
        // Calculate queue age metrics
        if let Some(oldest) = stats.oldest_entry {
            let age_minutes = (chrono::Utc::now() - oldest).num_minutes();
            metrics.insert("oldest_entry_age_minutes".to_string(), age_minutes as f32);
        }
        
        // Database size metrics
        if let Ok(db_size) = self.db.size_on_disk() {
            metrics.insert("db_size_mb".to_string(), (db_size / 1024 / 1024) as f32);
        }
        
        // Determine health status
        let queue_capacity_percent = (current_size as f32 / self.config.queue.max_queue_size as f32) * 100.0;
        metrics.insert("queue_capacity_percent".to_string(), queue_capacity_percent);
        
        let status = if queue_capacity_percent > 95.0 {
            HealthLevel::Critical
        } else if queue_capacity_percent > 80.0 || stats.failed_syncs > 100 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        let message = match status {
            HealthLevel::Healthy => format!("Queue healthy with {} items ({:.1}% capacity)", current_size, queue_capacity_percent),
            HealthLevel::Warning => format!("Queue at {:.1}% capacity with {} items", queue_capacity_percent, current_size),
            HealthLevel::Critical => format!("Queue near capacity: {} items ({:.1}%)", current_size, queue_capacity_percent),
            HealthLevel::Unknown => format!("Queue status unknown: {} items", current_size),
        };
        
        Ok(ComponentHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down persistent queue...");
        
        // Final flush to ensure all data is persisted
        self.db.flush_async().await
            .map_err(|e| Error::Queue(format!("Failed to flush database on shutdown: {}", e)))?;
        
        // Clear the HTTP client
        {
            let mut client_guard = self.sync_client.lock().await;
            *client_guard = None;
        }
        
        let final_size = self.requests_tree.len();
        info!("Persistent queue shutdown complete. Final queue size: {} items", final_size);
        
        Ok(())
    }
}