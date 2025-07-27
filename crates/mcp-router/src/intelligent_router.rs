//! Intelligent router implementation

use crate::Router;
use async_trait::async_trait;
use mcp_common::config::RoutingStrategy;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, Error, MCPRequest, MCPResponse, Result, RoutingDecision};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Intelligent router that makes routing decisions based on multiple factors
pub struct IntelligentRouter {
    config: Arc<Config>,
    metrics_store: Arc<RwLock<MetricsStore>>,
    cloud_client: Option<crate::cloud_client::CloudClient>,
}

/// Store for routing metrics and decision history
#[derive(Debug, Default)]
struct MetricsStore {
    local_performance: HashMap<String, f32>, // model_id -> avg_latency_ms
    cloud_performance: HashMap<String, f32>, // endpoint -> avg_latency_ms
    resource_usage: ResourceUsage,
    request_history: Vec<RoutingDecisionRecord>,
}

#[derive(Debug, Default)]
struct ResourceUsage {
    cpu_usage_percent: f32,
    memory_usage_percent: f32,
    active_requests: u32,
}

#[derive(Debug)]
struct RoutingDecisionRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    request_complexity: f32,
    decision: RoutingDecision,
    actual_latency_ms: Option<u64>,
}

impl IntelligentRouter {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let cloud_client = if config.router.cloud_fallback_enabled {
            Some(crate::cloud_client::CloudClient::new(config.clone()).await?)
        } else {
            None
        };

        Ok(Self {
            config,
            metrics_store: Arc::new(RwLock::new(MetricsStore::default())),
            cloud_client,
        })
    }

    /// Analyze request complexity to help with routing decisions
    fn analyze_request_complexity(&self, request: &MCPRequest) -> f32 {
        let mut complexity_score = 0.0;

        // Base complexity on method
        complexity_score += match request.method.as_str() {
            "completion" => 0.8,
            "embedding" => 0.4,
            "chat" => 0.7,
            "summarization" => 0.6,
            _ => 0.5,
        };

        // Add complexity based on parameters
        if let Some(context) = &request.context {
            if let Some(max_latency) = context.requirements.max_latency_ms {
                if max_latency < 1000 {
                    complexity_score += 0.3; // High urgency adds complexity
                }
            }

            if context.requirements.require_local {
                complexity_score += 0.2; // Local requirement adds complexity
            }
        }

        // Analyze parameter complexity
        let param_count = request.params.len();
        complexity_score += (param_count as f32) * 0.05;

        // Check for large text inputs
        for value in request.params.values() {
            if let Some(text) = value.as_str() {
                if text.len() > 1000 {
                    complexity_score += 0.2;
                }
            }
        }

        complexity_score.min(1.0) // Cap at 1.0
    }

    /// Make routing decision based on current state and request
    async fn make_routing_decision(&self, request: &MCPRequest) -> Result<RoutingDecision> {
        let complexity = self.analyze_request_complexity(request);
        let metrics = self.metrics_store.read().await;

        debug!("Request complexity: {:.2}", complexity);

        // Check if local processing is required
        if let Some(context) = &request.context {
            if context.requirements.require_local {
                return Ok(RoutingDecision::Local {
                    model_id: "default".to_string(), // TODO: Select appropriate model
                    estimated_latency_ms: 500,
                });
            }
        }

        // Apply routing strategy
        match &self.config.router.strategy {
            RoutingStrategy::ComplexityBased => {
                if complexity > self.config.router.local_processing_threshold {
                    // High complexity - try cloud if available
                    if self.cloud_client.is_some() && !self.config.router.cloud_endpoints.is_empty()
                    {
                        Ok(RoutingDecision::Cloud {
                            endpoint: self.config.router.cloud_endpoints[0].url.clone(),
                            estimated_latency_ms: 2000,
                        })
                    } else {
                        Ok(RoutingDecision::Queue {
                            reason: "High complexity request queued for later processing"
                                .to_string(),
                            retry_after_ms: 5000,
                        })
                    }
                } else {
                    // Low complexity - process locally
                    Ok(RoutingDecision::Local {
                        model_id: "fast_model".to_string(),
                        estimated_latency_ms: 200,
                    })
                }
            },
            RoutingStrategy::ResourceAware => {
                // Consider current resource usage
                if metrics.resource_usage.cpu_usage_percent > 85.0
                    || metrics.resource_usage.memory_usage_percent > 90.0
                {
                    // High resource usage - offload to cloud or queue
                    if self.cloud_client.is_some() && !self.config.router.cloud_endpoints.is_empty()
                    {
                        Ok(RoutingDecision::Cloud {
                            endpoint: self.config.router.cloud_endpoints[0].url.clone(),
                            estimated_latency_ms: 2000,
                        })
                    } else {
                        Ok(RoutingDecision::Queue {
                            reason: "High resource usage - request queued".to_string(),
                            retry_after_ms: 10000,
                        })
                    }
                } else {
                    Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: 300,
                    })
                }
            },
            RoutingStrategy::PerformanceOptimized => {
                // Choose based on historical performance
                let local_avg_latency = metrics.local_performance.get("default").unwrap_or(&500.0);
                let cloud_avg_latency = if !self.config.router.cloud_endpoints.is_empty() {
                    metrics
                        .cloud_performance
                        .get(&self.config.router.cloud_endpoints[0].url)
                        .unwrap_or(&2000.0)
                } else {
                    &f32::MAX
                };

                if local_avg_latency < cloud_avg_latency {
                    Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: *local_avg_latency as u64,
                    })
                } else if self.cloud_client.is_some() {
                    Ok(RoutingDecision::Cloud {
                        endpoint: self.config.router.cloud_endpoints[0].url.clone(),
                        estimated_latency_ms: *cloud_avg_latency as u64,
                    })
                } else {
                    Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: *local_avg_latency as u64,
                    })
                }
            },
            RoutingStrategy::Hybrid {
                weights,
            } => {
                // Combined scoring approach
                let mut local_score = 0.0;
                let mut cloud_score = 0.0;

                // Complexity factor
                local_score += (1.0 - complexity) * weights.complexity;
                cloud_score += complexity * weights.complexity;

                // Resource factor
                let resource_pressure = (metrics.resource_usage.cpu_usage_percent
                    + metrics.resource_usage.memory_usage_percent)
                    / 200.0;
                local_score += (1.0 - resource_pressure) * weights.resource_usage;
                cloud_score += resource_pressure * weights.resource_usage;

                // Performance factor
                let local_perf =
                    1.0 / (metrics.local_performance.get("default").unwrap_or(&500.0) / 1000.0);
                let cloud_perf = if !self.config.router.cloud_endpoints.is_empty() {
                    1.0 / (metrics
                        .cloud_performance
                        .get(&self.config.router.cloud_endpoints[0].url)
                        .unwrap_or(&2000.0)
                        / 1000.0)
                } else {
                    0.0
                };

                local_score += local_perf * weights.historical_performance;
                cloud_score += cloud_perf * weights.historical_performance;

                debug!(
                    "Routing scores - Local: {:.2}, Cloud: {:.2}",
                    local_score, cloud_score
                );

                if local_score >= cloud_score {
                    Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: 400,
                    })
                } else if self.cloud_client.is_some()
                    && !self.config.router.cloud_endpoints.is_empty()
                {
                    Ok(RoutingDecision::Cloud {
                        endpoint: self.config.router.cloud_endpoints[0].url.clone(),
                        estimated_latency_ms: 1500,
                    })
                } else {
                    Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: 400,
                    })
                }
            },
        }
    }
}

