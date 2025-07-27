use std::time::Duration;

use reqwest::Client;
use serde_json::json;

use crate::common::{with_timeout, TestGateway};

#[tokio::test]
async fn test_gateway_startup_shutdown() {
    let gateway = TestGateway::new().await;
    
    // Test startup
    let addr = with_timeout(Duration::from_secs(5), gateway.start()).await;
    assert!(addr.starts_with("http://127.0.0.1:"));
    
    // Test shutdown
    with_timeout(Duration::from_secs(5), gateway.stop()).await;
}

#[tokio::test]
async fn test_health_check_endpoint() {
    let gateway = TestGateway::new().await;
    let base_url = gateway.start().await;
    
    let client = Client::new();
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .expect("Failed to make health check request");
    
    assert_eq!(response.status(), 200);
    
    let health_data: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse health check response");
    
    assert_eq!(health_data["status"], "ok");
    assert!(health_data["timestamp"].is_string());
    
    gateway.stop().await;
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let gateway = TestGateway::new().await;
    let base_url = gateway.start().await;
    
    let client = Client::new();
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to make metrics request");
    
    assert_eq!(response.status(), 200);
    
    let metrics_text = response
        .text()
        .await
        .expect("Failed to get metrics text");
    
    // Check for basic Prometheus metrics format
    assert!(metrics_text.contains("# HELP"));
    assert!(metrics_text.contains("# TYPE"));
    
    gateway.stop().await;
}

#[tokio::test]
async fn test_concurrent_requests() {
    let gateway = TestGateway::new().await;
    let base_url = gateway.start().await;
    
    let client = Client::new();
    let mut handles = vec![];
    
    // Spawn multiple concurrent health check requests
    for _ in 0..10 {
        let client = client.clone();
        let base_url = base_url.clone();
        
        let handle = tokio::spawn(async move {
            let response = client
                .get(&format!("{}/health", base_url))
                .send()
                .await
                .expect("Failed to make health check request");
            
            assert_eq!(response.status(), 200);
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        handle.await.expect("Request task failed");
    }
    
    gateway.stop().await;
}

#[tokio::test]
async fn test_request_timeout() {
    let gateway = TestGateway::new().await;
    let base_url = gateway.start().await;
    
    let client = Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to create client");
    
    // This should succeed as health check is fast
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .expect("Failed to make health check request");
    
    assert_eq!(response.status(), 200);
    
    gateway.stop().await;
}

#[tokio::test]
async fn test_invalid_endpoints() {
    let gateway = TestGateway::new().await;
    let base_url = gateway.start().await;
    
    let client = Client::new();
    
    // Test non-existent endpoint
    let response = client
        .get(&format!("{}/nonexistent", base_url))
        .send()
        .await
        .expect("Failed to make request");
    
    assert_eq!(response.status(), 404);
    
    gateway.stop().await;
}

#[tokio::test]
async fn test_gateway_configuration() {
    use crate::common::test_config;
    
    let config = test_config();
    
    // Verify configuration values
    assert_eq!(config.max_connections(), 10);
    assert_eq!(config.request_timeout_ms(), 1000);
    assert_eq!(config.local_model(), "test-model");
    assert_eq!(config.max_memory_mb(), 64);
    assert!(!config.enable_cloud_fallback());
    assert_eq!(config.offline_queue_size(), 100);
    assert!(!config.telemetry_enabled());
    assert!(config.low_power_mode());
}