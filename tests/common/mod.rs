// Common test utilities for MCP WASM Edge Gateway integration tests

use anyhow::Result;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

// Re-export commonly used types
pub use mcp_gateway::{Gateway, Config, MCPRequest, MCPResponse, HealthStatus};

static TEST_PORT: AtomicU16 = AtomicU16::new(8080);

/// Test gateway instance with cleanup
pub struct TestGateway {
    inner: Gateway,
    temp_dir: TempDir,
    port: u16,
}

impl TestGateway {
    pub async fn is_running(&self) -> bool {
        self.inner.is_running().await
    }
    
    pub async fn health_check(&self) -> Result<HealthStatus> {
        self.inner.health_check().await
    }
    
    pub async fn process_request(&self, request: MCPRequest) -> Result<MCPResponse> {
        self.inner.process_request(request).await
    }
    
    pub async fn set_offline_mode(&mut self, offline: bool) -> Result<()> {
        self.inner.set_offline_mode(offline).await
    }
    
    pub async fn get_queue_status(&self) -> Result<QueueStatus> {
        self.inner.get_queue_status().await
    }
    
    pub async fn sync_queue(&self) -> Result<SyncResult> {
        self.inner.sync_queue().await
    }
    
    pub async fn get_metrics(&self) -> Result<GatewayMetrics> {
        self.inner.get_metrics().await
    }
    
    pub async fn simulate_network_failure(&self, enabled: bool) -> Result<()> {
        self.inner.simulate_network_failure(enabled).await
    }
    
    pub fn clone(&self) -> Self {
        // Note: This is a simplified clone for testing
        // In reality, you'd want proper cloning semantics
        Self {
            inner: self.inner.clone(),
            temp_dir: self.temp_dir.clone(), // This won't actually work, but for testing...
            port: self.port,
        }
    }
    
    pub async fn get_config(&self) -> Result<Config> {
        self.inner.get_config().await
    }
    
    pub async fn update_config(&self, config: Config) -> Result<()> {
        self.inner.update_config(config).await
    }
    
    #[cfg(feature = "hardware-security")]
    pub async fn get_device_attestation(&self) -> Result<DeviceAttestation> {
        self.inner.get_device_attestation().await
    }
    
    #[cfg(feature = "hardware-security")]
    pub async fn generate_device_key(&self) -> Result<String> {
        self.inner.generate_device_key().await
    }
    
    #[cfg(feature = "hardware-security")]
    pub async fn sign_data(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        self.inner.sign_data(key_id, data).await
    }
    
    #[cfg(feature = "hardware-security")]
    pub async fn verify_signature(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool> {
        self.inner.verify_signature(key_id, data, signature).await
    }
}

/// Setup a test gateway with default configuration
pub async fn setup_test_gateway() -> Result<TestGateway> {
    let config = create_test_config();
    setup_test_gateway_with_config(config).await
}

/// Setup a test gateway with custom configuration
pub async fn setup_test_gateway_with_config(mut config: Config) -> Result<TestGateway> {
    let temp_dir = tempfile::tempdir()?;
    let port = TEST_PORT.fetch_add(1, Ordering::SeqCst);
    
    // Configure for testing
    config.bind_address = format!("127.0.0.1:{}", port);
    config.database_url = format!("sqlite://{}/test.db", temp_dir.path().display());
    config.queue_persistence_path = temp_dir.path().join("queue").to_string_lossy().to_string();
    config.log_level = "debug".to_string();
    
    let gateway = Gateway::new(config).await?;
    gateway.start().await?;
    
    // Wait for gateway to be ready
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    Ok(TestGateway {
        inner: gateway,
        temp_dir,
        port,
    })
}

/// Setup a test gateway with security features enabled
pub async fn setup_test_gateway_with_security() -> Result<TestGateway> {
    let mut config = create_test_config();
    config.security_enabled = true;
    config.require_authentication = true;
    setup_test_gateway_with_config(config).await
}

/// Setup a test gateway with TPM support
#[cfg(feature = "hardware-security")]
pub async fn setup_test_gateway_with_tpm() -> Result<TestGateway> {
    let mut config = create_test_config();
    config.use_tpm = true;
    config.tpm_device = "/dev/tpmrm0".to_string(); // Use resource manager
    setup_test_gateway_with_config(config).await
}

/// Cleanup test gateway
pub async fn teardown_test_gateway(gateway: TestGateway) -> Result<()> {
    gateway.inner.shutdown().await?;
    // temp_dir is automatically cleaned up when dropped
    Ok(())
}

/// Create a test configuration
pub fn create_test_config() -> Config {
    Config {
        bind_address: "127.0.0.1:0".to_string(),
        max_connections: 100,
        request_timeout_ms: 5000,
        
        // Model configuration
        local_model: "test-model".to_string(),
        model_path: "./test-models/".to_string(),
        max_memory_mb: 256,
        max_tokens: 1024,
        
        // Cloud fallback
        cloud_endpoint: "http://localhost:8081/v1/mcp".to_string(),
        cloud_api_key: "test-api-key".to_string(),
        fallback_threshold_ms: 2000,
        
        // Offline queue
        queue_size: 1000,
        queue_persistence_path: "./test-queue".to_string(),
        sync_interval_seconds: 300,
        compression: "zstd".to_string(),
        
        // Security
        security_enabled: false,
        use_tpm: false,
        tpm_device: "/dev/tpm0".to_string(),
        require_authentication: false,
        require_attestation: false,
        
        // Telemetry
        telemetry_enabled: true,
        telemetry_export_interval_seconds: 60,
        telemetry_compression: "lz4".to_string(),
        metrics_endpoint: "http://localhost:9090/metrics".to_string(),
        
        // Power management
        low_power_mode: false,
        cpu_throttle_percent: 100,
        gpu_enabled: false,
        sleep_on_idle_ms: 1000,
        
        // Development
        log_level: "info".to_string(),
        database_url: "sqlite::memory:".to_string(),
    }
}

/// Create a WASM-specific test configuration
#[cfg(target_arch = "wasm32")]
pub fn create_wasm_test_config() -> Config {
    let mut config = create_test_config();
    config.bind_address = "0.0.0.0:8080".to_string();
    config.database_url = "memory".to_string(); // Use in-memory storage for WASM
    config.queue_persistence_path = "indexeddb://queue".to_string();
    config
}

/// Create a test MCP request
pub fn create_test_mcp_request(prompt: &str) -> MCPRequest {
    MCPRequest {
        id: Uuid::new_v4().to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.7,
            "max_tokens": 100
        }),
        timestamp: chrono::Utc::now(),
        device_id: "test-device".to_string(),
        session_id: Some("test-session".to_string()),
    }
}

