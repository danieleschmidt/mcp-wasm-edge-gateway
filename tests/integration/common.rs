use std::sync::Once;
use std::time::Duration;

use mcp_wasm_edge_gateway::{Config, Gateway};
use tokio::time::timeout;

static INIT: Once = Once::new();

pub fn init_test_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .init();
    });
}

pub struct TestGateway {
    pub gateway: Gateway,
    pub config: Config,
}

impl TestGateway {
    pub async fn new() -> Self {
        init_test_logging();
        
        let config = Config::builder()
            .bind_address("127.0.0.1:0") // Use random port
            .max_connections(10)
            .request_timeout_ms(1000)
            .local_model("test-model")
            .max_memory_mb(64)
            .enable_cloud_fallback(false)
            .offline_queue_size(100)
            .telemetry_enabled(false)
            .low_power_mode(false)
            .build()
            .expect("Failed to create test config");

        let gateway = Gateway::new(config.clone())
            .await
            .expect("Failed to create test gateway");

        Self { gateway, config }
    }

    pub async fn start(&self) -> String {
        let addr = self.gateway.start().await
            .expect("Failed to start test gateway");
        format!("http://{}", addr)
    }

    pub async fn stop(self) {
        self.gateway.shutdown().await
            .expect("Failed to shutdown test gateway");
    }
}

pub async fn with_timeout<F, T>(duration: Duration, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future)
        .await
        .expect("Test timed out")
}

pub fn test_config() -> Config {
    Config::builder()
        .bind_address("127.0.0.1:0")
        .max_connections(10)
        .request_timeout_ms(1000)
        .local_model("test-model")
        .max_memory_mb(64)
        .enable_cloud_fallback(false)
        .offline_queue_size(100)
        .telemetry_enabled(false)
        .low_power_mode(true)
        .build()
        .expect("Failed to create test config")
}

pub async fn wait_for_condition<F>(mut condition: F, timeout_duration: Duration)
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while !condition() {
        if start.elapsed() > timeout_duration {
            panic!("Condition not met within timeout");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[macro_export]
macro_rules! assert_eventually {
    ($condition:expr, $timeout:expr) => {
        $crate::common::wait_for_condition(|| $condition, $timeout).await;
    };
}

pub use assert_eventually;