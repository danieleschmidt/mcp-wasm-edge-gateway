//! Metrics and monitoring utilities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u32,
    pub memory_total_mb: u32,
    pub disk_usage_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub temperature_celsius: Option<f32>,
    pub power_consumption_watts: Option<f32>,
}

/// Request metrics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    pub request_id: String,
    pub method: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub status: RequestStatus,
    pub routing_decision: String,
    pub model_used: Option<String>,
    pub memory_used_mb: u32,
    pub cpu_time_ms: u64,
}

/// Request status for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestStatus {
    Success,
    Error(String),
    Timeout,
    Queued,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub model_id: String,
    pub load_time_ms: u64,
    pub inference_time_ms: u64,
    pub memory_usage_mb: u32,
    pub accuracy_score: Option<f32>,
    pub throughput_requests_per_sec: f32,
    pub error_rate: f32,
    pub last_used: DateTime<Utc>,
}

/// Queue metrics for monitoring offline operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub queue_size: u32,
    pub pending_requests: u32,
    pub failed_requests: u32,
    pub sync_attempts: u32,
    pub sync_successes: u32,
    pub avg_sync_time_ms: u64,
    pub oldest_request_age_ms: u64,
}

/// Security metrics for audit and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub authentication_attempts: u32,
    pub authentication_failures: u32,
    pub authorization_denials: u32,
    pub encryption_operations: u32,
    pub key_rotations: u32,
    pub security_violations: u32,
    pub audit_events: u32,
}

/// Aggregated metrics for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub timestamp: DateTime<Utc>,
    pub time_window_ms: u64,
    pub system: SystemMetrics,
    pub requests: RequestAggregates,
    pub models: Vec<ModelMetrics>,
    pub queue: QueueMetrics,
    pub security: SecurityMetrics,
    pub custom: HashMap<String, f32>,
}

/// Request aggregates for summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestAggregates {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub avg_latency_ms: f32,
    pub p95_latency_ms: f32,
    pub p99_latency_ms: f32,
    pub requests_per_second: f32,
    pub local_processing_ratio: f32,
    pub cloud_fallback_ratio: f32,
    pub queue_ratio: f32,
}

/// Health status of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_health: HealthLevel,
    pub components: HashMap<String, ComponentHealth>,
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
}

/// Health levels for components
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Critical,
    Unknown,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthLevel,
    pub message: String,
    pub last_check: DateTime<Utc>,
    pub metrics: HashMap<String, f32>,
}

impl HealthLevel {
    /// Get numeric value for comparison
    pub fn as_score(&self) -> u8 {
        match self {
            HealthLevel::Healthy => 100,
            HealthLevel::Degraded => 50,
            HealthLevel::Critical => 10,
            HealthLevel::Unknown => 0,
        }
    }
}

impl HealthStatus {
    /// Calculate overall health from component health
    pub fn calculate_overall_health(&mut self) {
        if self.components.is_empty() {
            self.overall_health = HealthLevel::Unknown;
            return;
        }

        let critical_count = self
            .components
            .values()
            .filter(|c| c.status == HealthLevel::Critical)
            .count();

        let degraded_count = self
            .components
            .values()
            .filter(|c| c.status == HealthLevel::Degraded)
            .count();

        self.overall_health = if critical_count > 0 {
            HealthLevel::Critical
        } else if degraded_count > 0 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };
    }
}
