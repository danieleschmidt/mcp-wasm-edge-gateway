// Test utilities for MCP WASM Edge Gateway
// Provides common functionality for testing across all test types

pub mod fixtures;
pub mod mock_services;
pub mod test_server;
pub mod assertions;
pub mod performance;

use std::path::PathBuf;
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use serde::{Deserialize, Serialize};

static INIT: Once = Once::new();

/// Initialize test environment - call once per test run
pub fn init_test_env() {
    INIT.call_once(|| {
        // Initialize logging for tests
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
        
        // Clean up any leftover test artifacts
        cleanup_test_artifacts();
        
        // Create necessary test directories
        create_test_directories();
    });
}

/// Configuration for test environment
#[derive(Debug, Clone, Deserialize)]
pub struct TestConfig {
    pub test_environment: TestEnvironment,
    pub mock_services: MockServices,
    pub test_data: TestData,
    pub logging: LoggingConfig,
    pub coverage: CoverageConfig,
    pub benchmarks: BenchmarkConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestEnvironment {
    pub db_path: String,
    pub clean_db_between_tests: bool,
    pub use_mock_models: bool,
    pub mock_model_path: String,
    pub model_load_timeout_ms: u64,
    pub test_server_port: u16,
    pub test_metrics_port: u16,
    pub bind_to_localhost_only: bool,
    pub disable_tpm_in_tests: bool,
    pub use_test_certificates: bool,
    pub test_cert_path: String,
    pub performance_test_duration_seconds: u64,
    pub max_concurrent_requests: usize,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MockServices {
    pub enable_cloud_service_mock: bool,
    pub mock_cloud_latency_ms: u64,
    pub mock_cloud_success_rate: f64,
    pub mock_hardware_security: bool,
    pub mock_sensor_data: bool,
    pub simulate_low_memory: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestData {
    pub fixtures_path: String,
    pub generate_test_data: bool,
    pub test_data_size: String,
    pub sample_requests_file: String,
    pub stress_test_requests: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub log_level: String,
    pub log_to_file: bool,
    pub log_file_path: String,
    pub capture_stdout: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverageConfig {
    pub enable_coverage: bool,
    pub coverage_threshold: f64,
    pub exclude_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    pub run_benchmarks: bool,
    pub benchmark_iterations: usize,
    pub warm_up_iterations: usize,
    pub measurement_time_seconds: u64,
}

/// Load test configuration from file
pub fn load_test_config() -> TestConfig {
    let config_path = PathBuf::from("tests/test_config.toml");
    let config_str = std::fs::read_to_string(config_path)
        .expect("Failed to read test configuration file");
    
    toml::from_str(&config_str)
        .expect("Failed to parse test configuration")
}

/// Test result tracking
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
}

/// Performance metrics for testing
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub requests_per_second: f64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Async timeout wrapper for tests
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future)
        .await
        .map_err(|_| "Test timed out".to_string())
}

/// Wait for condition with timeout
pub async fn wait_for_condition<F>(
    condition: F,
    timeout_duration: Duration,
    check_interval: Duration,
) -> Result<(), String>
where
    F: Fn() -> bool,
{
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout_duration {
        if condition() {
            return Ok(());
        }
        tokio::time::sleep(check_interval).await;
    }
    
    Err("Condition not met within timeout".to_string())
}

/// Generate test data
pub fn generate_test_request(size: &str) -> serde_json::Value {
    match size {
        "small" => serde_json::json!({
            "messages": [{"role": "user", "content": "Hello"}],
            "model": "test-model"
        }),
        "medium" => serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Explain quantum computing in simple terms."}
            ],
            "model": "test-model",
            "temperature": 0.7,
            "max_tokens": 200
        }),
        "large" => serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are an expert in multiple fields."},
                {"role": "user", "content": format!("Analyze this large text: {}", "Lorem ipsum ".repeat(100))}
            ],
            "model": "test-model",
            "temperature": 0.5,
            "max_tokens": 1000,
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "analyze_text",
                        "description": "Analyze text for patterns"
                    }
                }
            ]
        }),
        _ => serde_json::json!({
            "messages": [{"role": "user", "content": "Default test message"}],
            "model": "test-model"
        }),
    }
}

