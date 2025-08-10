//! Comprehensive performance benchmarks for the MCP WASM Edge Gateway
//!
//! These benchmarks test:
//! - Request processing latency and throughput
//! - Memory usage and cache performance
//! - Concurrent request handling
//! - Edge optimization effectiveness
//! - Model ensemble performance
//! - Security validation overhead

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mcp_common::{Config, MCPRequest};
use mcp_gateway::Gateway;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

// Benchmark single request processing
fn benchmark_single_request_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let request = MCPRequest {
        id: Uuid::new_v4().to_string(),
        device_id: "benchmark-device".to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "prompt": "Benchmark test request",
            "max_tokens": 100,
        }).as_object().unwrap().clone(),
        context: None,
    };

    c.bench_function("single_request_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let response = gateway.process_request(black_box(&request)).await;
            black_box(response)
        });
    });
}

// Benchmark different request types
fn benchmark_request_types(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let request_types = vec![
        ("completion", "Generate a short story about a robot"),
        ("embedding", "Convert this text to vector representation"),
        ("classification", "Classify this sentiment: I love this product!"),
        ("summarization", "Summarize: The quick brown fox jumps over the lazy dog. This is a test of text summarization capabilities."),
        ("chat", "Hello, how are you today?"),
    ];

    let mut group = c.benchmark_group("request_types");
    
    for (method, prompt) in request_types {
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            device_id: "benchmark-device".to_string(),
            method: method.to_string(),
            params: serde_json::json!({
                "prompt": prompt,
                "max_tokens": 100,
            }).as_object().unwrap().clone(),
            context: None,
        };

        group.bench_with_input(BenchmarkId::new("method", method), &request, |b, req| {
            b.to_async(&rt).iter(|| async {
                let response = gateway.process_request(black_box(req)).await;
                black_box(response)
            });
        });
    }
    group.finish();
}

// Benchmark concurrent request handling
fn benchmark_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let concurrency_levels = vec![1, 5, 10, 25, 50];
    let mut group = c.benchmark_group("concurrent_requests");
    
    // Set longer measurement time for concurrent tests
    group.measurement_time(Duration::from_secs(20));

    for &concurrency in &concurrency_levels {
        group.bench_with_input(BenchmarkId::new("concurrency", concurrency), &concurrency, |b, &conc| {
            b.to_async(&rt).iter(|| async {
                let mut handles = Vec::new();
                
                for i in 0..conc {
                    let gateway_clone = gateway.clone();
                    let request = MCPRequest {
                        id: format!("concurrent-{}", i),
                        device_id: format!("device-{}", i % 5), // Use multiple devices
                        method: "completion".to_string(),
                        params: serde_json::json!({
                            "prompt": format!("Concurrent request {}", i),
                            "max_tokens": 50,
                        }).as_object().unwrap().clone(),
                        context: None,
                    };

                    let handle = tokio::spawn(async move {
                        gateway_clone.process_request(&request).await
                    });
                    
                    handles.push(handle);
                }

                let results = futures::future::join_all(handles).await;
                black_box(results)
            });
        });
    }
    group.finish();
}

// Benchmark request size scaling
fn benchmark_request_size_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let request_sizes = vec![100, 500, 1000, 2000, 5000]; // Character counts
    let mut group = c.benchmark_group("request_size_scaling");

    for &size in &request_sizes {
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            device_id: "benchmark-device".to_string(),
            method: "completion".to_string(),
            params: serde_json::json!({
                "prompt": "A".repeat(size),
                "max_tokens": 100,
            }).as_object().unwrap().clone(),
            context: None,
        };

        group.bench_with_input(BenchmarkId::new("size_chars", size), &request, |b, req| {
            b.to_async(&rt).iter(|| async {
                let response = gateway.process_request(black_box(req)).await;
                black_box(response)
            });
        });
    }
    group.finish();
}