#[async_trait]
impl Router for IntelligentRouter {
    async fn route(&self, request: &MCPRequest) -> Result<RoutingDecision> {
        debug!("Routing request {}", request.id);

        let decision = self.make_routing_decision(request).await?;

        // Record the decision for learning
        {
            let mut metrics = self.metrics_store.write().await;
            metrics.request_history.push(RoutingDecisionRecord {
                timestamp: chrono::Utc::now(),
                request_complexity: self.analyze_request_complexity(request),
                decision: decision.clone(),
                actual_latency_ms: None, // Will be updated later
            });

            // Keep only recent history (last 1000 decisions)
            if metrics.request_history.len() > 1000 {
                metrics.request_history.drain(0..100);
            }
        }

        info!("Routing decision: {:?}", decision);
        Ok(decision)
    }

    async fn forward_to_cloud(&self, request: &MCPRequest, endpoint: &str) -> Result<MCPResponse> {
        if let Some(client) = &self.cloud_client {
            client.forward_request(request, endpoint).await
        } else {
            Err(Error::Routing("Cloud client not available".to_string()))
        }
    }

    async fn update_metrics(&self, metrics: &mcp_common::PerformanceMetrics) -> Result<()> {
        let mut store = self.metrics_store.write().await;

        store.resource_usage.cpu_usage_percent = metrics.cpu_usage_percent;
        store.resource_usage.memory_usage_percent =
            (metrics.memory_usage_mb as f32 / self.config.platform.max_memory_mb as f32) * 100.0;

        debug!(
            "Updated router metrics: CPU {:.1}%, Memory {:.1}%",
            store.resource_usage.cpu_usage_percent, store.resource_usage.memory_usage_percent
        );

        Ok(())
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let metrics = self.metrics_store.read().await;

        let mut health_metrics = HashMap::new();
        health_metrics.insert(
            "cpu_usage_percent".to_string(),
            metrics.resource_usage.cpu_usage_percent,
        );
        health_metrics.insert(
            "memory_usage_percent".to_string(),
            metrics.resource_usage.memory_usage_percent,
        );
        health_metrics.insert(
            "active_requests".to_string(),
            metrics.resource_usage.active_requests as f32,
        );
        health_metrics.insert(
            "decision_history_size".to_string(),
            metrics.request_history.len() as f32,
        );

        let status = if metrics.resource_usage.cpu_usage_percent > 95.0
            || metrics.resource_usage.memory_usage_percent > 95.0
        {
            HealthLevel::Critical
        } else if metrics.resource_usage.cpu_usage_percent > 85.0
            || metrics.resource_usage.memory_usage_percent > 85.0
        {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };

        let message = match status {
            HealthLevel::Healthy => "Router is operating normally".to_string(),
            HealthLevel::Warning => "Router is under high load".to_string(),
            HealthLevel::Critical => "Router is critically overloaded".to_string(),
            HealthLevel::Unknown => "Router status unknown".to_string(),
        };

        Ok(ComponentHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            metrics: health_metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down intelligent router");

        if let Some(client) = &self.cloud_client {
            client.shutdown().await?;
        }

        Ok(())
    }
}
