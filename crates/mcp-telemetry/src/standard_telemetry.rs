//! Standard telemetry collector implementation

use mcp_common::{Error, Result, RequestId, MCPRequest, MCPResponse};
use mcp_common::metrics::{
    ComponentHealth, HealthLevel, AggregatedMetrics, SystemMetrics, RequestAggregates,
    QueueMetrics, SecurityMetrics
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use chrono::{DateTime, Utc, Timelike};
use uuid::Uuid;

use crate::TelemetryCollector;

/// Standard implementation of telemetry collector
pub struct StandardTelemetryCollector {
    metrics: Arc<RwLock<TelemetryMetrics>>,
    config: TelemetryConfig,
}

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub retention_hours: u32,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            retention_hours: 24,
            batch_size: 100,
            flush_interval_seconds: 60,
        }
    }
}

/// Internal telemetry metrics storage
#[derive(Debug, Default)]
struct TelemetryMetrics {
    request_count: u64,
    success_count: u64,
    error_count: u64,
    total_latency_ms: u64,
    last_flush: DateTime<Utc>,
    
    // Enhanced metrics for Generation 2
    request_latencies: Vec<u64>,              // For percentile calculations
    error_categories: HashMap<String, u64>,   // Error categorization
    hourly_request_counts: HashMap<u8, u64>,  // Requests by hour of day
    component_health_scores: HashMap<String, f32>, // Component health tracking
    alert_count: u64,                         // Number of alerts triggered
    circuit_breaker_states: HashMap<String, bool>, // Circuit breaker status
    memory_usage_samples: Vec<u32>,           // Memory usage over time
    cpu_usage_samples: Vec<f32>,              // CPU usage over time
    active_connections: u32,                  // Current active connections
    peak_connections: u32,                    // Peak concurrent connections
    last_error_time: Option<DateTime<Utc>>,   // Last error timestamp
    recovery_attempts: u64,                   // Recovery operation count
}

impl StandardTelemetryCollector {
    /// Create a new standard telemetry collector
    pub fn new() -> Self {
        Self::with_config(TelemetryConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: TelemetryConfig) -> Self {
        StandardTelemetryCollector {
            metrics: Arc::new(RwLock::new(TelemetryMetrics::default())),
            config,
        }
    }
}

#[async_trait::async_trait]
impl TelemetryCollector for StandardTelemetryCollector {
    async fn record_request_success(&self, request_id: Uuid, _response: &MCPResponse) {
        debug!("Recording successful request: {}", request_id);
        
        let mut metrics = self.metrics.write().await;
        metrics.request_count += 1;
        metrics.success_count += 1;
    }

    async fn record_request_error(&self, request_id: Uuid, error: &Error) {
        debug!("Recording failed request: {} - {}", request_id, error);
        
        let mut metrics = self.metrics.write().await;
        metrics.request_count += 1;
        metrics.error_count += 1;
    }