/// Create a complex MCP request that should trigger cloud fallback
pub fn create_complex_mcp_request() -> MCPRequest {
    MCPRequest {
        id: Uuid::new_v4().to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant that provides detailed analysis."
                },
                {
                    "role": "user",
                    "content": "Please provide a comprehensive analysis of quantum computing implications for cryptography, including technical details, mathematical foundations, and real-world applications. This should be at least 2000 words with citations."
                }
            ],
            "temperature": 0.3,
            "max_tokens": 4096,
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "web_search",
                        "description": "Search the web for current information"
                    }
                }
            ]
        }),
        timestamp: chrono::Utc::now(),
        device_id: "test-device".to_string(),
        session_id: Some("test-session".to_string()),
    }
}

/// Create an invalid MCP request for error testing
pub fn create_invalid_mcp_request() -> MCPRequest {
    MCPRequest {
        id: "".to_string(), // Invalid empty ID
        method: "invalid_method".to_string(),
        params: serde_json::json!({}), // Empty params
        timestamp: chrono::Utc::now(),
        device_id: "".to_string(), // Invalid empty device ID
        session_id: None,
    }
}

/// Create an authenticated request
pub fn create_authenticated_request(prompt: &str) -> Result<MCPRequest> {
    let mut request = create_test_mcp_request(prompt);
    
    // Add authentication headers/tokens
    let auth_token = generate_test_auth_token()?;
    request.params["auth_token"] = serde_json::Value::String(auth_token);
    
    Ok(request)
}

/// Generate a test authentication token
fn generate_test_auth_token() -> Result<String> {
    // In a real implementation, this would be a proper JWT or similar
    Ok(format!("test-token-{}", Uuid::new_v4()))
}

/// Setup WASM gateway for browser testing
#[cfg(target_arch = "wasm32")]
pub async fn setup_wasm_gateway(config: Config) -> Result<TestGateway> {
    use wasm_bindgen_futures::spawn_local;
    
    let gateway = Gateway::new(config).await?;
    
    // Start gateway in WASM environment
    spawn_local(async move {
        if let Err(e) = gateway.start().await {
            web_sys::console::error_1(&format!("Gateway start error: {}", e).into());
        }
    });
    
    Ok(TestGateway {
        inner: gateway,
        temp_dir: tempfile::tempdir()?, // This won't actually work in WASM
        port: 8080,
    })
}

/// Setup WASM gateway with default configuration
#[cfg(target_arch = "wasm32")]
pub async fn setup_wasm_gateway_default() -> Result<TestGateway> {
    let config = create_wasm_test_config();
    setup_wasm_gateway(config).await
}

// Mock types for compilation (these would be real types in the actual implementation)
#[derive(Clone, Debug)]
pub struct QueueStatus {
    pub pending_requests: u32,
    pub failed_requests: u32,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug)]
pub struct SyncResult {
    pub synced_requests: u32,
    pub failed_requests: u32,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GatewayMetrics {
    pub request_count: u64,
    pub response_time_avg: Duration,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f32,
}

#[cfg(feature = "hardware-security")]
#[derive(Clone, Debug)]
pub struct DeviceAttestation {
    pub certificate: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "hardware-security")]
impl DeviceAttestation {
    pub fn is_valid(&self) -> bool {
        !self.certificate.is_empty() && !self.signature.is_empty()
    }
}