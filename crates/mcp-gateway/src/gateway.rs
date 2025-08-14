//! Core gateway implementation

use mcp_common::{Config, Error, MCPRequest, MCPResponse, Result};
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_models::ModelEngine;
use mcp_queue::OfflineQueue;
use mcp_router::Router;
use mcp_security::SecurityManager;
use mcp_telemetry::TelemetryCollector;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Main gateway component that orchestrates all other components
pub struct Gateway {
    config: Arc<Config>,
    router: Arc<dyn Router + Send + Sync>,
    model_engine: Arc<dyn ModelEngine + Send + Sync>,
    queue: Arc<dyn OfflineQueue + Send + Sync>,
    security: Arc<dyn SecurityManager + Send + Sync>,
    telemetry: Arc<dyn TelemetryCollector + Send + Sync>,
    state: Arc<RwLock<GatewayState>>,
}

/// Gateway state for monitoring and control
#[derive(Debug, Clone)]
pub struct GatewayState {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub active_requests: u32,
    pub total_requests: u64,
    pub last_health_check: chrono::DateTime<chrono::Utc>,
    pub is_healthy: bool,
}

impl Gateway {
    /// Create a new gateway instance
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing MCP Gateway");

        let config = Arc::new(config);

        // Initialize components
        let router = mcp_router::create_router(config.clone()).await?;
        let model_engine = mcp_models::create_model_engine(config.clone()).await?;
        let queue = mcp_queue::create_offline_queue(config.clone()).await?;
        let security = mcp_security::create_security_manager(config.clone()).await?;
        let telemetry = mcp_telemetry::create_telemetry_collector(config.clone()).await?;

        let state = Arc::new(RwLock::new(GatewayState {
            started_at: chrono::Utc::now(),
            active_requests: 0,
            total_requests: 0,
            last_health_check: chrono::Utc::now(),
            is_healthy: true,
        }));

        info!("Gateway initialized successfully");

        Ok(Gateway {
            config,
            router,
            model_engine,
            queue,
            security,
            telemetry,
            state,
        })
    }

    /// Process an MCP request
    pub async fn process_request(&self, mut request: MCPRequest) -> Result<MCPResponse> {
        let request_id = request.id;
        debug!("Processing request {}", request_id);

        // Update state
        {
            let mut state = self.state.write().await;
            state.active_requests += 1;
            state.total_requests += 1;
        }

        let result = self.process_request_internal(request).await;

        // Update state
        {
            let mut state = self.state.write().await;
            state.active_requests = state.active_requests.saturating_sub(1);
        }

        match &result {
            Ok(response) => {
                debug!("Request {} completed successfully", request_id);
                self.telemetry
                    .record_request_success(request_id, response)
                    .await;
            },
            Err(error) => {
                error!("Request {} failed: {}", request_id, error);
                self.telemetry.record_request_error(request_id, error).await;
            },
        }

        result
    }

    async fn process_request_internal(&self, request: MCPRequest) -> Result<MCPResponse> {
        // Security validation
        self.security.validate_request(&request).await?;

        // Route the request
        let routing_decision = self.router.route(&request).await?;

        // Process based on routing decision
        let response = match routing_decision {
            mcp_common::RoutingDecision::Local {
                model_id,
                ..
            } => {
                self.model_engine
                    .process_request(&request, &model_id)
                    .await?
            },
            mcp_common::RoutingDecision::Cloud {
                endpoint,
                ..
            } => self.router.forward_to_cloud(&request, &endpoint).await?,
            mcp_common::RoutingDecision::Queue {
                reason,
                ..
            } => {
                let request_id = request.id;
                self.queue.enqueue_request(request).await?;
                MCPResponse {
                    id: request_id,
                    result: Some(serde_json::json!({
                        "status": "queued",
                        "reason": reason
                    })),
                    error: None,
                    timestamp: chrono::Utc::now(),
                }
            },
        };

        Ok(response)
    }

    /// Get gateway configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get current gateway state
    pub async fn state(&self) -> GatewayState {
        self.state.read().await.clone()
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<mcp_common::HealthStatus> {
        debug!("Performing health check");

        let mut health_status = mcp_common::HealthStatus {
            overall_health: HealthLevel::Healthy,
            components: std::collections::HashMap::new(),
            last_check: chrono::Utc::now(),
            uptime_seconds: {
                let state = self.state.read().await;
                chrono::Utc::now()
                    .signed_duration_since(state.started_at)
                    .num_seconds() as u64
            },
        };

        // Check each component
        health_status.components.insert(
            "router".to_string(),
            self.router
                .health_check()
                .await
                .unwrap_or_else(|_| ComponentHealth {
                    status: HealthLevel::Critical,
                    message: "Router health check failed".to_string(),
                    last_check: chrono::Utc::now(),
                    metrics: std::collections::HashMap::new(),
                }),
        );

        health_status.components.insert(
            "model_engine".to_string(),
            self.model_engine.health_check().await.unwrap_or_else(|_| {
                ComponentHealth {
                    status: HealthLevel::Critical,
                    message: "Model engine health check failed".to_string(),
                    last_check: chrono::Utc::now(),
                    metrics: std::collections::HashMap::new(),
                }
            }),
        );

        health_status.components.insert(
            "queue".to_string(),
            self.queue
                .health_check()
                .await
                .unwrap_or_else(|_| ComponentHealth {
                    status: HealthLevel::Critical,
                    message: "Queue health check failed".to_string(),
                    last_check: chrono::Utc::now(),
                    metrics: std::collections::HashMap::new(),
                }),
        );

        health_status.components.insert(
            "security".to_string(),
            self.security
                .health_check()
                .await
                .unwrap_or_else(|_| ComponentHealth {
                    status: HealthLevel::Critical,
                    message: "Security health check failed".to_string(),
                    last_check: chrono::Utc::now(),
                    metrics: std::collections::HashMap::new(),
                }),
        );

        health_status.components.insert(
            "telemetry".to_string(),
            self.telemetry
                .health_check()
                .await
                .unwrap_or_else(|_| ComponentHealth {
                    status: HealthLevel::Critical,
                    message: "Telemetry health check failed".to_string(),
                    last_check: chrono::Utc::now(),
                    metrics: std::collections::HashMap::new(),
                }),
        );

        // Calculate overall health
        health_status.calculate_overall_health();

        // Update gateway state
        {
            let mut state = self.state.write().await;
            state.last_health_check = chrono::Utc::now();
            state.is_healthy = health_status.overall_health == HealthLevel::Healthy;
        }

        Ok(health_status)
    }

    /// Get component metrics
    pub async fn get_metrics(&self) -> Result<mcp_common::metrics::AggregatedMetrics> {
        self.telemetry.get_aggregated_metrics().await
    }

    /// Shutdown the gateway gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down gateway");

        // Shutdown components in reverse order
        if let Err(e) = self.telemetry.shutdown().await {
            error!("Error shutting down telemetry: {}", e);
        }

        if let Err(e) = self.security.shutdown().await {
            error!("Error shutting down security: {}", e);
        }

        if let Err(e) = self.queue.shutdown().await {
            error!("Error shutting down queue: {}", e);
        }

        if let Err(e) = self.model_engine.shutdown().await {
            error!("Error shutting down model engine: {}", e);
        }

        if let Err(e) = self.router.shutdown().await {
            error!("Error shutting down router: {}", e);
        }

        info!("Gateway shutdown complete");
        Ok(())
    }
}
