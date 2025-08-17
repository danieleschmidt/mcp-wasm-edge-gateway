//! Advanced load balancing with predictive algorithms and dynamic adaptation

// Load balancer is defined in this module
use mcp_common::{Config, Result, Error, CircuitBreaker, CircuitBreakerConfig, retry_operation, RetryStrategy};
use mcp_common::config::{CloudEndpoint, LoadBalancingAlgorithm};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Advanced load balancer with predictive capabilities
pub struct AdvancedLoadBalancer {
    config: Arc<Config>,
    endpoints: Vec<CloudEndpoint>,
    endpoint_metrics: Arc<RwLock<HashMap<String, EndpointMetrics>>>,
    global_metrics: Arc<RwLock<GlobalMetrics>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    request_counter: AtomicU64,
    algorithm: LoadBalancingAlgorithm,
}

/// Detailed metrics for each endpoint
#[derive(Debug)]
struct EndpointMetrics {
    // Basic health metrics
    is_healthy: bool,
    last_health_check: Instant,
    
    // Performance metrics
    avg_response_time_ms: f32,
    p95_response_time_ms: f32,
    p99_response_time_ms: f32,
    response_times: Vec<u64>, // Rolling window of recent response times
    
    // Load metrics
    active_connections: AtomicU32,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    
    // Capacity estimation
    estimated_capacity: f32,
    current_utilization: f32,
    max_observed_concurrent: u32,
    
    // Predictive metrics
    load_trend: f32, // Increasing/decreasing load
    predicted_response_time: f32,
    quality_score: f32, // Overall endpoint quality (0.0 - 1.0)
    
    // Time-based patterns
    hourly_load_pattern: [f32; 24],
    weekend_adjustment: f32,
}

/// Global load balancing metrics
#[derive(Debug, Default)]
struct GlobalMetrics {
    total_requests: AtomicU64,
    total_response_time: AtomicU64,
    peak_concurrent_requests: AtomicU32,
    load_distribution: HashMap<String, f32>, // Percentage of load per endpoint
    optimal_distribution: HashMap<String, f32>, // Calculated optimal distribution
}

impl AdvancedLoadBalancer {
    /// Create a new advanced load balancer
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let endpoints = config.router.cloud_endpoints.clone();
        let algorithm = config.router.load_balancing.algorithm.clone();
        
        let mut endpoint_metrics = HashMap::new();
        let mut circuit_breakers = HashMap::new();
        
        for endpoint in &endpoints {
            // Initialize metrics for each endpoint
            endpoint_metrics.insert(
                endpoint.url.clone(),
                EndpointMetrics {
                    is_healthy: true,
                    last_health_check: Instant::now(),
                    avg_response_time_ms: 1000.0,
                    p95_response_time_ms: 2000.0,
                    p99_response_time_ms: 3000.0,
                    response_times: Vec::with_capacity(1000),
                    active_connections: AtomicU32::new(0),
                    total_requests: AtomicU64::new(0),
                    successful_requests: AtomicU64::new(0),
                    failed_requests: AtomicU64::new(0),
                    estimated_capacity: 100.0, // Start with default capacity
                    current_utilization: 0.0,
                    max_observed_concurrent: 0,
                    load_trend: 0.0,
                    predicted_response_time: 1000.0,
                    quality_score: 1.0,
                    hourly_load_pattern: [0.0; 24],
                    weekend_adjustment: 1.0,
                },
            );
            
            // Initialize circuit breaker for each endpoint
            let cb_config = CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 5,
                timeout: Duration::from_secs(30),
                window_size: 20,
                minimum_requests: 5,
            };
            
