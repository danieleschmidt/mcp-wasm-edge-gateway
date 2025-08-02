use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mcp_wasm_edge_gateway::{Gateway, Config, MCPRequest};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

fn benchmark_request_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let gateway = rt.block_on(async {
        let config = Config::builder()
            .max_memory_mb(256)
            .build()
            .unwrap();
        Gateway::new(config).await.unwrap()
    });
    
    let mut group = c.benchmark_group("request_processing");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark different request sizes
    for content_size in [100, 500, 1000, 5000].iter() {
        let content = "x".repeat(*content_size);
        let request = MCPRequest {
            id: "bench-001".to_string(),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": content}
                ],
                "max_tokens": 50
            }),
        };
        
        group.throughput(Throughput::Bytes(*content_size as u64));
        group.bench_with_input(
            BenchmarkId::new("single_request", content_size),
            content_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let response = gateway.process_request(black_box(request.clone())).await.unwrap();
                    black_box(response)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let gateway = rt.block_on(async {
        let config = Config::builder()
            .max_memory_mb(512)
            .build()
            .unwrap();
        std::sync::Arc::new(Gateway::new(config).await.unwrap())
    });
    
    let mut group = c.benchmark_group("concurrent_requests");
    group.measurement_time(Duration::from_secs(15));
    
    for concurrency in [1, 5, 10, 20].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let gateway = gateway.clone();
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrency {
                        let gateway_clone = gateway.clone();
                        let handle = tokio::spawn(async move {
                            let request = MCPRequest {
                                id: format!("concurrent-bench-{}", i),
                                method: "completion".to_string(),
                                params: json!({
                                    "messages": [
                                        {"role": "user", "content": format!("Concurrent test {}", i)}
                                    ],
                                    "max_tokens": 25
                                }),
                            };
                            gateway_clone.process_request(request).await.unwrap()
                        });
                        handles.push(handle);
                    }
                    
                    let results = futures::future::join_all(handles).await;
                    black_box(results)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_model_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let gateway = rt.block_on(async {
        let config = Config::builder()
            .model_cache_size(3)
            .build()
            .unwrap();
        Gateway::new(config).await.unwrap()
    });
    
    let mut group = c.benchmark_group("model_operations");
    
    // Benchmark model loading
    group.bench_function("model_loading", |b| {
        b.to_async(&rt).iter(|| async {
            let result = gateway.load_model(black_box("test-model")).await;
            black_box(result)
        });
    });
    
    // Benchmark model switching
    group.bench_function("model_switching", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = gateway.switch_model(black_box("model-a")).await;
            let _ = gateway.switch_model(black_box("model-b")).await;
        });
    });
    
    group.finish();
}

fn benchmark_queue_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut gateway = rt.block_on(async {
        let config = Config::builder()
            .offline_queue_size(10000)
            .build()
            .unwrap();
        Gateway::new(config).await.unwrap()
    });
    
    rt.block_on(async {
        gateway.set_offline_mode(true).await;
    });
    
    let mut group = c.benchmark_group("queue_operations");
    
    // Benchmark queue insertion
    group.bench_function("queue_insertion", |b| {
        b.to_async(&rt).iter(|| async {
            let request = MCPRequest {
                id: format!("queue-bench-{}", rand::random::<u32>()),
                method: "completion".to_string(),
                params: json!({
                    "messages": [
                        {"role": "user", "content": "Queue benchmark"}
                    ],
                    "max_tokens": 25
                }),
            };
            
            let response = gateway.process_request(black_box(request)).await.unwrap();
            black_box(response)
        });
    });
    
    // Benchmark queue sync
    group.bench_function("queue_sync", |b| {
        b.to_async(&rt).iter(|| async {
            let result = gateway.sync_offline_queue().await;
            black_box(result)
        });
    });
    
    group.finish();
}

fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    // Create test data of different sizes
    for size in [100, 1000, 10000].iter() {
        let content = "x".repeat(*size);
        let request = MCPRequest {
            id: "serialization-bench".to_string(),
            method: "completion".to_string(),
            params: json!({
                "messages": [
                    {"role": "user", "content": content}
                ],
                "max_tokens": 100
            }),
        };
        
        group.throughput(Throughput::Bytes(*size as u64));
        
        // Benchmark JSON serialization
        group.bench_with_input(
            BenchmarkId::new("json_serialize", size),
            &request,
            |b, request| {
                b.iter(|| {
                    let serialized = serde_json::to_string(black_box(request)).unwrap();
                    black_box(serialized)
                });
            },
        );
        
        // Benchmark MessagePack serialization
        group.bench_with_input(
            BenchmarkId::new("msgpack_serialize", size),
            &request,
            |b, request| {
                b.iter(|| {
                    let serialized = rmp_serde::to_vec(black_box(request)).unwrap();
                    black_box(serialized)
                });
            },
        );
        
        // Benchmark JSON deserialization
        let json_data = serde_json::to_string(&request).unwrap();
        group.bench_with_input(
            BenchmarkId::new("json_deserialize", size),
            &json_data,
            |b, data| {
                b.iter(|| {
                    let deserialized: MCPRequest = serde_json::from_str(black_box(data)).unwrap();
                    black_box(deserialized)
                });
            },
        );
        
        // Benchmark MessagePack deserialization
        let msgpack_data = rmp_serde::to_vec(&request).unwrap();
        group.bench_with_input(
            BenchmarkId::new("msgpack_deserialize", size),
            &msgpack_data,
            |b, data| {
                b.iter(|| {
                    let deserialized: MCPRequest = rmp_serde::from_slice(black_box(data)).unwrap();
                    black_box(deserialized)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    for size in [1000, 10000, 100000].iter() {
        let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(*size / 50);
        let data_bytes = data.as_bytes();
        
        group.throughput(Throughput::Bytes(data_bytes.len() as u64));
        
        // Benchmark LZ4 compression
        group.bench_with_input(
            BenchmarkId::new("lz4_compress", size),
            &data_bytes,
            |b, data| {
                b.iter(|| {
                    let compressed = lz4::block::compress(black_box(data), None, true).unwrap();
                    black_box(compressed)
                });
            },
        );
        
        // Benchmark Zstd compression
        group.bench_with_input(
            BenchmarkId::new("zstd_compress", size),
            &data_bytes,
            |b, data| {
                b.iter(|| {
                    let compressed = zstd::bulk::compress(black_box(data), 3).unwrap();
                    black_box(compressed)
                });
            },
        );
        
        // Benchmark LZ4 decompression
        let lz4_compressed = lz4::block::compress(data_bytes, None, true).unwrap();
        group.bench_with_input(
            BenchmarkId::new("lz4_decompress", size),
            &lz4_compressed,
            |b, compressed| {
                b.iter(|| {
                    let decompressed = lz4::block::decompress(black_box(compressed), Some(data_bytes.len() as i32)).unwrap();
                    black_box(decompressed)
                });
            },
        );
        
        // Benchmark Zstd decompression
        let zstd_compressed = zstd::bulk::compress(data_bytes, 3).unwrap();
        group.bench_with_input(
            BenchmarkId::new("zstd_decompress", size),
            &zstd_compressed,
            |b, compressed| {
                b.iter(|| {
                    let decompressed = zstd::bulk::decompress(black_box(compressed), data_bytes.len()).unwrap();
                    black_box(decompressed)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    // Benchmark Vec allocation patterns
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("vec_with_capacity", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(size);
                    for i in 0..size {
                        vec.push(black_box(i));
                    }
                    black_box(vec)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("vec_push_grow", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut vec = Vec::new();
                    for i in 0..size {
                        vec.push(black_box(i));
                    }
                    black_box(vec)
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_request_processing,
    benchmark_concurrent_requests,
    benchmark_model_operations,
    benchmark_queue_operations,
    benchmark_serialization,
    benchmark_compression,
    benchmark_memory_allocation
);
criterion_main!(benches);