use super::common::*;
use mcp_wasm_edge_gateway::{Gateway, Config, MCPRequest, PerformanceMetrics};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[tokio::test]
async fn test_latency_requirements() {
    let gateway = setup_test_gateway().await;
    
    let request = MCPRequest {
        id: "latency-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Quick response test"}
            ],
            "max_tokens": 50
        }),
    };
    
    let start_time = Instant::now();
    let response = timeout(Duration::from_millis(200), gateway.process_request(request))
        .await
        .expect("Request should complete within 200ms latency requirement")
        .expect("Request should succeed");
    
    let duration = start_time.elapsed();
    
    assert!(response.success);
    assert!(duration.as_millis() < 200, "Latency requirement: p99 < 200ms, actual: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_throughput_capacity() {
    let gateway = setup_test_gateway().await;
    let gateway = std::sync::Arc::new(gateway);
    
    let request_count = 100;
    let concurrent_limit = 10;
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Send requests in batches to test throughput
    for batch in 0..(request_count / concurrent_limit) {
        let mut batch_handles = Vec::new();
        
        for i in 0..concurrent_limit {
            let gateway_clone = gateway.clone();
            let request_id = batch * concurrent_limit + i;
            
            let handle = tokio::spawn(async move {
                let request = MCPRequest {
                    id: format!("throughput-{:03}", request_id),
                    method: "completion".to_string(),
                    params: json!({
                        "messages": [
                            {"role": "user", "content": format!("Throughput test {}", request_id)}
                        ],
                        "max_tokens": 25
                    }),
                };
                
                gateway_clone.process_request(request).await
            });
            batch_handles.push(handle);
        }
        
        // Wait for this batch to complete
        let _batch_results: Vec<_> = futures::future::join_all(batch_handles).await;
    }
    
    let total_duration = start_time.elapsed();
    let throughput = request_count as f64 / total_duration.as_secs_f64();
    
    // Requirement: > 50 requests/second on standard edge hardware
    assert!(throughput > 50.0, 
        "Throughput requirement: > 50 req/s, actual: {:.2} req/s", throughput);
}

#[tokio::test]
async fn test_memory_usage_limits() {
    let config = Config::builder()
        .max_memory_mb(512)
        .enable_memory_monitoring(true)
        .build()
        .unwrap();
    
    let gateway = Gateway::new(config).await.unwrap();
    
    // Get initial memory usage
    let initial_memory = gateway.get_memory_usage().await.unwrap();
    
    // Process multiple requests to test memory usage
    for i in 0..50 {
        let request = MCPRequest {
            id: format!("memory-{:03}", i),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": format!("Memory test request {} with some content to use memory", i)}
                ],
                "max_tokens": 100
            }),
        };
        
        let _response = gateway.process_request(request).await.unwrap();
        
        // Check memory usage periodically
        if i % 10 == 0 {
            let current_memory = gateway.get_memory_usage().await.unwrap();
            assert!(current_memory.total_mb < 512.0, 
                "Memory usage should stay under 512MB limit, current: {:.2}MB", current_memory.total_mb);
        }
    }
    
    // Final memory check
    let final_memory = gateway.get_memory_usage().await.unwrap();
    assert!(final_memory.total_mb < 512.0);
    
    // Memory should not have grown excessively
    let memory_growth = final_memory.total_mb - initial_memory.total_mb;
    assert!(memory_growth < 100.0, 
        "Memory growth should be reasonable, growth: {:.2}MB", memory_growth);
}

#[tokio::test]
async fn test_cpu_utilization() {
    let gateway = setup_test_gateway().await;
    
    // Monitor CPU usage during load
    let cpu_monitor = gateway.start_cpu_monitoring().await.unwrap();
    
    // Generate sustained load
    let load_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    while start_time.elapsed() < load_duration {
        let request = MCPRequest {
            id: format!("cpu-{}", start_time.elapsed().as_millis()),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": "CPU load test"}
                ],
                "max_tokens": 50
            }),
        };
        
        let _ = gateway.process_request(request).await;
        tokio::task::yield_now().await; // Allow other tasks to run
    }
    
    let cpu_stats = cpu_monitor.stop().await.unwrap();
    
    // Requirement: CPU utilization < 80% under normal load
    assert!(cpu_stats.average_utilization < 80.0,
        "CPU utilization should be under 80%, actual: {:.2}%", cpu_stats.average_utilization);
}