            circuit_breakers.insert(
                endpoint.url.clone(),
                CircuitBreaker::new(format!("endpoint-{}", endpoint.url), cb_config),
            );
        }

        Ok(Self {
            config,
            endpoints,
            endpoint_metrics: Arc::new(RwLock::new(endpoint_metrics)),
            global_metrics: Arc::new(RwLock::new(GlobalMetrics::default())),
            circuit_breakers: Arc::new(RwLock::new(circuit_breakers)),
            request_counter: AtomicU64::new(0),
            algorithm,
        })
    }

    /// Select the best endpoint using advanced algorithms
    pub async fn select_endpoint(&self) -> Result<&CloudEndpoint> {
        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => self.round_robin_selection().await,
            LoadBalancingAlgorithm::LeastConnections => self.least_connections_selection().await,
            LoadBalancingAlgorithm::WeightedRoundRobin => self.weighted_round_robin_selection().await,
            LoadBalancingAlgorithm::HealthBased => self.health_based_selection().await,
        }
    }

    /// Advanced round robin with health awareness
    async fn round_robin_selection(&self) -> Result<&CloudEndpoint> {
        let request_num = self.request_counter.fetch_add(1, Ordering::Relaxed);
        let metrics = self.endpoint_metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;

        // Filter healthy endpoints
        let mut healthy_endpoints = Vec::new();
        for (i, endpoint) in self.endpoints.iter().enumerate() {
            if let Some(endpoint_metrics) = metrics.get(&endpoint.url) {
                if endpoint_metrics.is_healthy {
                    if let Some(cb) = circuit_breakers.get(&endpoint.url) {
                        if cb.should_allow_call().await {
                            healthy_endpoints.push((i, endpoint));
                        }
                    }
                }
            }
        }

        if healthy_endpoints.is_empty() {
            return Err(Error::Routing("No healthy endpoints available".to_string()));
        }

        let selected_idx = (request_num as usize) % healthy_endpoints.len();
        Ok(healthy_endpoints[selected_idx].1)
    }

    /// Least connections with predictive capacity estimation
    async fn least_connections_selection(&self) -> Result<&CloudEndpoint> {
        let metrics = self.endpoint_metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;

        let mut best_endpoint = None;
        let mut best_score = f32::MAX;

        for endpoint in &self.endpoints {
            if let Some(endpoint_metrics) = metrics.get(&endpoint.url) {
                if !endpoint_metrics.is_healthy {
                    continue;
                }

                if let Some(cb) = circuit_breakers.get(&endpoint.url) {
                    if !cb.should_allow_call().await {
                        continue;
                    }
                }

                let active_connections = endpoint_metrics.active_connections.load(Ordering::Relaxed);
                let utilization = endpoint_metrics.current_utilization;
                let predicted_response_time = endpoint_metrics.predicted_response_time;

                // Calculate composite score (lower is better)
                let connection_score = active_connections as f32 / endpoint_metrics.estimated_capacity;
                let utilization_score = utilization;
                let latency_score = predicted_response_time / 1000.0; // Normalize to seconds
                let quality_penalty = 1.0 - endpoint_metrics.quality_score;

                let composite_score = connection_score * 0.4 
                    + utilization_score * 0.3 
                    + latency_score * 0.2 
                    + quality_penalty * 0.1;

                if composite_score < best_score {
                    best_score = composite_score;
                    best_endpoint = Some(endpoint);
                }
            }
        }

        best_endpoint.ok_or_else(|| Error::Routing("No suitable endpoints available".to_string()))
    }

    /// Weighted round robin based on dynamic performance metrics
    async fn weighted_round_robin_selection(&self) -> Result<&CloudEndpoint> {
        let metrics = self.endpoint_metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;

        // Calculate dynamic weights based on performance
        let mut weights = Vec::new();
        let mut total_weight = 0.0f32;

        for endpoint in &self.endpoints {
            if let Some(endpoint_metrics) = metrics.get(&endpoint.url) {
                if endpoint_metrics.is_healthy {
                    if let Some(cb) = circuit_breakers.get(&endpoint.url) {
                        if cb.should_allow_call().await {
                            // Calculate weight based on quality score and inverse of response time
                            let weight = endpoint_metrics.quality_score 
                                * (2000.0 / (endpoint_metrics.avg_response_time_ms + 100.0));
                            weights.push(weight);
                            total_weight += weight;
                        } else {
                            weights.push(0.0);
                        }
                    } else {
                        weights.push(0.0);
                    }
                } else {
                    weights.push(0.0);
                }
            } else {
                weights.push(0.0);
            }
        }

        if total_weight == 0.0 {
            return Err(Error::Routing("No healthy endpoints with positive weights".to_string()));
        }

        // Select endpoint based on weighted probability
        let request_num = self.request_counter.fetch_add(1, Ordering::Relaxed);
        let mut cumulative_weight = 0.0;
        let selection_point = (request_num as f32 % total_weight) / total_weight;

        for (i, &weight) in weights.iter().enumerate() {
            cumulative_weight += weight / total_weight;
            if selection_point <= cumulative_weight {
                return Ok(&self.endpoints[i]);
            }
        }

        // Fallback to first healthy endpoint
        for (i, &weight) in weights.iter().enumerate() {
            if weight > 0.0 {
                return Ok(&self.endpoints[i]);
            }
        }

        Err(Error::Routing("No healthy endpoints available".to_string()))
    }

    /// Health-based selection with predictive analytics
    async fn health_based_selection(&self) -> Result<&CloudEndpoint> {
        let metrics = self.endpoint_metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;

        let mut best_endpoint = None;
        let mut best_health_score = 0.0f32;

        for endpoint in &self.endpoints {
            if let Some(endpoint_metrics) = metrics.get(&endpoint.url) {
                if !endpoint_metrics.is_healthy {
                    continue;
                }

                if let Some(cb) = circuit_breakers.get(&endpoint.url) {
                    if !cb.should_allow_call().await {
                        continue;
                    }
                }

                // Calculate comprehensive health score
                let health_score = self.calculate_health_score(endpoint_metrics);

                if health_score > best_health_score {
                    best_health_score = health_score;
                    best_endpoint = Some(endpoint);
                }
            }
        }

        best_endpoint.ok_or_else(|| Error::Routing("No healthy endpoints available".to_string()))
    }

    /// Calculate comprehensive health score for an endpoint
    fn calculate_health_score(&self, metrics: &EndpointMetrics) -> f32 {
        let mut score = 0.0f32;

        // Base health (30% weight)
        if metrics.is_healthy {
            score += 0.3;
        }

        // Response time factor (25% weight)
        let response_time_score = (5000.0 - metrics.avg_response_time_ms.min(5000.0)) / 5000.0;
        score += response_time_score * 0.25;

        // Success rate factor (25% weight)
        let total_requests = metrics.total_requests.load(Ordering::Relaxed);
        if total_requests > 0 {
            let success_rate = metrics.successful_requests.load(Ordering::Relaxed) as f32 / total_requests as f32;
            score += success_rate * 0.25;
        } else {
            score += 0.25; // No data, assume good
        }

        // Utilization factor (10% weight) - prefer less utilized endpoints
        let utilization_score = 1.0 - metrics.current_utilization.min(1.0);
        score += utilization_score * 0.1;

        // Quality score factor (10% weight)
        score += metrics.quality_score * 0.1;

        score.min(1.0)
    }

    /// Record request completion for learning
    pub async fn record_request_completion(
        &self,
        endpoint_url: &str,
        response_time_ms: u64,
        success: bool,
    ) -> Result<()> {
        let mut metrics = self.endpoint_metrics.write().await;
        let circuit_breakers = self.circuit_breakers.read().await;

        if let Some(endpoint_metrics) = metrics.get_mut(endpoint_url) {
            // Update response time metrics
            endpoint_metrics.response_times.push(response_time_ms);
            if endpoint_metrics.response_times.len() > 1000 {
                endpoint_metrics.response_times.drain(0..100); // Keep recent 1000 samples
            }

            // Update average response time with exponential moving average
            let alpha = 0.1;
            endpoint_metrics.avg_response_time_ms = alpha * response_time_ms as f32 
                + (1.0 - alpha) * endpoint_metrics.avg_response_time_ms;

            // Update percentiles
            let mut sorted_times = endpoint_metrics.response_times.clone();
            sorted_times.sort_unstable();
            let len = sorted_times.len();
            if len > 0 {
                endpoint_metrics.p95_response_time_ms = sorted_times[(len * 95 / 100).min(len - 1)] as f32;
                endpoint_metrics.p99_response_time_ms = sorted_times[(len * 99 / 100).min(len - 1)] as f32;
            }

            // Update request counters
            endpoint_metrics.total_requests.fetch_add(1, Ordering::Relaxed);
            if success {
                endpoint_metrics.successful_requests.fetch_add(1, Ordering::Relaxed);
            } else {
                endpoint_metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
            }

            // Update quality score
            let total = endpoint_metrics.total_requests.load(Ordering::Relaxed) as f32;
            let successful = endpoint_metrics.successful_requests.load(Ordering::Relaxed) as f32;
            let success_rate = if total > 0.0 { successful / total } else { 1.0 };
            
            // Quality score combines success rate and response time
            let response_factor = (3000.0 - endpoint_metrics.avg_response_time_ms.min(3000.0)) / 3000.0;
            endpoint_metrics.quality_score = (success_rate * 0.7 + response_factor * 0.3).min(1.0);

            // Update current utilization
            let active = endpoint_metrics.active_connections.load(Ordering::Relaxed) as f32;
            endpoint_metrics.current_utilization = active / endpoint_metrics.estimated_capacity;

            // Record with circuit breaker
            if let Some(cb) = circuit_breakers.get(endpoint_url) {
                cb.record_call_result(success).await;
            }
        }

        // Update global metrics
        let mut global = self.global_metrics.write().await;
        global.total_requests.fetch_add(1, Ordering::Relaxed);
        global.total_response_time.fetch_add(response_time_ms, Ordering::Relaxed);

        Ok(())
    }

    /// Start request tracking
    pub async fn start_request(&self, endpoint_url: &str) -> Result<()> {
        let metrics = self.endpoint_metrics.read().await;
        if let Some(endpoint_metrics) = metrics.get(endpoint_url) {
            let active = endpoint_metrics.active_connections.fetch_add(1, Ordering::Relaxed);
            
            // Update max observed concurrent
            let mut metrics_write = self.endpoint_metrics.write().await;
            if let Some(metrics_mut) = metrics_write.get_mut(endpoint_url) {
                if active > metrics_mut.max_observed_concurrent {
                    metrics_mut.max_observed_concurrent = active;
                    // Update capacity estimate based on observed maximum
                    metrics_mut.estimated_capacity = (active as f32 * 1.2).max(metrics_mut.estimated_capacity);
                }
            }
        }
        Ok(())
    }

    /// End request tracking
    pub async fn end_request(&self, endpoint_url: &str) -> Result<()> {
        let metrics = self.endpoint_metrics.read().await;
        if let Some(endpoint_metrics) = metrics.get(endpoint_url) {
            endpoint_metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Perform health checks on all endpoints
    pub async fn health_check_all(&self) -> Result<()> {
        let endpoints = self.endpoints.clone();
        let mut tasks = Vec::new();

        for endpoint in endpoints {
            let metrics = self.endpoint_metrics.clone();
            let has_cb = self.circuit_breakers.read().await.contains_key(&endpoint.url);
            
            tasks.push(tokio::spawn(async move {
                let is_healthy = Self::check_endpoint_health(&endpoint).await;
                
                let mut metrics_write = metrics.write().await;
                if let Some(endpoint_metrics) = metrics_write.get_mut(&endpoint.url) {
                    endpoint_metrics.is_healthy = is_healthy;
                    endpoint_metrics.last_health_check = Instant::now();
                }

                // Update circuit breaker
                if let Some(cb) = cb {
                    if is_healthy {
                        // Reset circuit breaker if endpoint is healthy
                        cb.record_call_result(true).await;
                    }
                }

                (endpoint.url.clone(), is_healthy)
            }));
        }

        // Wait for all health checks to complete
        for task in tasks {
            match task.await {
                Ok((url, is_healthy)) => {
                    debug!("Health check for {}: {}", url, if is_healthy { "healthy" } else { "unhealthy" });
                }
                Err(e) => {
                    warn!("Health check task failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Check health of a single endpoint
    async fn check_endpoint_health(endpoint: &CloudEndpoint) -> bool {
        // Use retry logic for health checks
        let result = retry_operation(
            &format!("health_check_{}", endpoint.url),
            RetryStrategy::fixed_delay(Duration::from_millis(1000)),
            || {
                Box::pin(async {
                    // Simple TCP connection test or HTTP health endpoint
                    let client = reqwest::Client::new();
                    let health_url = format!("{}/health", endpoint.url);
                    
                    match tokio::time::timeout(
                        Duration::from_secs(5),
                        client.get(&health_url).send()
                    ).await {
                        Ok(Ok(response)) => {
                            if response.status().is_success() {
                                Ok(true)
                            } else {
                                Err(Error::Network(format!("Health check failed with status: {}", response.status())))
                            }
                        }
                        Ok(Err(e)) => Err(Error::Network(format!("Health check request failed: {}", e))),
                        Err(_) => Err(Error::Timeout("Health check timed out".to_string())),
                    }
                })
            }
        ).await;

        result.unwrap_or(false)
    }

    /// Get load balancer statistics
    pub async fn get_stats(&self) -> LoadBalancerStats {
        let metrics = self.endpoint_metrics.read().await;
        let global = self.global_metrics.read().await;

        let mut endpoint_stats = HashMap::new();
        for (url, endpoint_metrics) in metrics.iter() {
            let total = endpoint_metrics.total_requests.load(Ordering::Relaxed);
            let successful = endpoint_metrics.successful_requests.load(Ordering::Relaxed);
            let success_rate = if total > 0 { (successful as f32 / total as f32) * 100.0 } else { 100.0 };

            endpoint_stats.insert(url.clone(), EndpointStats {
                is_healthy: endpoint_metrics.is_healthy,
                avg_response_time_ms: endpoint_metrics.avg_response_time_ms,
                p95_response_time_ms: endpoint_metrics.p95_response_time_ms,
                active_connections: endpoint_metrics.active_connections.load(Ordering::Relaxed),
                total_requests: total,
                success_rate,
                quality_score: endpoint_metrics.quality_score,
                current_utilization: endpoint_metrics.current_utilization,
            });
        }

        let total_global = global.total_requests.load(Ordering::Relaxed);
        let avg_global_response_time = if total_global > 0 {
            global.total_response_time.load(Ordering::Relaxed) as f32 / total_global as f32
        } else {
            0.0
        };

        LoadBalancerStats {
            algorithm: format!("{:?}", self.algorithm),
            total_requests: total_global,
            avg_response_time_ms: avg_global_response_time,
            endpoint_stats,
        }
    }

    /// Optimize load distribution based on current metrics
    pub async fn optimize_distribution(&self) -> Result<()> {
        let mut global = self.global_metrics.write().await;
        let metrics = self.endpoint_metrics.read().await;

        // Calculate optimal distribution based on endpoint capacities and quality
        let mut total_capacity = 0.0f32;
        let mut endpoint_capacities = HashMap::new();

        for (url, endpoint_metrics) in metrics.iter() {
            if endpoint_metrics.is_healthy {
                let adjusted_capacity = endpoint_metrics.estimated_capacity * endpoint_metrics.quality_score;
                endpoint_capacities.insert(url.clone(), adjusted_capacity);
                total_capacity += adjusted_capacity;
            }
        }

        // Calculate optimal percentage for each endpoint
        global.optimal_distribution.clear();
        for (url, capacity) in endpoint_capacities {
            let percentage = if total_capacity > 0.0 {
                capacity / total_capacity
            } else {
                0.0
            };
            global.optimal_distribution.insert(url, percentage);
        }

        info!("Optimized load distribution: {:?}", global.optimal_distribution);
        Ok(())
    }
}

/// Statistics for the load balancer
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    pub algorithm: String,
    pub total_requests: u64,
    pub avg_response_time_ms: f32,
    pub endpoint_stats: HashMap<String, EndpointStats>,
}

/// Statistics for individual endpoints
#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub is_healthy: bool,
    pub avg_response_time_ms: f32,
    pub p95_response_time_ms: f32,
    pub active_connections: u32,
    pub total_requests: u64,
    pub success_rate: f32,
    pub quality_score: f32,
    pub current_utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_common::config::{CloudEndpoint, RouterConfig, LoadBalancingConfig};

    fn create_test_config() -> Arc<Config> {
        let mut config = Config::default();
        config.router = RouterConfig {
            cloud_endpoints: vec![
                CloudEndpoint {
                    url: "http://endpoint1.test".to_string(),
                    api_key: None,
                    timeout_ms: 5000,
                    max_retries: 3,
                },
                CloudEndpoint {
                    url: "http://endpoint2.test".to_string(),
                    api_key: None,
                    timeout_ms: 5000,
                    max_retries: 3,
                },
            ],
            load_balancing: LoadBalancingConfig {
                algorithm: LoadBalancingAlgorithm::LeastConnections,
                health_check_interval_seconds: 30,
            },
            local_processing_threshold: 0.8,
            cloud_fallback_enabled: true,
        };
        Arc::new(config)
    }

    #[tokio::test]
    async fn test_load_balancer_creation() {
        let config = create_test_config();
        let lb = AdvancedLoadBalancer::new(config).await.unwrap();
        
        assert_eq!(lb.endpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_endpoint_selection() {
        let config = create_test_config();
        let lb = AdvancedLoadBalancer::new(config).await.unwrap();
        
        // Since we don't have real endpoints, this will select based on initial metrics
        // In a real scenario, health checks would determine availability
        let _endpoint = lb.least_connections_selection().await;
        // Just verify it doesn't panic - can't test actual selection without real endpoints
    }

    #[tokio::test]
    async fn test_request_tracking() {
        let config = create_test_config();
        let lb = AdvancedLoadBalancer::new(config).await.unwrap();
        
        let endpoint_url = "http://endpoint1.test";
        
        lb.start_request(endpoint_url).await.unwrap();
        lb.record_request_completion(endpoint_url, 1500, true).await.unwrap();
        lb.end_request(endpoint_url).await.unwrap();
        
        let stats = lb.get_stats().await;
        assert_eq!(stats.total_requests, 1);
    }
}