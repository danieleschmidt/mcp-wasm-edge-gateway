//! Configuration management for MCP Edge Gateway

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub gateway: GatewayConfig,
    pub router: RouterConfig,
    pub models: ModelsConfig,
    pub queue: QueueConfig,
    pub security: SecurityConfig,
    pub telemetry: TelemetryConfig,
    pub platform: PlatformConfig,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_connections: u32,
    pub request_timeout_ms: u64,
    pub max_request_size_bytes: u64,
    pub enable_cors: bool,
    pub cors_origins: Vec<String>,
}

/// Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub strategy: RoutingStrategy,
    pub local_processing_threshold: f32,
    pub cloud_fallback_enabled: bool,
    pub cloud_endpoints: Vec<CloudEndpoint>,
    pub load_balancing: LoadBalancingConfig,
}

/// Routing strategy options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    ComplexityBased,
    ResourceAware,
    PerformanceOptimized,
    Hybrid { weights: RoutingWeights },
}

/// Routing weights for hybrid strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingWeights {
    pub complexity: f32,
    pub resource_usage: f32,
    pub historical_performance: f32,
}

/// Cloud endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEndpoint {
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    pub algorithm: LoadBalancingAlgorithm,
    pub health_check_interval_ms: u64,
    pub failure_threshold: u32,
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    HealthBased,
}

/// Models configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub models_directory: PathBuf,
    pub cache_size_mb: u32,
    pub max_models_in_memory: u32,
    pub model_timeout_ms: u64,
    pub auto_optimization: bool,
    pub supported_formats: Vec<String>,
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub storage_path: PathBuf,
    pub max_queue_size: u32,
    pub sync_interval_ms: u64,
    pub retry_policy: RetryPolicy,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f32,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tpm_enabled: bool,
    pub mutual_tls: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub ca_cert_path: Option<PathBuf>,
    pub device_attestation: bool,
    pub encryption_algorithm: String,
    pub key_rotation_interval_hours: u64,
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub metrics_interval_ms: u64,
    pub export_endpoint: Option<String>,
    pub compression_enabled: bool,
    pub max_buffer_size: u32,
    pub retention_days: u32,
    pub prometheus_enabled: bool,
    pub opentelemetry_enabled: bool,
}

/// Platform-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub max_memory_mb: u32,
    pub max_cpu_usage_percent: f32,
    pub thermal_management: ThermalProfile,
    pub power_profile: PowerProfile,
    pub threading_model: ThreadingModel,
    pub enable_simd: bool,
    pub enable_gpu_acceleration: bool,
}

/// Thermal management profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalProfile {
    Passive,
    Moderate,
    Aggressive,
}

/// Power management profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerProfile {
    HighPerformance,
    Balanced,
    PowerSaving,
    UltraLowPower,
}

/// Threading model options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreadingModel {
    SingleThreaded,
    MultiThreaded { max_threads: u32 },
    WorkStealing,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 8080,
                max_connections: 1000,
                request_timeout_ms: 30000,
                max_request_size_bytes: 1024 * 1024, // 1MB
                enable_cors: true,
                cors_origins: vec!["*".to_string()],
            },
            router: RouterConfig {
                strategy: RoutingStrategy::Hybrid {
                    weights: RoutingWeights {
                        complexity: 0.4,
                        resource_usage: 0.4,
                        historical_performance: 0.2,
                    },
                },
                local_processing_threshold: 0.7,
                cloud_fallback_enabled: true,
                cloud_endpoints: Vec::new(),
                load_balancing: LoadBalancingConfig {
                    algorithm: LoadBalancingAlgorithm::HealthBased,
                    health_check_interval_ms: 30000,
                    failure_threshold: 3,
                },
            },
            models: ModelsConfig {
                models_directory: PathBuf::from("./models"),
                cache_size_mb: 512,
                max_models_in_memory: 3,
                model_timeout_ms: 60000,
                auto_optimization: true,
                supported_formats: vec![
                    "ggml".to_string(),
                    "onnx".to_string(),
                    "tflite".to_string(),
                ],
            },
            queue: QueueConfig {
                storage_path: PathBuf::from("./queue.db"),
                max_queue_size: 10000,
                sync_interval_ms: 5000,
                retry_policy: RetryPolicy {
                    max_retries: 3,
                    initial_delay_ms: 1000,
                    max_delay_ms: 60000,
                    backoff_multiplier: 2.0,
                },
                compression_enabled: true,
                encryption_enabled: true,
            },
            security: SecurityConfig {
                tpm_enabled: false,
                mutual_tls: false,
                cert_path: None,
                key_path: None,
                ca_cert_path: None,
                device_attestation: false,
                encryption_algorithm: "AES-256-GCM".to_string(),
                key_rotation_interval_hours: 24,
            },
            telemetry: TelemetryConfig {
                enabled: true,
                metrics_interval_ms: 10000,
                export_endpoint: None,
                compression_enabled: true,
                max_buffer_size: 1000,
                retention_days: 7,
                prometheus_enabled: true,
                opentelemetry_enabled: false,
            },
            platform: PlatformConfig {
                max_memory_mb: 512,
                max_cpu_usage_percent: 80.0,
                thermal_management: ThermalProfile::Moderate,
                power_profile: PowerProfile::Balanced,
                threading_model: ThreadingModel::MultiThreaded {
                    max_threads: 4,
                },
                enable_simd: true,
                enable_gpu_acceleration: false,
            },
        }
    }
}
