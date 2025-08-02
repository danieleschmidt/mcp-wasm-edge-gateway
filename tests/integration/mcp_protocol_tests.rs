use super::common::*;
use mcp_wasm_edge_gateway::{Gateway, Config, MCPRequest, MCPResponse};
use serde_json::json;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_mcp_request_response_cycle() {
    let gateway = setup_test_gateway().await;
    
    let request = MCPRequest {
        id: "test-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Hello, world!"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        }),
    };
    
    let response = timeout(Duration::from_secs(5), gateway.process_request(request))
        .await
        .expect("Request should complete within timeout")
        .expect("Request should succeed");
    
    assert_eq!(response.id, "test-001");
    assert!(response.success);
    assert!(response.data.is_some());
}

#[tokio::test]
async fn test_mcp_streaming_request() {
    let gateway = setup_test_gateway().await;
    
    let request = MCPRequest {
        id: "stream-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Generate a long response"}
            ],
            "stream": true,
            "max_tokens": 500
        }),
    };
    
    let mut response_stream = gateway.process_streaming_request(request)
        .await
        .expect("Streaming request should succeed");
    
    let mut chunk_count = 0;
    while let Some(chunk) = response_stream.next().await {
        chunk_count += 1;
        assert!(chunk.is_ok());
    }
    
    assert!(chunk_count > 0, "Should receive at least one streaming chunk");
}

#[tokio::test]
async fn test_mcp_tool_execution() {
    let gateway = setup_test_gateway().await;
    
    let request = MCPRequest {
        id: "tool-001".to_string(),
        method: "tool_execution".to_string(),
        params: json!({
            "tool": "calculator",
            "action": "add",
            "arguments": {
                "a": 5,
                "b": 3
            }
        }),
    };
    
    let response = gateway.process_request(request)
        .await
        .expect("Tool execution should succeed");
    
    assert_eq!(response.id, "tool-001");
    assert!(response.success);
    
    let result = response.data.unwrap();
    assert_eq!(result["result"], 8);
}

#[tokio::test]
async fn test_mcp_protocol_validation() {
    let gateway = setup_test_gateway().await;
    
    // Test invalid method
    let invalid_request = MCPRequest {
        id: "invalid-001".to_string(),
        method: "invalid_method".to_string(),
        params: json!({}),
    };
    
    let response = gateway.process_request(invalid_request)
        .await
        .expect("Should return error response");
    
    assert!(!response.success);
    assert!(response.error.is_some());
    assert!(response.error.unwrap().code == "METHOD_NOT_FOUND");
}

#[tokio::test]
async fn test_mcp_concurrent_requests() {
    let gateway = setup_test_gateway().await;
    let gateway = std::sync::Arc::new(gateway);
    
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let gateway_clone = gateway.clone();
        let handle = tokio::spawn(async move {
            let request = MCPRequest {
                id: format!("concurrent-{:03}", i),
                method: "completion".to_string(),
                params: json!({
                    "messages": [
                        {"role": "user", "content": format!("Request {}", i)}
                    ],
                    "max_tokens": 50
                }),
            };
            
            gateway_clone.process_request(request).await
        });
        handles.push(handle);
    }
    
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("All concurrent requests should complete");
    
    assert_eq!(results.len(), 10);
    for (i, result) in results.iter().enumerate() {
        let response = result.as_ref().unwrap();
        assert_eq!(response.id, format!("concurrent-{:03}", i));
        assert!(response.success);
    }
}

#[tokio::test]
async fn test_mcp_protocol_error_handling() {
    let gateway = setup_test_gateway().await;
    
    // Test malformed request
    let malformed_request = MCPRequest {
        id: "malformed-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "invalid_field": "this should cause an error"
        }),
    };
    
    let response = gateway.process_request(malformed_request)
        .await
        .expect("Should return error response");
    
    assert!(!response.success);
    assert!(response.error.is_some());
    
    let error = response.error.unwrap();
    assert!(error.code == "INVALID_PARAMS" || error.code == "VALIDATION_ERROR");
    assert!(!error.message.is_empty());
}