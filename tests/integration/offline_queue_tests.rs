use super::common::*;
use mcp_wasm_edge_gateway::{Gateway, Config, MCPRequest, OfflineQueue, QueueStatus};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_offline_queue_basic_operations() {
    let mut gateway = setup_test_gateway().await;
    gateway.set_offline_mode(true).await;
    
    let request = MCPRequest {
        id: "offline-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "This should be queued"}
            ],
            "max_tokens": 100
        }),
    };
    
    // Queue the request
    let queue_response = gateway.process_request(request.clone())
        .await
        .expect("Request should be queued");
    
    assert!(queue_response.success);
    assert_eq!(queue_response.id, "offline-001");
    assert!(queue_response.data.is_some());
    assert_eq!(queue_response.data.unwrap()["status"], "queued");
    
    // Check queue status
    let queue_status = gateway.get_queue_status().await;
    assert_eq!(queue_status.pending_count, 1);
    assert_eq!(queue_status.failed_count, 0);
}

#[tokio::test]
async fn test_offline_queue_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config::builder()
        .offline_queue_path(temp_dir.path().join("queue.db"))
        .offline_queue_size(1000)
        .build()
        .unwrap();
    
    let mut gateway = Gateway::new(config.clone()).await.unwrap();
    gateway.set_offline_mode(true).await;
    
    // Add requests to queue
    for i in 0..5 {
        let request = MCPRequest {
            id: format!("persist-{:03}", i),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": format!("Persistent request {}", i)}
                ],
                "max_tokens": 50
            }),
        };
        
        gateway.process_request(request).await.unwrap();
    }
    
    // Verify queue has 5 items
    let status = gateway.get_queue_status().await;
    assert_eq!(status.pending_count, 5);
    
    // Restart gateway with same config
    drop(gateway);
    let gateway = Gateway::new(config).await.unwrap();
    
    // Verify queue still has 5 items after restart
    let status = gateway.get_queue_status().await;
    assert_eq!(status.pending_count, 5);
}

#[tokio::test]
async fn test_offline_to_online_sync() {
    let mut gateway = setup_test_gateway().await;
    gateway.set_offline_mode(true).await;
    
    // Queue several requests while offline
    for i in 0..3 {
        let request = MCPRequest {
            id: format!("sync-{:03}", i),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": format!("Sync request {}", i)}
                ],
                "max_tokens": 50
            }),
        };
        
        gateway.process_request(request).await.unwrap();
    }
    
    // Verify all requests are queued
    let status = gateway.get_queue_status().await;
    assert_eq!(status.pending_count, 3);
    
    // Go back online and trigger sync
    gateway.set_offline_mode(false).await;
    let sync_result = gateway.sync_offline_queue().await.unwrap();
    
    assert_eq!(sync_result.processed_count, 3);
    assert_eq!(sync_result.successful_count, 3);
    assert_eq!(sync_result.failed_count, 0);
    
    // Verify queue is now empty
    let status = gateway.get_queue_status().await;
    assert_eq!(status.pending_count, 0);
}

#[tokio::test]
async fn test_queue_size_limit() {
    let config = Config::builder()
        .offline_queue_size(3) // Small queue for testing
        .build()
        .unwrap();
    
    let mut gateway = Gateway::new(config).await.unwrap();
    gateway.set_offline_mode(true).await;
    
    // Fill the queue to capacity
    for i in 0..3 {
        let request = MCPRequest {
            id: format!("limit-{:03}", i),
            method: "completion".to_string(),
            params: json!({"test": format!("request {}", i)}),
        };
        
        let response = gateway.process_request(request).await.unwrap();
        assert!(response.success);
    }
    
    // Try to add one more request (should fail or replace oldest)
    let overflow_request = MCPRequest {
        id: "overflow-001".to_string(),
        method: "completion".to_string(),
        params: json!({"test": "overflow request"}),
    };
    
    let response = gateway.process_request(overflow_request).await;
    
    // Should either fail or succeed with LRU eviction
    if response.is_ok() {
        // If it succeeds, queue should still be at capacity
        let status = gateway.get_queue_status().await;
        assert_eq!(status.pending_count, 3);
    } else {
        // If it fails, queue should be at capacity
        let status = gateway.get_queue_status().await;
        assert_eq!(status.pending_count, 3);
    }
}

#[tokio::test]
async fn test_queue_retry_mechanism() {
    let mut gateway = setup_test_gateway().await;
    gateway.set_offline_mode(true).await;
    
    // Queue a request that will fail during sync
    let failing_request = MCPRequest {
        id: "retry-001".to_string(),
        method: "invalid_method_for_retry".to_string(),
        params: json!({}),
    };
    
    gateway.process_request(failing_request).await.unwrap();
    
    // Go online and attempt sync
    gateway.set_offline_mode(false).await;
    
    // Mock network failure for the first sync attempt
    gateway.set_network_available(false).await;
    let sync_result = gateway.sync_offline_queue().await.unwrap();
    assert_eq!(sync_result.failed_count, 1);
    
    // Restore network and retry
    gateway.set_network_available(true).await;
    sleep(Duration::from_millis(100)).await; // Wait for retry backoff
    
    let retry_result = gateway.sync_offline_queue().await.unwrap();
    assert!(retry_result.processed_count > 0);
}

#[tokio::test]
async fn test_queue_compression() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config::builder()
        .offline_queue_path(temp_dir.path().join("compressed_queue.db"))
        .offline_queue_compression("zstd")
        .build()
        .unwrap();
    
    let mut gateway = Gateway::new(config).await.unwrap();
    gateway.set_offline_mode(true).await;
    
    // Create a request with large payload
    let large_content = "x".repeat(10000); // 10KB of data
    let request = MCPRequest {
        id: "compression-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": large_content}
            ],
            "max_tokens": 100
        }),
    };
    
    gateway.process_request(request).await.unwrap();
    
    // Verify the queue file exists and check its size
    let queue_file = temp_dir.path().join("compressed_queue.db");
    assert!(queue_file.exists());
    
    let metadata = std::fs::metadata(&queue_file).unwrap();
    // Compressed size should be significantly smaller than 10KB + overhead
    assert!(metadata.len() < 5000, "Compressed queue should be smaller than uncompressed data");
}