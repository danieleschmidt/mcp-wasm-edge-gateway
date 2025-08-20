//! Intelligent routing implementation for MCP requests

use crate::{cloud_client::CloudClient, load_balancer::LoadBalancer, Router};
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{
    Config, Error, MCPRequest, MCPResponse, RequestContext, Result, RoutingDecision, Priority,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Intelligent router that makes routing decisions based on request complexity,
/// system resources, and historical performance
pub struct IntelligentRouter {
    config: Arc<Config>,
    cloud_client: Arc<CloudClient>,
    load_balancer: Arc<LoadBalancer>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    routing_state: Arc<RwLock<RoutingState>>,
}

/// Performance metrics for routing decisions
#[derive(Debug, Default)]
struct PerformanceMetrics {
    local_avg_latency_ms: f32,
    local_success_rate: f32,
    cloud_avg_latency_ms: f32,
    cloud_success_rate: f32,
    local_processing_count: u64,
    cloud_processing_count: u64,
    recent_local_failures: u32,
    recent_cloud_failures: u32,
}

/// Current state for routing decisions
#[derive(Debug, Default)]
struct RoutingState {
    local_capacity_percent: f32,
    memory_usage_percent: f32,
    active_local_requests: u32,
    queue_size: u32,
    last_health_check: chrono::DateTime<chrono::Utc>,
}

impl IntelligentRouter {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let cloud_client = Arc::new(CloudClient::new(config.clone()).await?);
        let load_balancer = Arc::new(LoadBalancer::new(config.clone())?);

        Ok(Self {
            config,
            cloud_client,
            load_balancer,
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            routing_state: Arc::new(RwLock::new(RoutingState::default())),
        })
    }

    /// Analyze request complexity to determine processing requirements
    async fn analyze_request_complexity(&self, request: &MCPRequest) -> f32 {
        let mut complexity = 0.0;

        // Method complexity
        complexity += match request.method.as_str() {
            "completion" => 0.8,
            "chat" => 0.7,
            "embedding" => 0.3,
            "classification" => 0.4,
            "summarization" => 0.6,
            _ => 0.5,
        };

        // Parameter complexity
        let param_count = request.params.len();
        complexity += (param_count as f32) * 0.05;

        // Context complexity (if present)
        if let Some(context) = &request.context {
            complexity += match context.priority {
                Priority::Critical => 0.1,
                Priority::High => 0.05,
                Priority::Normal => 0.0,
                Priority::Low => -0.05,
            };

            if context.requirements.max_latency_ms.is_some() {
                complexity += 0.1; // Time-sensitive requests are more complex
            }

            if context.requirements.require_local {
                complexity += 0.2; // Local-only processing may need more resources
            }
        }

        // Content size heuristic
        if let Some(content) = request.params.get("content") {
            if let Some(text) = content.as_str() {
                let char_count = text.len();
                complexity += (char_count as f32 / 10000.0).min(0.3); // Cap at 0.3
            }
        }

        complexity.min(1.0)
    }

    /// Estimate local processing capability for this request
    async fn estimate_local_capability(&self, complexity: f32) -> f32 {
        let state = self.routing_state.read().await;
        let metrics = self.performance_metrics.read().await;

        let mut capability_score = 1.0;

        // Adjust based on current system load
        capability_score *= 1.0 - (state.local_capacity_percent / 100.0);
        capability_score *= 1.0 - (state.memory_usage_percent / 100.0);

        // Adjust based on active requests
        let max_concurrent = 10.0; // Configurable
        capability_score *= 1.0 - (state.active_local_requests as f32 / max_concurrent).min(0.9);

        // Adjust based on historical performance
        if metrics.local_processing_count > 0 {
            capability_score *= metrics.local_success_rate;
        }

        // Apply complexity penalty
        capability_score *= 1.0 - (complexity * 0.3);

        // Recent failure penalty
        if metrics.recent_local_failures > 3 {
            capability_score *= 0.5;
        }

        capability_score.max(0.0).min(1.0)
    }

    /// Estimate cloud processing benefit for this request
    async fn estimate_cloud_benefit(&self, complexity: f32) -> f32 {
        let metrics = self.performance_metrics.read().await;

        let mut benefit_score = 0.7; // Base cloud benefit

        // High complexity benefits from cloud
        if complexity > 0.7 {
            benefit_score += 0.2;
        }

        // Adjust based on historical cloud performance
        if metrics.cloud_processing_count > 0 {
            benefit_score *= metrics.cloud_success_rate;
            
            // Prefer cloud if it's significantly faster for complex requests
            if complexity > 0.6 && metrics.cloud_avg_latency_ms < metrics.local_avg_latency_ms * 0.8 {
                benefit_score += 0.15;
            }
        }

        // Recent failure penalty
        if metrics.recent_cloud_failures > 3 {
            benefit_score *= 0.5;
        }

        benefit_score.max(0.0).min(1.0)
    }

    /// Make routing decision based on analysis
    async fn make_routing_decision(
        &self,
        request: &MCPRequest,
        complexity: f32,
        local_capability: f32,
        cloud_benefit: f32,
    ) -> Result<RoutingDecision> {
        let state = self.routing_state.read().await;

        // Check for explicit requirements
        if let Some(context) = &request.context {
            if context.requirements.require_local {
                if local_capability > 0.3 {
                    return Ok(RoutingDecision::Local {
                        model_id: "default".to_string(), // TODO: Implement model selection
                        estimated_latency_ms: 200,
                    });
                } else {
                    return Ok(RoutingDecision::Queue {
                        reason: "Local processing required but insufficient capacity".to_string(),
                        retry_after_ms: 5000,
                    });
                }
            }

            if !context.requirements.allow_fallback {
                // No fallback allowed, must decide between local and queue
                if local_capability > 0.5 {
                    return Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: 300,
                    });
                } else {
                    return Ok(RoutingDecision::Queue {
                        reason: "Insufficient local capacity and fallback not allowed".to_string(),
                        retry_after_ms: 3000,
                    });
                }
            }

            // Handle latency requirements
            if let Some(max_latency) = context.requirements.max_latency_ms {
                if max_latency < 500 && local_capability > 0.4 {
                    // Low latency requirement favors local
                    return Ok(RoutingDecision::Local {
                        model_id: "default".to_string(),
                        estimated_latency_ms: 150,
                    });
                }
            }
        }

        // Decision logic based on scores
        let threshold = self.config.router.local_processing_threshold;

        if local_capability >= threshold && local_capability > cloud_benefit {
            Ok(RoutingDecision::Local {
                model_id: "default".to_string(), // TODO: Implement intelligent model selection
                estimated_latency_ms: (200.0 * (1.0 + complexity)).round() as u64,
            })
        } else if self.config.router.cloud_fallback_enabled && cloud_benefit > 0.5 {
            // Select best cloud endpoint
            let endpoint = self.load_balancer.select_endpoint().await?;
            Ok(RoutingDecision::Cloud {
                endpoint: endpoint.url.clone(),
                estimated_latency_ms: (300.0 * (1.0 + complexity * 0.5)).round() as u64,
            })
        } else {
            // Queue for later processing
            Ok(RoutingDecision::Queue {
                reason: format!(
                    "Insufficient capacity (local: {:.2}, cloud: {:.2})",
                    local_capability, cloud_benefit
                ),
                retry_after_ms: if state.queue_size < 100 { 2000 } else { 5000 },
            })
        }
    }

    /// Update system state for routing decisions
    pub async fn update_system_state(
        &self,
        cpu_usage: f32,
        memory_usage: f32,
        active_requests: u32,
        queue_size: u32,
    ) {
        let mut state = self.routing_state.write().await;
        state.local_capacity_percent = cpu_usage;
        state.memory_usage_percent = memory_usage;
        state.active_local_requests = active_requests;
        state.queue_size = queue_size;
        state.last_health_check = chrono::Utc::now();
    }

    /// Update performance metrics based on request outcome
    pub async fn record_request_outcome(
        &self,
        is_local: bool,
        latency_ms: u64,
        success: bool,
    ) {
        let mut metrics = self.performance_metrics.write().await;

        if is_local {
            metrics.local_processing_count += 1;
            
            // Update rolling average latency
            let alpha = 0.1; // Smoothing factor
            metrics.local_avg_latency_ms = if metrics.local_processing_count == 1 {
                latency_ms as f32
            } else {
                alpha * latency_ms as f32 + (1.0 - alpha) * metrics.local_avg_latency_ms
            };

            // Update success rate
            let total = metrics.local_processing_count as f32;
            let current_successes = (metrics.local_success_rate * (total - 1.0)).round();
            metrics.local_success_rate = if success {
                (current_successes + 1.0) / total
            } else {
                current_successes / total
            };

            // Track recent failures
            if success {
                metrics.recent_local_failures = 0;
            } else {
                metrics.recent_local_failures += 1;
            }
        } else {
            metrics.cloud_processing_count += 1;
            
            // Update rolling average latency
            let alpha = 0.1;
            metrics.cloud_avg_latency_ms = if metrics.cloud_processing_count == 1 {
                latency_ms as f32
            } else {
                alpha * latency_ms as f32 + (1.0 - alpha) * metrics.cloud_avg_latency_ms
            };

            // Update success rate
            let total = metrics.cloud_processing_count as f32;
            let current_successes = (metrics.cloud_success_rate * (total - 1.0)).round();
            metrics.cloud_success_rate = if success {
                (current_successes + 1.0) / total
            } else {
                current_successes / total
            };

            // Track recent failures
            if success {
                metrics.recent_cloud_failures = 0;
            } else {
                metrics.recent_cloud_failures += 1;
            }
        }
    }
}

