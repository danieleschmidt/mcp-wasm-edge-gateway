//! Performance optimization and scaling features
//!
//! This module provides advanced performance optimization capabilities including:
//! - Connection pooling and resource management
//! - Intelligent caching with TTL and eviction policies
//! - Concurrent request processing with load balancing
//! - Auto-scaling triggers and metrics-based optimization
//! - Memory and CPU optimization strategies

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{interval, timeout};
use tracing::{debug, info, warn, error};
use serde::{Deserialize, Serialize};

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum cache size in MB
    pub max_cache_size_mb: usize,
    /// Auto-scaling thresholds
    pub auto_scaling: AutoScalingConfig,
    /// Performance monitoring interval
    pub monitoring_interval_seconds: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 1000,
            connection_pool_size: 100,
            cache_ttl_seconds: 300, // 5 minutes
            max_cache_size_mb: 256,
            auto_scaling: AutoScalingConfig::default(),
            monitoring_interval_seconds: 30,
        }
    }
}

/// Auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    /// CPU threshold to trigger scaling up (0.0-1.0)
    pub cpu_scale_up_threshold: f32,
    /// CPU threshold to trigger scaling down (0.0-1.0)
    pub cpu_scale_down_threshold: f32,
    /// Memory threshold to trigger scaling up (0.0-1.0)
    pub memory_scale_up_threshold: f32,
    /// Request rate threshold (requests per second)
    pub request_rate_threshold: f32,
    /// Minimum instances to maintain
    pub min_instances: u32,
    /// Maximum instances allowed
    pub max_instances: u32,
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        Self {
            cpu_scale_up_threshold: 0.75,
            cpu_scale_down_threshold: 0.3,
            memory_scale_up_threshold: 0.8,
            request_rate_threshold: 100.0,
            min_instances: 1,
            max_instances: 10,
        }
    }
}

/// High-performance cache with TTL and intelligent eviction
#[derive(Clone)]
pub struct PerformanceCache<K, V> {
    data: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    ttl: Duration,
    max_size: usize,
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

impl<K, V> PerformanceCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(ttl: Duration, max_size: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_size,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut data = self.data.write().await;
        
        if let Some(entry) = data.get_mut(key) {
            // Check if entry is still valid
            if entry.created_at.elapsed() < self.ttl {
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
                *self.hits.write().await += 1;
                return Some(entry.value.clone());
            } else {
                // Entry expired, remove it
                data.remove(key);
            }
        }
        
        *self.misses.write().await += 1;
        None
    }

    /// Put value in cache
    pub async fn put(&self, key: K, value: V) {
        let mut data = self.data.write().await;
        
        // Check if we need to evict entries
        if data.len() >= self.max_size {
            self.evict_least_recently_used(&mut data).await;
        }
        
        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
        };
        
        data.insert(key, entry);
    }

    /// Evict least recently used entries
    async fn evict_least_recently_used(&self, data: &mut HashMap<K, CacheEntry<V>>) {
        if data.is_empty() {
            return;
        }

        // Find the entry with the oldest last_accessed time
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();

        for (key, entry) in data.iter() {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            data.remove(&key);
            debug!("Evicted cache entry due to size limit");
        }
    }

    /// Clear expired entries
    pub async fn clear_expired(&self) {
        let mut data = self.data.write().await;
        let now = Instant::now();
        
        data.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;
        let total = hits + misses;
        let hit_rate = if total > 0 { hits as f32 / total as f32 } else { 0.0 };
        
        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.data.read().await.len(),
            max_size: self.max_size,
        }
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f32,
    pub size: usize,
    pub max_size: usize,
}

/// Connection pool for resource management
pub struct ConnectionPool<T> {
    pool: Arc<RwLock<Vec<T>>>,
    semaphore: Arc<Semaphore>,
    max_size: usize,
    active_connections: Arc<RwLock<usize>>,
}

