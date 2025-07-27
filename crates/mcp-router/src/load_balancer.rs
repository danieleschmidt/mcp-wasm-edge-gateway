//! Load balancing utilities for distributing requests

use mcp_common::config::{CloudEndpoint, LoadBalancingAlgorithm};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Load balancer for distributing requests across multiple endpoints
pub struct LoadBalancer {
    algorithm: LoadBalancingAlgorithm,
    endpoints: Vec<CloudEndpoint>,
    round_robin_counter: AtomicUsize,
    endpoint_health: RwLock<HashMap<String, EndpointHealth>>,
}

#[derive(Debug, Clone)]
struct EndpointHealth {
    is_healthy: bool,
    failure_count: u32,
    last_check: chrono::DateTime<chrono::Utc>,
    avg_response_time_ms: f32,
    active_connections: u32,
}

impl LoadBalancer {
    pub fn new(algorithm: LoadBalancingAlgorithm, endpoints: Vec<CloudEndpoint>) -> Self {
        let endpoint_health = endpoints
            .iter()
            .map(|e| {
                (
                    e.url.clone(),
                    EndpointHealth {
                        is_healthy: true,
                        failure_count: 0,
                        last_check: chrono::Utc::now(),
                        avg_response_time_ms: 1000.0,
                        active_connections: 0,
                    },
                )
            })
            .collect();

        Self {
            algorithm,
            endpoints,
            round_robin_counter: AtomicUsize::new(0),
            endpoint_health: RwLock::new(endpoint_health),
        }
    }

    /// Select the best endpoint for a request
    pub async fn select_endpoint(&self) -> Option<&CloudEndpoint> {
        if self.endpoints.is_empty() {
            return None;
        }

        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let index =
                    self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % self.endpoints.len();
                Some(&self.endpoints[index])
            },

            LoadBalancingAlgorithm::LeastConnections => {
                let health = self.endpoint_health.read().await;
                let mut best_endpoint = &self.endpoints[0];
                let mut min_connections = u32::MAX;

                for endpoint in &self.endpoints {
                    if let Some(health_info) = health.get(&endpoint.url) {
                        if health_info.is_healthy
                            && health_info.active_connections < min_connections
                        {
                            min_connections = health_info.active_connections;
                            best_endpoint = endpoint;
                        }
                    }
                }

                Some(best_endpoint)
            },

            LoadBalancingAlgorithm::WeightedRoundRobin => {
                // For simplicity, treat all endpoints equally for now
                // In a real implementation, you'd use endpoint weights
                let index =
                    self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % self.endpoints.len();
                Some(&self.endpoints[index])
            },

            LoadBalancingAlgorithm::HealthBased => {
                let health = self.endpoint_health.read().await;
                let mut best_endpoint = None;
                let mut best_score = f32::MIN;

                for endpoint in &self.endpoints {
                    if let Some(health_info) = health.get(&endpoint.url) {
                        if health_info.is_healthy {
                            // Calculate health score (lower response time and connections = better)
                            let score = 1000.0 / (health_info.avg_response_time_ms + 1.0)
                                - (health_info.active_connections as f32 * 10.0);

                            if score > best_score {
                                best_score = score;
                                best_endpoint = Some(endpoint);
                            }
                        }
                    }
                }

                best_endpoint.or_else(|| self.endpoints.first())
            },
        }
    }

    /// Update endpoint health information
    pub async fn update_endpoint_health(&self, url: &str, is_healthy: bool, response_time_ms: f32) {
        let mut health = self.endpoint_health.write().await;

        if let Some(endpoint_health) = health.get_mut(url) {
            endpoint_health.is_healthy = is_healthy;
            endpoint_health.last_check = chrono::Utc::now();

            if is_healthy {
                endpoint_health.failure_count = 0;
                // Update average response time with exponential moving average
                endpoint_health.avg_response_time_ms =
                    endpoint_health.avg_response_time_ms * 0.7 + response_time_ms * 0.3;
            } else {
                endpoint_health.failure_count += 1;
            }
        }
    }

    /// Mark the start of a connection to an endpoint
    pub async fn connection_started(&self, url: &str) {
        let mut health = self.endpoint_health.write().await;
        if let Some(endpoint_health) = health.get_mut(url) {
            endpoint_health.active_connections += 1;
        }
    }

    /// Mark the end of a connection to an endpoint
    pub async fn connection_ended(&self, url: &str) {
        let mut health = self.endpoint_health.write().await;
        if let Some(endpoint_health) = health.get_mut(url) {
            endpoint_health.active_connections =
                endpoint_health.active_connections.saturating_sub(1);
        }
    }

    /// Get health status of all endpoints
    pub async fn get_endpoint_health(&self) -> HashMap<String, EndpointHealth> {
        self.endpoint_health.read().await.clone()
    }

    /// Get healthy endpoints
    pub async fn get_healthy_endpoints(&self) -> Vec<&CloudEndpoint> {
        let health = self.endpoint_health.read().await;
        self.endpoints
            .iter()
            .filter(|endpoint| {
                health
                    .get(&endpoint.url)
                    .map(|h| h.is_healthy)
                    .unwrap_or(false)
            })
            .collect()
    }
}