#[async_trait]
impl Router for IntelligentRouter {
    async fn route(&self, request: &MCPRequest) -> Result<RoutingDecision> {
        debug!("Routing request {} (method: {})", request.id, request.method);

        // Analyze request complexity
        let complexity = self.analyze_request_complexity(request).await;
        debug!("Request complexity: {:.2}", complexity);

        // Estimate local processing capability
        let local_capability = self.estimate_local_capability(complexity).await;
        debug!("Local capability: {:.2}", local_capability);

        // Estimate cloud processing benefit
        let cloud_benefit = self.estimate_cloud_benefit(complexity).await;
        debug!("Cloud benefit: {:.2}", cloud_benefit);

        // Make routing decision
        let decision = self.make_routing_decision(request, complexity, local_capability, cloud_benefit).await?;
        
        match &decision {
            RoutingDecision::Local { model_id, estimated_latency_ms } => {
                info!("Routing request {} to local model {} (estimated latency: {}ms)", 
                      request.id, model_id, estimated_latency_ms);
            },
            RoutingDecision::Cloud { endpoint, estimated_latency_ms } => {
                info!("Routing request {} to cloud endpoint {} (estimated latency: {}ms)", 
                      request.id, endpoint, estimated_latency_ms);
            },
            RoutingDecision::Queue { reason, retry_after_ms } => {
                info!("Queueing request {} (reason: {}, retry after: {}ms)", 
                      request.id, reason, retry_after_ms);
            },
        }

        Ok(decision)
    }

