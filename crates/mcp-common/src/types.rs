//! Common types for the MCP Edge Gateway

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for MCP requests
pub type RequestId = Uuid;

/// Unique identifier for devices
pub type DeviceId = String;

/// Unique identifier for models
pub type ModelId = String;

/// MCP Request structure following the Model Context Protocol specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub id: RequestId,
    pub device_id: DeviceId,
    pub method: String,
    pub params: HashMap<String, serde_json::Value>,
    pub context: Option<RequestContext>,
    pub timestamp: DateTime<Utc>,
}

/// MCP Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub id: RequestId,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
    pub timestamp: DateTime<Utc>,
}

/// MCP Error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Request context for processing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub priority: Priority,
    pub timeout_ms: Option<u64>,
    pub retry_count: u32,
    pub source: RequestSource,
    pub requirements: ProcessingRequirements,
}

/// Request priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Source of the request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestSource {
    Local,
    Remote(String),
    Queue,
}

/// Processing requirements for requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingRequirements {
    pub max_latency_ms: Option<u64>,
    pub min_accuracy: Option<f32>,
    pub max_memory_mb: Option<u32>,
    pub require_local: bool,
    pub allow_fallback: bool,
    pub pii_present: Option<bool>,
}

/// Routing decision for requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingDecision {
    Local {
        model_id: ModelId,
        estimated_latency_ms: u64,
    },
    Cloud {
        endpoint: String,
        estimated_latency_ms: u64,
    },
    Queue {
        reason: String,
        retry_after_ms: u64,
    },
}

/// Hardware capabilities of the device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub has_tpm: bool,
    pub has_gpu: bool,
    pub max_cpu_cores: u32,
    pub memory_bandwidth_gbps: f32,
    pub total_memory_mb: u32,
    pub available_memory_mb: u32,
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub latency_ms: u64,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f32,
    pub throughput_requests_per_sec: f32,
    pub error_rate: f32,
    pub timestamp: DateTime<Utc>,
}

/// Model metadata and information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: ModelId,
    pub name: String,
    pub version: String,
    pub format: ModelFormat,
    pub size_bytes: u64,
    pub memory_requirement_mb: u32,
    pub supported_operations: Vec<String>,
    pub accuracy_metrics: HashMap<String, f32>,
}

/// Supported model formats
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ModelFormat {
    GGML,
    ONNX,
    TensorFlowLite,
    Custom(String),
}

/// Device status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_id: DeviceId,
    pub online: bool,
    pub last_seen: DateTime<Utc>,
    pub capabilities: HardwareCapabilities,
    pub current_load: PerformanceMetrics,
    pub health_score: f32,
}