// Benchmark cache performance
fn benchmark_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    // Warm up cache with some requests
    let warmup_request = MCPRequest {
        id: Uuid::new_v4().to_string(),
        device_id: "cache-benchmark-device".to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "prompt": "Cache warmup request",
            "model": "benchmark-model",
        }).as_object().unwrap().clone(),
        context: None,
    };

    // Warmup
    rt.block_on(async {
        for _ in 0..10 {
            let _ = gateway.process_request(&warmup_request).await;
        }
    });

    let mut group = c.benchmark_group("cache_performance");

    // Benchmark cache hit scenario
    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let response = gateway.process_request(black_box(&warmup_request)).await;
            black_box(response)
        });
    });

    // Benchmark cache miss scenario
    group.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let unique_request = MCPRequest {
                id: Uuid::new_v4().to_string(),
                device_id: "cache-benchmark-device".to_string(),
                method: "completion".to_string(),
                params: serde_json::json!({
                    "prompt": format!("Unique request {}", Uuid::new_v4()),
                    "model": format!("unique-model-{}", Uuid::new_v4()),
                }).as_object().unwrap().clone(),
                context: None,
            };
            
            let response = gateway.process_request(black_box(&unique_request)).await;
            black_box(response)
        });
    });

    group.finish();
}

// Benchmark security validation overhead
fn benchmark_security_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let security_manager = rt.block_on(async {
        mcp_security::StandardSecurityManager::new(config).await
            .expect("Failed to create security manager")
    });

    let request = MCPRequest {
        id: Uuid::new_v4().to_string(),
        device_id: "security-benchmark-device".to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "prompt": "Security validation benchmark request",
        }).as_object().unwrap().clone(),
        context: None,
    };

    let mut group = c.benchmark_group("security_overhead");

    group.bench_function("request_validation", |b| {
        b.to_async(&rt).iter(|| async {
            let result = security_manager.validate_request(black_box(&request)).await;
            black_box(result)
        });
    });

    let test_data = b"Security encryption benchmark data that should be reasonably sized for testing";
    
    group.bench_function("data_encryption", |b| {
        b.to_async(&rt).iter(|| async {
            let result = security_manager.encrypt_data(black_box(test_data)).await;
            black_box(result)
        });
    });

    // Benchmark decryption (need to encrypt first)
    let encrypted_data = rt.block_on(async {
        security_manager.encrypt_data(test_data).await
            .expect("Failed to encrypt test data")
    });

    group.bench_function("data_decryption", |b| {
        b.to_async(&rt).iter(|| async {
            let result = security_manager.decrypt_data(black_box(&encrypted_data)).await;
            black_box(result)
        });
    });

    group.finish();
}

// Benchmark ensemble strategies
fn benchmark_ensemble_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let strategies = vec![
        "fastest_first",
        "weighted_voting",
        "task_specialized",
        "complexity_based",
    ];

    let mut group = c.benchmark_group("ensemble_strategies");
    
    // Longer measurement time for ensemble tests
    group.measurement_time(Duration::from_secs(15));

    for strategy in strategies {
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            device_id: format!("ensemble-{}-device", strategy),
            method: "reasoning".to_string(),
            params: serde_json::json!({
                "query": "What are the benefits of ensemble methods?",
                "ensemble_strategy": strategy,
            }).as_object().unwrap().clone(),
            context: None,
        };

        group.bench_with_input(BenchmarkId::new("strategy", strategy), &request, |b, req| {
            b.to_async(&rt).iter(|| async {
                let response = gateway.process_request(black_box(req)).await;
                black_box(response)
            });
        });
    }
    group.finish();
}

// Benchmark memory usage patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let mut group = c.benchmark_group("memory_usage");

    // Test memory usage with increasing load
    let load_levels = vec![1, 10, 50, 100];

    for &load in &load_levels {
        group.bench_with_input(BenchmarkId::new("concurrent_load", load), &load, |b, &level| {
            b.to_async(&rt).iter(|| async {
                let mut handles = Vec::new();
                
                for i in 0..level {
                    let gateway_clone = gateway.clone();
                    let request = MCPRequest {
                        id: format!("memory-test-{}", i),
                        device_id: format!("memory-device-{}", i % 10),
                        method: "completion".to_string(),
                        params: serde_json::json!({
                            "prompt": format!("Memory usage test request {}", i),
                            "max_tokens": 200,
                        }).as_object().unwrap().clone(),
                        context: None,
                    };

                    let handle = tokio::spawn(async move {
                        gateway_clone.process_request(&request).await
                    });
                    
                    handles.push(handle);
                }

                let results = futures::future::join_all(handles).await;
                black_box(results)
            });
        });
    }
    group.finish();
}