    async fn get_aggregated_metrics(&self) -> Result<AggregatedMetrics> {
        let metrics = self.metrics.read().await;
        
        Ok(AggregatedMetrics {
            timestamp: Utc::now(),
            time_window_ms: 60000, // 1 minute window
            system: SystemMetrics {
                timestamp: Utc::now(),
                cpu_usage_percent: 10.0,
                memory_usage_mb: 128,
                memory_total_mb: 512,
                disk_usage_mb: 1024,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
                temperature_celsius: None,
                power_consumption_watts: None,
            },
            requests: RequestAggregates {
                total_requests: metrics.request_count as u32,
                successful_requests: metrics.success_count as u32,
                failed_requests: metrics.error_count as u32,
                avg_latency_ms: if metrics.success_count > 0 {
                    (metrics.total_latency_ms / metrics.success_count) as f32
                } else {
                    0.0
                },
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                requests_per_second: 0.0,
                local_processing_ratio: 1.0,
                cloud_fallback_ratio: 0.0,
                queue_ratio: 0.0,
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
        Ok(ComponentHealth {
            status: HealthLevel::Healthy,
            message: "Telemetry collector operational".to_string(),
            last_check: Utc::now(),
            metrics: HashMap::new(),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down telemetry collector");
        Ok(())
    }
}

impl StandardTelemetryCollector {
    /// Record detailed request metrics with latency and context
    pub async fn record_detailed_request(&self, 
        request_id: Uuid, 
        latency_ms: u64, 
        success: bool, 
        error_category: Option<&str>
    ) {
        let mut metrics = self.metrics.write().await;
        metrics.request_count += 1;
        
        // Record latency for percentile calculations
        metrics.request_latencies.push(latency_ms);
        
        // Keep only recent latencies (last 1000 requests)
        if metrics.request_latencies.len() > 1000 {
            metrics.request_latencies.drain(0..100);
        }
        
        // Record by hour for usage patterns
        let hour = Utc::now().hour() as u8;
        *metrics.hourly_request_counts.entry(hour).or_insert(0) += 1;
        
        if success {
            metrics.success_count += 1;
        } else {
            metrics.error_count += 1;
            metrics.last_error_time = Some(Utc::now());
            
            if let Some(category) = error_category {
                *metrics.error_categories.entry(category.to_string()).or_insert(0) += 1;
            }
        }
        
        metrics.total_latency_ms += latency_ms;
    }

    /// Record system resource usage
    pub async fn record_system_metrics(&self, cpu_percent: f32, memory_mb: u32) {
        let mut metrics = self.metrics.write().await;
        
        // Keep rolling window of samples
        metrics.cpu_usage_samples.push(cpu_percent);
        metrics.memory_usage_samples.push(memory_mb);
        
        // Keep only recent samples (last 100)
        if metrics.cpu_usage_samples.len() > 100 {
            metrics.cpu_usage_samples.drain(0..10);
        }
        if metrics.memory_usage_samples.len() > 100 {
            metrics.memory_usage_samples.drain(0..10);
        }
    }

    /// Record component health score
    pub async fn record_component_health(&self, component: &str, health_score: f32) {
        let mut metrics = self.metrics.write().await;
        metrics.component_health_scores.insert(component.to_string(), health_score);
    }

    /// Record alert triggered
    pub async fn record_alert(&self, component: &str, severity: u8, message: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.alert_count += 1;
        
        tracing::warn!(
            component = component,
            severity = severity,
            message = message,
            "Alert triggered"
        );
    }

    /// Record circuit breaker state change
    pub async fn record_circuit_breaker(&self, component: &str, is_open: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.circuit_breaker_states.insert(component.to_string(), is_open);
        
        tracing::info!(
            component = component,
            state = if is_open { "OPEN" } else { "CLOSED" },
            "Circuit breaker state changed"
        );
    }

    /// Record connection metrics
    pub async fn record_connection_opened(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.active_connections += 1;
        
        if metrics.active_connections > metrics.peak_connections {
            metrics.peak_connections = metrics.active_connections;
        }
    }

    /// Record connection closed
    pub async fn record_connection_closed(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
    }

    /// Record recovery attempt
    pub async fn record_recovery_attempt(&self, component: &str, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.recovery_attempts += 1;
        
        tracing::info!(
            component = component,
            success = success,
            total_attempts = metrics.recovery_attempts,
            "Recovery attempt recorded"
        );
    }

    /// Get detailed performance metrics
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.metrics.read().await;
        
        // Calculate percentiles
        let mut sorted_latencies = metrics.request_latencies.clone();
        sorted_latencies.sort_unstable();
        
        let p50 = percentile(&sorted_latencies, 50.0);
        let p95 = percentile(&sorted_latencies, 95.0);
        let p99 = percentile(&sorted_latencies, 99.0);
        
        // Calculate averages
        let avg_cpu = if !metrics.cpu_usage_samples.is_empty() {
            metrics.cpu_usage_samples.iter().sum::<f32>() / metrics.cpu_usage_samples.len() as f32
        } else {
            0.0
        };
        
        let avg_memory = if !metrics.memory_usage_samples.is_empty() {
            metrics.memory_usage_samples.iter().sum::<u32>() / metrics.memory_usage_samples.len() as u32
        } else {
            0
        };
        
        let success_rate = if metrics.request_count > 0 {
            (metrics.success_count as f32 / metrics.request_count as f32) * 100.0
        } else {
            100.0
        };

        PerformanceSummary {
            total_requests: metrics.request_count,
            success_rate,
            avg_latency_ms: if metrics.request_count > 0 {
                metrics.total_latency_ms / metrics.request_count
            } else {
                0
            },
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            avg_cpu_percent: avg_cpu,
            avg_memory_mb: avg_memory,
            active_connections: metrics.active_connections,
            peak_connections: metrics.peak_connections,
            alert_count: metrics.alert_count,
            recovery_attempts: metrics.recovery_attempts,
            error_categories: metrics.error_categories.clone(),
            component_health_scores: metrics.component_health_scores.clone(),
        }
    }

    /// Check if any alerts should be triggered based on current metrics
    pub async fn check_alert_conditions(&self) -> Vec<AlertCondition> {
        let summary = self.get_performance_summary().await;
        let mut alerts = Vec::new();

        // High error rate alert
        if summary.success_rate < 95.0 {
            alerts.push(AlertCondition {
                component: "gateway".to_string(),
                severity: 3,
                message: format!("Low success rate: {:.1}%", summary.success_rate),
                metric_name: "success_rate".to_string(),
                current_value: summary.success_rate,
                threshold: 95.0,
            });
        }

        // High latency alert
        if summary.p95_latency_ms > 5000 {
            alerts.push(AlertCondition {
                component: "gateway".to_string(),
                severity: 2,
                message: format!("High P95 latency: {}ms", summary.p95_latency_ms),
                metric_name: "p95_latency_ms".to_string(),
                current_value: summary.p95_latency_ms as f32,
                threshold: 5000.0,
            });
        }

        // High CPU usage alert
        if summary.avg_cpu_percent > 80.0 {
            alerts.push(AlertCondition {
                component: "system".to_string(),
                severity: 2,
                message: format!("High CPU usage: {:.1}%", summary.avg_cpu_percent),
                metric_name: "cpu_usage_percent".to_string(),
                current_value: summary.avg_cpu_percent,
                threshold: 80.0,
            });
        }

        // High memory usage alert
        if summary.avg_memory_mb > 400 {
            alerts.push(AlertCondition {
                component: "system".to_string(),
                severity: 2,
                message: format!("High memory usage: {}MB", summary.avg_memory_mb),
                metric_name: "memory_usage_mb".to_string(),
                current_value: summary.avg_memory_mb as f32,
                threshold: 400.0,
            });
        }

        alerts
    }
}

/// Performance summary for monitoring dashboards
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_requests: u64,
    pub success_rate: f32,
    pub avg_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub avg_cpu_percent: f32,
    pub avg_memory_mb: u32,
    pub active_connections: u32,
    pub peak_connections: u32,
    pub alert_count: u64,
    pub recovery_attempts: u64,
    pub error_categories: HashMap<String, u64>,
    pub component_health_scores: HashMap<String, f32>,
}

/// Alert condition that needs attention
#[derive(Debug, Clone)]
pub struct AlertCondition {
    pub component: String,
    pub severity: u8, // 1=info, 2=warning, 3=error, 4=critical
    pub message: String,
    pub metric_name: String,
    pub current_value: f32,
    pub threshold: f32,
}

/// Calculate percentile from sorted values
fn percentile(sorted_values: &[u64], percentile: f32) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    
    let index = ((percentile / 100.0) * (sorted_values.len() - 1) as f32).round() as usize;
    sorted_values.get(index).copied().unwrap_or(0)
}