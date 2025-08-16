//! MCP Telemetry - Monitoring and metrics collection for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth};
use mcp_common::{Config, Error, Result};
use std::sync::Arc;
use uuid::Uuid;

/// Telemetry collector trait for metrics and monitoring
#[async_trait]
pub trait TelemetryCollector {
    /// Record a successful request
    async fn record_request_success(&self, request_id: Uuid, response: &mcp_common::MCPResponse);

    /// Record a failed request
    async fn record_request_error(&self, request_id: Uuid, error: &Error);

    /// Get aggregated metrics
    async fn get_aggregated_metrics(&self) -> Result<mcp_common::metrics::AggregatedMetrics>;

    /// Get health status
    async fn health_check(&self) -> Result<ComponentHealth>;

    /// Shutdown the telemetry collector
    async fn shutdown(&self) -> Result<()>;
}

mod standard_telemetry;

pub use standard_telemetry::{StandardTelemetryCollector, TelemetryConfig, PerformanceSummary};

/// Create a new telemetry collector instance
pub async fn create_telemetry_collector(
    _config: Arc<Config>,
) -> Result<Arc<dyn TelemetryCollector + Send + Sync>> {
    let collector = StandardTelemetryCollector::new();
    Ok(Arc::new(collector))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_telemetry_creation() {
        let config = TelemetryConfig::default();
        let collector = StandardTelemetryCollector::new_with_config(config).await.unwrap();
        
        // Verify basic functionality
        let health = collector.get_health().await;
        assert!(matches!(health, ComponentHealth::Healthy { .. }));
    }
    
    #[tokio::test]
    async fn test_metric_recording() {
        let config = TelemetryConfig::default();
        let collector = StandardTelemetryCollector::new_with_config(config).await.unwrap();
        
        // Record some metrics
        collector.record_request_latency(100).await;
        collector.record_error("test_error").await;
        
        let summary = collector.get_performance_summary().await;
        assert_eq!(summary.total_requests, 1);
        assert_eq!(summary.total_errors, 1);
    }
    
    #[tokio::test]
    async fn test_performance_percentiles() {
        let config = TelemetryConfig::default();
        let collector = StandardTelemetryCollector::new_with_config(config).await.unwrap();
        
        // Record multiple latencies
        for latency in [50, 100, 150, 200, 250] {
            collector.record_request_latency(latency).await;
        }
        
        let summary = collector.get_performance_summary().await;
        assert!(summary.p50_latency_ms > 0);
        assert!(summary.p95_latency_ms >= summary.p50_latency_ms);
    }
    
    #[tokio::test]
    async fn test_create_telemetry_collector() {
        let config = Arc::new(Config::default());
        let result = create_telemetry_collector(config).await;
        assert!(result.is_ok());
    }
}