#[tokio::test]
async fn test_concurrent_request_handling() {
    let gateway = setup_test_gateway().await;
    let gateway = std::sync::Arc::new(gateway);
    
    let concurrent_requests = 20;
    let mut handles = Vec::new();
    
    let start_time = Instant::now();
    
    // Launch concurrent requests
    for i in 0..concurrent_requests {
        let gateway_clone = gateway.clone();
        let handle = tokio::spawn(async move {
            let request = MCPRequest {
                id: format!("concurrent-perf-{:03}", i),
                method: "completion".to_string(),
                params: json!({
                    "messages": [
                        {"role": "user", "content": format!("Concurrent performance test {}", i)}
                    ],
                    "max_tokens": 75
                }),
            };
            
            let request_start = Instant::now();
            let result = gateway_clone.process_request(request).await;
            let request_duration = request_start.elapsed();
            
            (result, request_duration)
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    let total_duration = start_time.elapsed();
    
    // Analyze results
    let mut successful_requests = 0;
    let mut total_latency = Duration::new(0, 0);
    let mut max_latency = Duration::new(0, 0);
    
    for result in results {
        let (response_result, duration) = result.unwrap();
        if response_result.is_ok() && response_result.unwrap().success {
            successful_requests += 1;
            total_latency += duration;
            if duration > max_latency {
                max_latency = duration;
            }
        }
    }
    
    // All requests should succeed
    assert_eq!(successful_requests, concurrent_requests);
    
    // Average latency should be reasonable
    let avg_latency = total_latency / concurrent_requests as u32;
    assert!(avg_latency.as_millis() < 500, 
        "Average latency under concurrent load should be < 500ms, actual: {}ms", 
        avg_latency.as_millis());
    
    // P99 latency (approximated by max in this small sample)
    assert!(max_latency.as_millis() < 1000,
        "P99 latency should be < 1000ms under concurrent load, actual: {}ms", 
        max_latency.as_millis());
}

#[tokio::test]
async fn test_model_loading_performance() {
    let gateway = setup_test_gateway().await;
    
    let model_names = vec!["tiny-model", "small-model", "medium-model"];
    
    for model_name in model_names {
        let start_time = Instant::now();
        
        let load_result = gateway.load_model(model_name).await;
        let load_duration = start_time.elapsed();
        
        // Model loading should complete within reasonable time
        assert!(load_duration.as_secs() < 10, 
            "Model {} loading should complete within 10 seconds, actual: {}s", 
            model_name, load_duration.as_secs());
        
        if load_result.is_ok() {
            // First inference after model loading should be fast
            let request = MCPRequest {
                id: format!("model-load-test-{}", model_name),
                method: "completion".to_string(),
                params: json!({
                    "model": model_name,
                    "messages": [
                        {"role": "user", "content": "First inference test"}
                    ],
                    "max_tokens": 25
                }),
            };
            
            let inference_start = Instant::now();
            let response = gateway.process_request(request).await.unwrap();
            let inference_duration = inference_start.elapsed();
            
            assert!(response.success);
            assert!(inference_duration.as_millis() < 1000,
                "First inference after model loading should be < 1000ms, actual: {}ms", 
                inference_duration.as_millis());
        }
    }
}

#[tokio::test]
async fn test_cache_performance() {
    let gateway = setup_test_gateway().await;
    
    let request = MCPRequest {
        id: "cache-test-001".to_string(),
        method: "completion".to_string(),
        params: json!({
            "messages": [
                {"role": "user", "content": "Cache performance test"}
            ],
            "max_tokens": 50,
            "cache_key": "cache-perf-test"
        }),
    };
    
    // First request (cache miss)
    let start_time = Instant::now();
    let response1 = gateway.process_request(request.clone()).await.unwrap();
    let cache_miss_duration = start_time.elapsed();
    
    assert!(response1.success);
    
    // Second identical request (should be cache hit)
    let start_time = Instant::now();
    let response2 = gateway.process_request(request.clone()).await.unwrap();
    let cache_hit_duration = start_time.elapsed();
    
    assert!(response2.success);
    
    // Cache hit should be significantly faster
    assert!(cache_hit_duration < cache_miss_duration / 2,
        "Cache hit ({:?}) should be much faster than cache miss ({:?})", 
        cache_hit_duration, cache_miss_duration);
    
    // Cache hit should be very fast
    assert!(cache_hit_duration.as_millis() < 50,
        "Cache hit should be < 50ms, actual: {}ms", cache_hit_duration.as_millis());
}

#[tokio::test] 
async fn test_resource_cleanup() {
    let gateway = setup_test_gateway().await;
    
    // Get initial resource usage
    let initial_resources = gateway.get_resource_usage().await.unwrap();
    
    // Process many requests to potentially create resource leaks
    for i in 0..100 {
        let request = MCPRequest {
            id: format!("cleanup-{:03}", i),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": format!("Resource cleanup test {}", i)}
                ],
                "max_tokens": 30
            }),
        };
        
        let _response = gateway.process_request(request).await.unwrap();
    }
    
    // Force garbage collection and cleanup
    gateway.force_cleanup().await.unwrap();
    
    // Wait for cleanup to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Check final resource usage
    let final_resources = gateway.get_resource_usage().await.unwrap();
    
    // Memory should not have leaked significantly
    let memory_growth = final_resources.memory_mb - initial_resources.memory_mb;
    assert!(memory_growth < 50.0,
        "Memory growth after cleanup should be minimal, growth: {:.2}MB", memory_growth);
    
    // File handles should not have leaked
    assert!(final_resources.file_handles <= initial_resources.file_handles + 5,
        "File handle count should not grow significantly, initial: {}, final: {}", 
        initial_resources.file_handles, final_resources.file_handles);
}