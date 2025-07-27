//! Standard telemetry collector implementation

use crate::TelemetryCollector;
use async_trait::async_trait;
use mcp_common::metrics::{
    AggregatedMetrics, ComponentHealth, HealthLevel, QueueMetrics, RequestAggregates,
    SecurityMetrics, SystemMetrics,
};
use mcp_common::{Config, Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Standard telemetry collector implementation
pub struct StandardTelemetryCollector {
    config: Arc<Config>,
    metrics: Arc<RwLock<MetricsData>>,
}

#[derive(Debug, Default)]
struct MetricsData {
    total_requests: u32,
    successful_requests: u32,
    failed_requests: u32,
    total_latency_ms: u64,
    request_count: u32,
}

impl StandardTelemetryCollector {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self {
            config,
            metrics: Arc::new(RwLock::new(MetricsData::default())),
        })
    }
}

#[async_trait]
impl TelemetryCollector for StandardTelemetryCollector {
    async fn record_request_success(&self, request_id: Uuid, _response: &mcp_common::MCPResponse) {
        debug!("Recording successful request: {}", request_id);

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_latency_ms += 100; // Mock latency
        metrics.request_count += 1;
    }

    async fn record_request_error(&self, request_id: Uuid, error: &Error) {
        debug!("Recording failed request: {} - {}", request_id, error);

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.failed_requests += 1;
        metrics.request_count += 1;
    }

    async fn get_aggregated_metrics(&self) -> Result<AggregatedMetrics> {
        let metrics = self.metrics.read().await;

        let avg_latency = if metrics.successful_requests > 0 {
            metrics.total_latency_ms as f32 / metrics.successful_requests as f32
        } else {
            0.0
        };

        Ok(AggregatedMetrics {
            timestamp: chrono::Utc::now(),
            time_window_ms: 60000, // 1 minute window
            system: SystemMetrics {
                timestamp: chrono::Utc::now(),
                cpu_usage_percent: 25.0,
                memory_usage_mb: 256,
                memory_total_mb: 1024,
                disk_usage_mb: 5000,
                network_rx_bytes: 1024000,
                network_tx_bytes: 512000,
                temperature_celsius: Some(45.0),
                power_consumption_watts: Some(15.0),
            },
            requests: RequestAggregates {
                total_requests: metrics.total_requests,
                successful_requests: metrics.successful_requests,
                failed_requests: metrics.failed_requests,
                avg_latency_ms: avg_latency,
                p95_latency_ms: avg_latency * 1.5,
                p99_latency_ms: avg_latency * 2.0,
                requests_per_second: metrics.request_count as f32 / 60.0,
                local_processing_ratio: 0.8,
                cloud_fallback_ratio: 0.15,
                queue_ratio: 0.05,
            },
            models: Vec::new(),
            queue: QueueMetrics {
                queue_size: 0,
                pending_requests: 0,
                failed_requests: 0,
                sync_attempts: 0,
                sync_successes: 0,
                avg_sync_time_ms: 0,
                oldest_request_age_ms: 0,
            },
            security: SecurityMetrics {
                authentication_attempts: 0,
                authentication_failures: 0,
                authorization_denials: 0,
                encryption_operations: 0,
                key_rotations: 0,
                security_violations: 0,
                audit_events: 0,
            },
            custom: HashMap::new(),
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