/// Random test data generation
pub struct TestDataGenerator {
    pub rng: fastrand::Rng,
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self {
            rng: fastrand::Rng::new(),
        }
    }
    
    pub fn random_string(&self, length: usize) -> String {
        (0..length)
            .map(|_| self.rng.alphanumeric())
            .collect()
    }
    
    pub fn random_message(&self) -> String {
        let templates = vec![
            "What is {topic}?",
            "Explain {topic} in simple terms",
            "How does {topic} work?",
            "What are the benefits of {topic}?",
            "Compare {topic} with alternatives",
        ];
        
        let topics = vec![
            "edge computing", "WASM", "Rust", "AI", "IoT", 
            "microservices", "containers", "security", "performance"
        ];
        
        let template = templates[self.rng.usize(..templates.len())];
        let topic = topics[self.rng.usize(..topics.len())];
        
        template.replace("{topic}", topic)
    }
    
    pub fn random_temperature(&self) -> f64 {
        self.rng.f64() * 2.0 // 0.0 to 2.0
    }
    
    pub fn random_max_tokens(&self) -> u32 {
        self.rng.u32(10..=1000)
    }
}

/// Cleanup test artifacts
fn cleanup_test_artifacts() {
    let _ = std::fs::remove_dir_all("/tmp/mcp_test_artifacts");
    let _ = std::fs::remove_file("/tmp/mcp_test.db");
}

/// Create necessary test directories
fn create_test_directories() {
    let dirs = vec![
        "/tmp/mcp_test_artifacts",
        "/tmp/mcp_test_logs",
        "/tmp/mcp_test_data",
    ];
    
    for dir in dirs {
        let _ = std::fs::create_dir_all(dir);
    }
}

/// Macro for creating test assertions
#[macro_export]
macro_rules! assert_within_range {
    ($value:expr, $min:expr, $max:expr) => {
        assert!(
            $value >= $min && $value <= $max,
            "Value {} is not within range [{}, {}]",
            $value, $min, $max
        );
    };
}

/// Macro for performance assertions
#[macro_export]
macro_rules! assert_performance {
    ($duration:expr, $max_ms:expr) => {
        assert!(
            $duration.as_millis() <= $max_ms as u128,
            "Performance assertion failed: {} ms > {} ms",
            $duration.as_millis(),
            $max_ms
        );
    };
}

/// Macro for async test setup
#[macro_export]
macro_rules! async_test {
    ($test_fn:ident) => {
        #[tokio::test]
        async fn $test_fn() {
            crate::utils::init_test_env();
            $test_fn().await;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_loading() {
        // This would normally load from file
        // For now, just test the structure
        let config = TestConfig {
            test_environment: TestEnvironment {
                db_path: "/tmp/test.db".to_string(),
                clean_db_between_tests: true,
                use_mock_models: true,
                mock_model_path: "./tests/fixtures/models".to_string(),
                model_load_timeout_ms: 5000,
                test_server_port: 18080,
                test_metrics_port: 19090,
                bind_to_localhost_only: true,
                disable_tpm_in_tests: true,
                use_test_certificates: true,
                test_cert_path: "./tests/fixtures/certs".to_string(),
                performance_test_duration_seconds: 10,
                max_concurrent_requests: 100,
                request_timeout_ms: 1000,
            },
            mock_services: MockServices {
                enable_cloud_service_mock: true,
                mock_cloud_latency_ms: 100,
                mock_cloud_success_rate: 0.95,
                mock_hardware_security: true,
                mock_sensor_data: true,
                simulate_low_memory: false,
            },
            test_data: TestData {
                fixtures_path: "./tests/fixtures".to_string(),
                generate_test_data: true,
                test_data_size: "small".to_string(),
                sample_requests_file: "./tests/fixtures/sample_requests.json".to_string(),
                stress_test_requests: 1000,
            },
            logging: LoggingConfig {
                log_level: "debug".to_string(),
                log_to_file: true,
                log_file_path: "./tests/logs/test.log".to_string(),
                capture_stdout: true,
            },
            coverage: CoverageConfig {
                enable_coverage: true,
                coverage_threshold: 80.0,
                exclude_files: vec!["tests/".to_string()],
            },
            benchmarks: BenchmarkConfig {
                run_benchmarks: false,
                benchmark_iterations: 100,
                warm_up_iterations: 10,
                measurement_time_seconds: 5,
            },
        };
        
        assert_eq!(config.test_environment.test_server_port, 18080);
        assert!(config.test_environment.use_mock_models);
    }
    
    #[test]
    fn test_data_generator() {
        let generator = TestDataGenerator::new();
        
        let string = generator.random_string(10);
        assert_eq!(string.len(), 10);
        
        let message = generator.random_message();
        assert!(!message.is_empty());
        
        let temp = generator.random_temperature();
        assert!(temp >= 0.0 && temp <= 2.0);
    }
}