impl<T> ConnectionPool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(RwLock::new(Vec::with_capacity(max_size))),
            semaphore: Arc::new(Semaphore::new(max_size)),
            max_size,
            active_connections: Arc::new(RwLock::new(0)),
        }
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection<T>, PoolError> 
    where 
        T: Send + Sync + 'static,
    {
        // Wait for available slot
        let permit = self.semaphore.acquire().await
            .map_err(|_| PoolError::AcquisitionFailed)?;

        let mut pool = self.pool.write().await;
        
        if let Some(connection) = pool.pop() {
            *self.active_connections.write().await += 1;
            Ok(PooledConnection {
                connection: Some(connection),
                pool: self.pool.clone(),
                active_connections: self.active_connections.clone(),
                _permit: (),
            })
        } else {
            Err(PoolError::NoConnectionsAvailable)
        }
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, connection: T) -> Result<(), PoolError> {
        let mut pool = self.pool.write().await;
        
        if pool.len() < self.max_size {
            pool.push(connection);
            Ok(())
        } else {
            Err(PoolError::PoolFull)
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let pool_size = self.pool.read().await.len();
        let active = *self.active_connections.read().await;
        
        PoolStats {
            total_connections: pool_size + active,
            available_connections: pool_size,
            active_connections: active,
            max_size: self.max_size,
        }
    }
}

/// Pooled connection wrapper
pub struct PooledConnection<T: Send + Sync + 'static> {
    connection: Option<T>,
    pool: Arc<RwLock<Vec<T>>>,
    active_connections: Arc<RwLock<usize>>,
    _permit: (),
}

impl<T: Send + Sync + 'static> PooledConnection<T> {
    /// Get reference to the underlying connection
    pub fn as_ref(&self) -> Option<&T> {
        self.connection.as_ref()
    }

    /// Get mutable reference to the underlying connection
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.connection.as_mut()
    }
}

impl<T: Send + Sync + 'static> Drop for PooledConnection<T> {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            // Return connection to pool in background
            let pool = self.pool.clone();
            let active_connections = self.active_connections.clone();
            
            tokio::spawn(async move {
                pool.write().await.push(connection);
                *active_connections.write().await -= 1;
            });
        }
    }
}

/// Connection pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Failed to acquire connection")]
    AcquisitionFailed,
    #[error("No connections available")]
    NoConnectionsAvailable,
    #[error("Pool is at maximum capacity")]
    PoolFull,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize)]
pub struct PoolStats {
    pub total_connections: usize,
    pub available_connections: usize,
    pub active_connections: usize,
    pub max_size: usize,
}

/// Performance monitoring and optimization manager
pub struct PerformanceManager {
    config: PerformanceConfig,
    request_cache: PerformanceCache<String, serde_json::Value>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

impl PerformanceManager {
    pub fn new(config: PerformanceConfig) -> Self {
        let request_cache = PerformanceCache::new(
            Duration::from_secs(config.cache_ttl_seconds),
            config.max_cache_size_mb * 1024, // Convert to entries estimate
        );

        Self {
            config,
            request_cache,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            monitoring_task: None,
        }
    }

    /// Start performance monitoring
    pub async fn start_monitoring(&mut self) {
        let metrics = self.metrics.clone();
        let request_cache = self.request_cache.clone();
        let interval_duration = Duration::from_secs(self.config.monitoring_interval_seconds);

        let handle = tokio::spawn(async move {
            let mut interval = interval(interval_duration);

            loop {
                interval.tick().await;

                // Update metrics
                let cache_stats = request_cache.stats().await;
                let mut metrics_guard = metrics.write().await;
                metrics_guard.cache_hit_rate = cache_stats.hit_rate;
                metrics_guard.cache_size = cache_stats.size;
                
                // Trigger cleanup
                request_cache.clear_expired().await;

                debug!("Performance monitoring cycle completed");
            }
        });

        self.monitoring_task = Some(handle);
        info!("Performance monitoring started");
    }

    /// Get cached response
    pub async fn get_cached_response(&self, key: &str) -> Option<serde_json::Value> {
        self.request_cache.get(&key.to_string()).await
    }

    /// Cache response
    pub async fn cache_response(&self, key: String, response: serde_json::Value) {
        self.request_cache.put(key, response).await;
    }

