//! Standard telemetry collector implementation

use crate::TelemetryCollector;
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, Error, MCPResponse, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Simple telemetry metrics
#[derive(Debug, Default)]
struct TelemetryMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_latency_ms: u64,
}

/// Standard telemetry collector
pub struct StandardTelemetryCollector {
    config: Arc<Config>,
    metrics: Arc<RwLock<TelemetryMetrics>>,
}

impl StandardTelemetryCollector {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        info!("Initializing telemetry collector");
        
        Ok(Self {
            config,
            metrics: Arc::new(RwLock::new(TelemetryMetrics::default())),
        })
    }
}

#[async_trait]
impl TelemetryCollector for StandardTelemetryCollector {
    async fn record_request_success(&self, request_id: Uuid, _response: &MCPResponse) {
        debug!("Recording successful request: {}", request_id);

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_latency_ms += 100; // Mock latency
    }

    async fn record_request_error(&self, request_id: Uuid, error: &Error) {
        debug!("Recording failed request: {} - {}", request_id, error);

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.failed_requests += 1;
    }

    async fn get_aggregated_metrics(&self) -> Result<mcp_common::AggregatedMetrics> {
        let metrics = self.metrics.read().await;

        let avg_latency = if metrics.successful_requests > 0 {
            metrics.total_latency_ms as f64 / metrics.successful_requests as f64
        } else {
            0.0
        };

        let success_rate = if metrics.total_requests > 0 {
            metrics.successful_requests as f32 / metrics.total_requests as f32
        } else {
            1.0
        };

        Ok(mcp_common::AggregatedMetrics {
            total_requests: metrics.total_requests,
            success_rate,
            avg_latency_ms: avg_latency,
            memory_usage_mb: 256,
            cpu_usage_percent: 25.0,
            active_models: 1,
            queue_size: 0,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let mut health_metrics = HashMap::new();
        let metrics = self.metrics.read().await;

        health_metrics.insert("total_requests".to_string(), metrics.total_requests as f32);
        health_metrics.insert(
            "success_rate".to_string(),
            if metrics.total_requests > 0 {
                metrics.successful_requests as f32 / metrics.total_requests as f32
            } else {
                1.0
            },
        );

        Ok(ComponentHealth {
            status: HealthLevel::Healthy,
            message: "Telemetry collector is operational".to_string(),
            last_check: chrono::Utc::now(),
            metrics: health_metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down telemetry collector");
        Ok(())
    }
}