    async fn forward_to_cloud(&self, request: &MCPRequest, endpoint: &str) -> Result<MCPResponse> {
        debug!("Forwarding request {} to cloud endpoint: {}", request.id, endpoint);
        
        let start_time = std::time::Instant::now();
        let result = self.cloud_client.send_request(endpoint, request).await;
        let latency = start_time.elapsed().as_millis() as u64;

        // Record the outcome for learning
        self.record_request_outcome(false, latency, result.is_ok()).await;

        result
    }

    async fn update_metrics(&self, _metrics: &mcp_common::PerformanceMetrics) -> Result<()> {
        // TODO: Integrate with system metrics
        Ok(())
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let state = self.routing_state.read().await;
        let metrics = self.performance_metrics.read().await;

        let mut health_metrics = HashMap::new();
        health_metrics.insert("local_capacity_percent".to_string(), state.local_capacity_percent);
        health_metrics.insert("memory_usage_percent".to_string(), state.memory_usage_percent);
        health_metrics.insert("active_requests".to_string(), state.active_local_requests as f32);
        health_metrics.insert("queue_size".to_string(), state.queue_size as f32);
        health_metrics.insert("local_success_rate".to_string(), metrics.local_success_rate);
        health_metrics.insert("cloud_success_rate".to_string(), metrics.cloud_success_rate);

        let status = if state.local_capacity_percent > 95.0 || state.memory_usage_percent > 95.0 {
            HealthLevel::Critical
        } else if state.local_capacity_percent > 85.0 || state.memory_usage_percent > 85.0 || state.queue_size > 1000 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };

        let message = match status {
            HealthLevel::Healthy => "Router is operating normally".to_string(),
            HealthLevel::Degraded => "Router experiencing high load".to_string(),
            HealthLevel::Critical => "Router at capacity".to_string(),
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
        self.cloud_client.shutdown().await?;
        Ok(())
    }
}