    /// Record request metrics
    pub async fn record_request(&self, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update average response time with exponential moving average
        let duration_ms = duration.as_millis() as f32;
        if metrics.avg_response_time_ms == 0.0 {
            metrics.avg_response_time_ms = duration_ms;
        } else {
            metrics.avg_response_time_ms = 0.9 * metrics.avg_response_time_ms + 0.1 * duration_ms;
        }

        // Track peak response time
        if duration_ms > metrics.peak_response_time_ms {
            metrics.peak_response_time_ms = duration_ms;
        }
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Check if auto-scaling is needed
    pub async fn should_scale_up(&self) -> bool {
        let metrics = self.metrics.read().await;
        
        // Check various thresholds
        metrics.avg_response_time_ms > 1000.0 || // Response time > 1s
        metrics.current_cpu_usage > self.config.auto_scaling.cpu_scale_up_threshold ||
        metrics.current_memory_usage > self.config.auto_scaling.memory_scale_up_threshold ||
        metrics.requests_per_second > self.config.auto_scaling.request_rate_threshold
    }

    /// Shutdown monitoring
    pub async fn shutdown(&mut self) {
        if let Some(handle) = self.monitoring_task.take() {
            handle.abort();
            info!("Performance monitoring stopped");
        }
    }
}

/// Performance metrics for monitoring and auto-scaling
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f32,
    pub peak_response_time_ms: f32,
    pub requests_per_second: f32,
    pub current_cpu_usage: f32,
    pub current_memory_usage: f32,
    pub cache_hit_rate: f32,
    pub cache_size: usize,
    pub active_connections: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            peak_response_time_ms: 0.0,
            requests_per_second: 0.0,
            current_cpu_usage: 0.0,
            current_memory_usage: 0.0,
            cache_hit_rate: 0.0,
            cache_size: 0,
            active_connections: 0,
        }
    }
}

/// Execute operation with timeout and retries for resilience
pub async fn execute_with_resilience<F, T, E>(
    operation: F,
    max_retries: u32,
    timeout_duration: Duration,
    backoff_multiplier: f32,
) -> Result<T, ResilienceError<E>>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Debug,
{
    let mut attempts = 0;
    let mut delay = Duration::from_millis(100);

    while attempts <= max_retries {
        match timeout(timeout_duration, operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                attempts += 1;
                if attempts > max_retries {
                    return Err(ResilienceError::MaxRetriesExceeded(format!("{:?}", e)));
                }
                
                warn!("Operation failed, retrying in {:?} (attempt {}/{})", delay, attempts, max_retries + 1);
                tokio::time::sleep(delay).await;
                delay = Duration::from_millis((delay.as_millis() as f32 * backoff_multiplier) as u64);
            }
            Err(_) => {
                return Err(ResilienceError::Timeout);
            }
        }
    }

    Err(ResilienceError::MaxRetriesExceeded("All retries exhausted".to_string()))
}

/// Resilience operation errors
#[derive(Debug, thiserror::Error)]
pub enum ResilienceError<E: std::fmt::Debug> {
    #[error("Operation timed out")]
    Timeout,
    #[error("Maximum retries exceeded: {0}")]
    MaxRetriesExceeded(String),
    #[error("Operation failed: {0:?}")]
    OperationFailed(E),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_performance_cache() {
        let cache = PerformanceCache::new(Duration::from_secs(1), 10);
        
        // Test basic put/get
        cache.put("key1".to_string(), "value1".to_string()).await;
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));
        
        // Test TTL expiration
        sleep(Duration::from_millis(1100)).await;
        let expired_value = cache.get(&"key1".to_string()).await;
        assert_eq!(expired_value, None);
        
        // Test stats
        let stats = cache.stats().await;
        assert!(stats.misses > 0);
    }

    #[tokio::test]
    async fn test_execute_with_resilience() {
        let mut attempt_count = 0;
        
        let operation = || {
            attempt_count += 1;
            Box::pin(async move {
                if attempt_count < 3 {
                    Err("simulated failure")
                } else {
                    Ok("success")
                }
            })
        };

        let result = execute_with_resilience(
            operation,
            3,
            Duration::from_secs(1),
            1.5,
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count, 3);
    }
}