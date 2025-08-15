//! MCP Telemetry - Monitoring and metrics collection for the MCP Edge Gateway

use async_trait::async_trait;
use mcp_common::metrics::{AggregatedMetrics, ComponentHealth};
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

pub use standard_telemetry::StandardTelemetryCollector;

/// Create a new telemetry collector instance
pub async fn create_telemetry_collector(
    _config: Arc<Config>,
) -> Result<Arc<dyn TelemetryCollector + Send + Sync>> {
    let collector = StandardTelemetryCollector::new();
    Ok(Arc::new(collector))
}
