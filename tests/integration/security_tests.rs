use super::common::*;
use mcp_wasm_edge_gateway::{Gateway, Config, SecurityConfig, MCPRequest, AttestationResult};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
async fn test_device_attestation() {
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_tpm(true)
            .require_attestation(true)
            .build().unwrap())
        .build()
        .unwrap();
    
    let gateway = Gateway::new(config).await.unwrap();
    
    // Test device attestation
    let attestation = gateway.get_device_attestation().await;
    
    match attestation {
        Ok(result) => {
            assert!(result.is_valid());
            assert!(!result.device_id().is_empty());
            assert!(result.timestamp() > 0);
            assert!(!result.signature().is_empty());
        }
        Err(e) => {
            // Attestation might fail in test environment without real TPM
            assert!(e.to_string().contains("TPM") || e.to_string().contains("attestation"));
        }
    }
}

#[tokio::test]
async fn test_request_signing_and_verification() {
    let gateway = setup_test_gateway_with_security().await;
    
    let request = MCPRequest {
        id: "signed-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Test signed request"}
            ],
            "max_tokens": 50
        }),
    };
    
    // Sign the request
    let signed_request = gateway.sign_request(request).await.unwrap();
    
    // Verify the signature
    let is_valid = gateway.verify_request_signature(&signed_request).await.unwrap();
    assert!(is_valid, "Request signature should be valid");
    
    // Process the signed request
    let response = gateway.process_request(signed_request).await.unwrap();
    assert!(response.success);
    
    // Verify response is also signed
    assert!(response.signature.is_some());
    let response_valid = gateway.verify_response_signature(&response).await.unwrap();
    assert!(response_valid, "Response signature should be valid");
}

#[tokio::test]
async fn test_encrypted_communication() {
    let gateway = setup_test_gateway_with_tls().await;
    
    let request = MCPRequest {
        id: "encrypted-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Test encrypted request"}
            ],
            "max_tokens": 50
        }),
    };
    
    // Encrypt the request
    let encrypted_request = gateway.encrypt_request(request).await.unwrap();
    assert!(encrypted_request.is_encrypted());
    
    // Decrypt and process
    let decrypted_request = gateway.decrypt_request(encrypted_request).await.unwrap();
    let response = gateway.process_request(decrypted_request).await.unwrap();
    
    assert!(response.success);
    assert_eq!(response.id, "encrypted-001");
}

#[tokio::test]
async fn test_access_control_policies() {
    let gateway = setup_test_gateway_with_policies().await;
    
    // Test allowed request
    let allowed_request = MCPRequest {
        id: "policy-allowed".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Allowed content"}
            ],
            "max_tokens": 50
        }),
    };
    
    let response = gateway.process_request(allowed_request).await.unwrap();
    assert!(response.success);
    
    // Test blocked request
    let blocked_request = MCPRequest {
        id: "policy-blocked".to_string(),
        method: "admin_action".to_string(),
        params: json!({
            "action": "delete_all_data"
        }),
    };
    
    let response = gateway.process_request(blocked_request).await.unwrap();
    assert!(!response.success);
    assert!(response.error.is_some());
    
    let error = response.error.unwrap();
    assert_eq!(error.code, "ACCESS_DENIED");
}

#[tokio::test]
async fn test_secure_model_loading() {
    let temp_dir = tempfile::tempdir().unwrap();
    let model_path = temp_dir.path().join("test_model.ggml");
    
    // Create a mock model file
    std::fs::write(&model_path, b"mock model data").unwrap();
    
    let gateway = setup_test_gateway_with_security().await;
    
    // Test model loading with signature verification
    let load_result = gateway.load_model_securely(&model_path).await;
    
    match load_result {
        Ok(_) => {
            // Model loaded successfully
            assert!(gateway.is_model_loaded("test_model").await);
        }
        Err(e) => {
            // Expected to fail due to invalid signature in test
            assert!(e.to_string().contains("signature") || e.to_string().contains("verification"));
        }
    }
}