// Benchmark edge optimization features
fn benchmark_edge_optimizations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let mut group = c.benchmark_group("edge_optimizations");

    // Test power-aware processing
    let power_modes = vec![
        ("high_performance", "Process with maximum performance"),
        ("balanced", "Balance performance and power consumption"),
        ("power_saver", "Minimize power consumption"),
        ("ultra_low_power", "Maximum power savings"),
    ];

    for (mode, description) in power_modes {
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            device_id: format!("power-{}-device", mode),
            method: "completion".to_string(),
            params: serde_json::json!({
                "prompt": description,
                "power_mode": mode,
                "max_tokens": 100,
            }).as_object().unwrap().clone(),
            context: None,
        };

        group.bench_with_input(BenchmarkId::new("power_mode", mode), &request, |b, req| {
            b.to_async(&rt).iter(|| async {
                let response = gateway.process_request(black_box(req)).await;
                black_box(response)
            });
        });
    }

    // Test compression effectiveness
    let compression_levels = vec!["none", "lz4", "zstd", "adaptive"];

    for compression in compression_levels {
        let request = MCPRequest {
            id: Uuid::new_v4().to_string(),
            device_id: format!("compression-{}-device", compression),
            method: "completion".to_string(),
            params: serde_json::json!({
                "prompt": "Test compression performance with this moderately long text that should benefit from compression algorithms",
                "compression": compression,
                "max_tokens": 200,
            }).as_object().unwrap().clone(),
            context: None,
        };

        group.bench_with_input(BenchmarkId::new("compression", compression), &request, |b, req| {
            b.to_async(&rt).iter(|| async {
                let response = gateway.process_request(black_box(req)).await;
                black_box(response)
            });
        });
    }

    group.finish();
}

// Benchmark latency percentiles
fn benchmark_latency_distribution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let request = MCPRequest {
        id: Uuid::new_v4().to_string(),
        device_id: "latency-benchmark-device".to_string(),
        method: "completion".to_string(),
        params: serde_json::json!({
            "prompt": "Latency distribution benchmark request",
            "max_tokens": 100,
        }).as_object().unwrap().clone(),
        context: None,
    };

    let mut group = c.benchmark_group("latency_distribution");
    
    // Configure for latency measurement
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(1000);

    group.bench_function("request_latency_p95", |b| {
        b.to_async(&rt).iter(|| async {
            let response = gateway.process_request(black_box(&request)).await;
            black_box(response)
        });
    });

    group.finish();
}

// Benchmark throughput under sustained load
fn benchmark_sustained_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(Config::default());
    let gateway = rt.block_on(async {
        Gateway::new(config).await.expect("Failed to create gateway")
    });

    let mut group = c.benchmark_group("sustained_throughput");
    
    // Longer measurement for sustained load
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("requests_per_second", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate sustained load with multiple concurrent streams
            let mut handles = Vec::new();
            
            for stream in 0..10 {
                let gateway_clone = gateway.clone();
                let handle = tokio::spawn(async move {
                    let request = MCPRequest {
                        id: format!("throughput-stream-{}", stream),
                        device_id: format!("throughput-device-{}", stream),
                        method: "completion".to_string(),
                        params: serde_json::json!({
                            "prompt": format!("Throughput test request from stream {}", stream),
                            "max_tokens": 50,
                        }).as_object().unwrap().clone(),
                        context: None,
                    };
                    
                    gateway_clone.process_request(&request).await
                });
                
                handles.push(handle);
            }

            let results = futures::future::join_all(handles).await;
            black_box(results)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_request_processing,
    benchmark_request_types,
    benchmark_concurrent_requests,
    benchmark_request_size_scaling,
    benchmark_cache_performance,
    benchmark_security_overhead,
    benchmark_ensemble_strategies,
    benchmark_memory_usage,
    benchmark_edge_optimizations,
    benchmark_latency_distribution,
    benchmark_sustained_throughput
);

criterion_main!(benches);