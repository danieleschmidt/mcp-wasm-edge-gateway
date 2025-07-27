// Integration tests for MCP WASM Edge Gateway
#![cfg(feature = "integration-tests")]

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

// Test utilities
mod common;
use common::*;

#[tokio::test]
async fn test_gateway_startup() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    // Test that gateway starts successfully
    assert!(gateway.is_running());
    
    // Test health endpoint
    let health_response = gateway.health_check().await?;
    assert_eq!(health_response.status, "healthy");
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_mcp_request_processing() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    let request = create_test_mcp_request("test prompt");
    let response = timeout(
        Duration::from_secs(10),
        gateway.process_request(request)
    ).await??;
    
    assert!(!response.content.is_empty());
    assert_eq!(response.status, "success");
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_offline_queue_functionality() -> Result<()> {
    let mut gateway = setup_test_gateway().await?;
    
    // Simulate offline mode
    gateway.set_offline_mode(true).await?;
    
    let request = create_test_mcp_request("offline test");
    let response = gateway.process_request(request.clone()).await?;
    
    // Should be queued
    assert_eq!(response.status, "queued");
    
    // Check queue status
    let queue_status = gateway.get_queue_status().await?;
    assert_eq!(queue_status.pending_requests, 1);
    
    // Go back online and sync
    gateway.set_offline_mode(false).await?;
    let sync_result = gateway.sync_queue().await?;
    assert_eq!(sync_result.synced_requests, 1);
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_model_routing() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    // Test simple request (should use local model)
    let simple_request = create_test_mcp_request("hello");
    let response = gateway.process_request(simple_request).await?;
    assert_eq!(response.routing_decision, "local");
    
    // Test complex request (should use cloud fallback)
    let complex_request = create_complex_mcp_request();
    let response = gateway.process_request(complex_request).await?;
    assert_eq!(response.routing_decision, "cloud");
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_security_features() -> Result<()> {
    let gateway = setup_test_gateway_with_security().await?;
    
    // Test authenticated request
    let auth_request = create_authenticated_request("test prompt")?;
    let response = gateway.process_request(auth_request).await?;
    assert_eq!(response.status, "success");
    
    // Test unauthenticated request
    let unauth_request = create_test_mcp_request("test prompt");
    let result = gateway.process_request(unauth_request).await;
    assert!(result.is_err());
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_telemetry_collection() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    // Process several requests
    for i in 0..5 {
        let request = create_test_mcp_request(&format!("test request {}", i));
        gateway.process_request(request).await?;
    }
    
    // Check metrics
    let metrics = gateway.get_metrics().await?;
    assert!(metrics.request_count >= 5);
    assert!(metrics.response_time_avg > Duration::from_millis(0));
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    // Test invalid request
    let invalid_request = create_invalid_mcp_request();
    let result = gateway.process_request(invalid_request).await;
    assert!(result.is_err());
    
    // Test network failure simulation
    gateway.simulate_network_failure(true).await?;
    let request = create_test_mcp_request("test");
    let response = gateway.process_request(request).await?;
    assert_eq!(response.status, "queued"); // Should fallback to queue
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_performance_under_load() -> Result<()> {
    let gateway = setup_test_gateway().await?;
    
    // Send concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let gateway_clone = gateway.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_mcp_request(&format!("concurrent test {}", i));
            gateway_clone.process_request(request).await
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let results: Result<Vec<_>, _> = futures::future::try_join_all(handles).await;
    let responses = results?;
    
    // Verify all requests succeeded
    for response in responses {
        let response = response?;
        assert_eq!(response.status, "success");
    }
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[tokio::test]
async fn test_configuration_management() -> Result<()> {
    let config = create_test_config();
    let gateway = setup_test_gateway_with_config(config).await?;
    
    // Test configuration retrieval
    let current_config = gateway.get_config().await?;
    assert_eq!(current_config.max_connections, 100);
    assert_eq!(current_config.request_timeout_ms, 5000);
    
    // Test configuration update
    let mut new_config = current_config.clone();
    new_config.max_connections = 200;
    gateway.update_config(new_config).await?;
    
    let updated_config = gateway.get_config().await?;
    assert_eq!(updated_config.max_connections, 200);
    
    teardown_test_gateway(gateway).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
mod wasm_specific_tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    async fn test_wasm_gateway_initialization() {
        let config = create_wasm_test_config();
        let gateway = setup_wasm_gateway(config).await.unwrap();
        
        assert!(gateway.is_running());
    }
    
    #[wasm_bindgen_test]
    async fn test_wasm_request_processing() {
        let gateway = setup_wasm_gateway_default().await.unwrap();
        
        let request = create_test_mcp_request("wasm test");
        let response = gateway.process_request(request).await.unwrap();
        
        assert_eq!(response.status, "success");
    }
}

#[cfg(feature = "hardware-security")]
mod hardware_security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tpm_attestation() -> Result<()> {
        let gateway = setup_test_gateway_with_tpm().await?;
        
        let attestation = gateway.get_device_attestation().await?;
        assert!(!attestation.certificate.is_empty());
        assert!(attestation.is_valid());
        
        teardown_test_gateway(gateway).await?;
        Ok(())
    }
    
    #[tokio::test]
    async fn test_secure_key_management() -> Result<()> {
        let gateway = setup_test_gateway_with_tpm().await?;
        
        // Test key generation
        let key_id = gateway.generate_device_key().await?;
        assert!(!key_id.is_empty());
        
        // Test key usage for signing
        let data = b"test data";
        let signature = gateway.sign_data(&key_id, data).await?;
        assert!(!signature.is_empty());
        
        // Test verification
        let is_valid = gateway.verify_signature(&key_id, data, &signature).await?;
        assert!(is_valid);
        
        teardown_test_gateway(gateway).await?;
        Ok(())
    }
}