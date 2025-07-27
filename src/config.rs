// Configuration module for MCP WASM Edge Gateway

use serde::{Deserialize, Serialize};

/// Main configuration structure for the gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server bind address
    pub bind_address: String,
    /// Maximum number of concurrent connections
    pub max_connections: u32,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    
    // Model configuration
    /// Local model name
    pub local_model: String,
    /// Path to model files
    pub model_path: String,
    /// Maximum memory usage in MB
    pub max_memory_mb: u32,
    /// Maximum tokens per request
    pub max_tokens: u32,
    
    // Cloud fallback
    /// Cloud MCP endpoint URL
    pub cloud_endpoint: String,
    /// Cloud API key
    pub cloud_api_key: String,
    /// Fallback threshold in milliseconds
    pub fallback_threshold_ms: u64,
    
    // Offline queue
    /// Maximum queue size
    pub queue_size: u32,
    /// Queue persistence path
    pub queue_persistence_path: String,
    /// Sync interval in seconds
    pub sync_interval_seconds: u64,
    /// Compression algorithm
    pub compression: String,
    
    // Security
    /// Enable security features
    pub security_enabled: bool,
    /// Use TPM for hardware security
    pub use_tpm: bool,
    /// TPM device path
    pub tpm_device: String,
    /// Require authentication
    pub require_authentication: bool,
    /// Require device attestation
    pub require_attestation: bool,
    
    // Telemetry
    /// Enable telemetry collection
    pub telemetry_enabled: bool,
    /// Telemetry export interval in seconds
    pub telemetry_export_interval_seconds: u64,
    /// Telemetry compression
    pub telemetry_compression: String,
    /// Metrics endpoint URL
    pub metrics_endpoint: String,
    
    // Power management
    /// Enable low power mode
    pub low_power_mode: bool,
    /// CPU throttle percentage
    pub cpu_throttle_percent: u32,
    /// Enable GPU acceleration
    pub gpu_enabled: bool,
    /// Sleep on idle timeout in milliseconds
    pub sleep_on_idle_ms: u64,
    
    // Development
    /// Log level
    pub log_level: String,
    /// Database URL
    pub database_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            max_connections: 100,
            request_timeout_ms: 5000,
            
            local_model: "phi-3-mini-q4".to_string(),
            model_path: "/opt/models/".to_string(),
            max_memory_mb: 512,
            max_tokens: 1024,
            
            cloud_endpoint: "https://api.example.com/v1/mcp".to_string(),
            cloud_api_key: "".to_string(),
            fallback_threshold_ms: 2000,
            
            queue_size: 1000,
            queue_persistence_path: "/var/lib/mcp/queue".to_string(),
            sync_interval_seconds: 300,
            compression: "zstd".to_string(),
            
            security_enabled: false,
            use_tpm: false,
            tpm_device: "/dev/tpm0".to_string(),
            require_authentication: false,
            require_attestation: false,
            
            telemetry_enabled: true,
            telemetry_export_interval_seconds: 60,
            telemetry_compression: "lz4".to_string(),
            metrics_endpoint: "http://localhost:9090/metrics".to_string(),
            
            low_power_mode: false,
            cpu_throttle_percent: 100,
            gpu_enabled: true,
            sleep_on_idle_ms: 1000,
            
            log_level: "info".to_string(),
            database_url: "sqlite:///var/lib/mcp/gateway.db".to_string(),
        }
    }
}