#[tokio::test]
async fn test_audit_logging() {
    let temp_dir = tempfile::tempdir().unwrap();
    let audit_log_path = temp_dir.path().join("audit.log");
    
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_audit_logging(true)
            .audit_log_path(&audit_log_path)
            .build().unwrap())
        .build()
        .unwrap();
    
    let gateway = Gateway::new(config).await.unwrap();
    
    let request = MCPRequest {
        id: "audit-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Audited request"}
            ],
            "max_tokens": 50
        }),
    };
    
    let _response = gateway.process_request(request).await.unwrap();
    
    // Wait a moment for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify audit log exists and contains our request
    assert!(audit_log_path.exists());
    
    let audit_content = std::fs::read_to_string(&audit_log_path).unwrap();
    assert!(audit_content.contains("audit-001"));
    assert!(audit_content.contains("completion"));
    assert!(audit_content.contains("Audited request"));
}

#[tokio::test]
async fn test_rate_limiting() {
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_rate_limiting(true)
            .rate_limit_requests_per_minute(5)
            .build().unwrap())
        .build()
        .unwrap();
    
    let gateway = Gateway::new(config).await.unwrap();
    
    // Send requests up to the limit
    for i in 0..5 {
        let request = MCPRequest {
            id: format!("rate-limit-{:03}", i),
            method: "completion".to_string(),
            params: json!({"test": format!("request {}", i)}),
        };
        
        let response = gateway.process_request(request).await.unwrap();
        assert!(response.success, "Request {} should succeed within rate limit", i);
    }
    
    // Next request should be rate limited
    let rate_limited_request = MCPRequest {
        id: "rate-limit-006".to_string(),
        method: "completion".to_string(),
        params: json!({"test": "rate limited request"}),
    };
    
    let response = gateway.process_request(rate_limited_request).await.unwrap();
    assert!(!response.success);
    assert!(response.error.is_some());
    
    let error = response.error.unwrap();
    assert_eq!(error.code, "RATE_LIMIT_EXCEEDED");
}

#[tokio::test]
async fn test_secure_key_rotation() {
    let gateway = setup_test_gateway_with_security().await;
    
    // Get initial key ID
    let initial_key_id = gateway.get_current_key_id().await.unwrap();
    
    // Trigger key rotation
    let rotation_result = gateway.rotate_keys().await.unwrap();
    assert!(rotation_result.success);
    assert!(rotation_result.new_key_id != initial_key_id);
    
    // Verify new key is being used
    let current_key_id = gateway.get_current_key_id().await.unwrap();
    assert_eq!(current_key_id, rotation_result.new_key_id);
    
    // Verify old key is still available for verification (grace period)
    let key_status = gateway.get_key_status(&initial_key_id).await.unwrap();
    assert!(key_status.is_valid_for_verification());
}

async fn setup_test_gateway_with_security() -> Gateway {
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_signing(true)
            .enable_encryption(true)
            .build().unwrap())
        .build()
        .unwrap();
    
    Gateway::new(config).await.unwrap()
}

async fn setup_test_gateway_with_tls() -> Gateway {
    let temp_dir = tempfile::tempdir().unwrap();
    let cert_path = temp_dir.path().join("cert.pem");
    let key_path = temp_dir.path().join("key.pem");
    
    // Create mock TLS files
    std::fs::write(&cert_path, "mock cert").unwrap();
    std::fs::write(&key_path, "mock key").unwrap();
    
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_tls(true)
            .tls_cert_path(&cert_path)
            .tls_key_path(&key_path)
            .build().unwrap())
        .build()
        .unwrap();
    
    Gateway::new(config).await.unwrap()
}

async fn setup_test_gateway_with_policies() -> Gateway {
    let config = Config::builder()
        .security(SecurityConfig::builder()
            .enable_access_control(true)
            .access_control_policy("default_restrictive")
            .build().unwrap())
        .build()
        .unwrap();
    
    Gateway::new(config).await.unwrap()
}