// Benchmark suite for MCP WASM Edge Gateway

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;

// Import test utilities
mod common;
use common::*;

fn benchmark_request_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("simple_request", |b| {
        let gateway = rt.block_on(setup_test_gateway()).unwrap();
        
        b.to_async(&rt).iter(|| async {
            let request = create_test_mcp_request("Hello, world!");
            gateway.process_request(request).await.unwrap()
        });
        
        rt.block_on(teardown_test_gateway(gateway)).unwrap();
    });
}

fn benchmark_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput");
    
    for concurrent_requests in [1, 5, 10, 25, 50].iter() {
        group.throughput(Throughput::Elements(*concurrent_requests as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent_requests", concurrent_requests),
            concurrent_requests,
            |b, &concurrent_requests| {
                let gateway = rt.block_on(setup_test_gateway()).unwrap();
                
                b.to_async(&rt).iter(|| async {
                    let mut handles = vec![];
                    
                    for i in 0..concurrent_requests {
                        let gateway_clone = gateway.clone();
                        let handle = tokio::spawn(async move {
                            let request = create_test_mcp_request(&format!("Request {}", i));
                            gateway_clone.process_request(request).await
                        });
                        handles.push(handle);
                    }
                    
                    futures::future::try_join_all(handles).await.unwrap()
                });
                
                rt.block_on(teardown_test_gateway(gateway)).unwrap();
            },
        );
    }
    group.finish();
}

fn benchmark_model_routing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("model_routing");
    
    let gateway = rt.block_on(setup_test_gateway()).unwrap();
    
    group.bench_function("simple_local", |b| {
        b.to_async(&rt).iter(|| async {
            let request = create_test_mcp_request("Simple prompt");
            gateway.process_request(request).await.unwrap()
        });
    });
    
    group.bench_function("complex_routing", |b| {
        b.to_async(&rt).iter(|| async {
            let request = create_complex_mcp_request();
            gateway.process_request(request).await.unwrap()
        });
    });
    
    rt.block_on(teardown_test_gateway(gateway)).unwrap();
    group.finish();
}

fn benchmark_queue_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("queue_operations");
    
    let mut gateway = rt.block_on(setup_test_gateway()).unwrap();
    rt.block_on(gateway.set_offline_mode(true)).unwrap();
    
    group.bench_function("queue_request", |b| {
        b.to_async(&rt).iter(|| async {
            let request = create_test_mcp_request("Queued request");
            gateway.process_request(request).await.unwrap()
        });
    });
    
    // Add some requests to queue first
    for i in 0..100 {
        let request = create_test_mcp_request(&format!("Batch request {}", i));
        rt.block_on(gateway.process_request(request)).unwrap();
    }
    
    rt.block_on(gateway.set_offline_mode(false)).unwrap();
    
    group.bench_function("sync_queue", |b| {
        b.to_async(&rt).iter(|| async {
            gateway.sync_queue().await.unwrap()
        });
    });
    
    rt.block_on(teardown_test_gateway(gateway)).unwrap();
    group.finish();
}

fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    let simple_request = create_test_mcp_request("Simple request");
    let complex_request = create_complex_mcp_request();
    
    group.bench_function("serialize_simple", |b| {
        b.iter(|| {
            serde_json::to_string(&simple_request).unwrap()
        });
    });
    
    group.bench_function("serialize_complex", |b| {
        b.iter(|| {
            serde_json::to_string(&complex_request).unwrap()
        });
    });
    
    let serialized_simple = serde_json::to_string(&simple_request).unwrap();
    let serialized_complex = serde_json::to_string(&complex_request).unwrap();
    
    group.bench_function("deserialize_simple", |b| {
        b.iter(|| {
            serde_json::from_str::<MCPRequest>(&serialized_simple).unwrap()
        });
    });
    
    group.bench_function("deserialize_complex", |b| {
        b.iter(|| {
            serde_json::from_str::<MCPRequest>(&serialized_complex).unwrap()
        });
    });
    
    group.finish();
}

#[cfg(feature = "compression")]
fn benchmark_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    let large_request = create_large_test_request();
    let serialized = serde_json::to_string(&large_request).unwrap();
    let data = serialized.as_bytes();
    
    group.bench_function("zstd_compress", |b| {
        b.iter(|| {
            zstd::bulk::compress(data, 3).unwrap()
        });
    });
    
    group.bench_function("lz4_compress", |b| {
        b.iter(|| {
            lz4_flex::compress_prepend_size(data)
        });
    });
    
    let zstd_compressed = zstd::bulk::compress(data, 3).unwrap();
    let lz4_compressed = lz4_flex::compress_prepend_size(data);
    
    group.bench_function("zstd_decompress", |b| {
        b.iter(|| {
            zstd::bulk::decompress(&zstd_compressed, data.len()).unwrap()
        });
    });
    
    group.bench_function("lz4_decompress", |b| {
        b.iter(|| {
            lz4_flex::decompress_size_prepended(&lz4_compressed).unwrap()
        });
    });
    
    group.finish();
}

#[cfg(feature = "hardware-security")]
fn benchmark_security_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("security");
    
    let gateway = rt.block_on(setup_test_gateway_with_tpm()).unwrap();
    
    group.bench_function("generate_key", |b| {
        b.to_async(&rt).iter(|| async {
            gateway.generate_device_key().await.unwrap()
        });
    });
    
    let key_id = rt.block_on(gateway.generate_device_key()).unwrap();
    let test_data = b"test data for signing";
    
    group.bench_function("sign_data", |b| {
        b.to_async(&rt).iter(|| async {
            gateway.sign_data(&key_id, test_data).await.unwrap()
        });
    });
    
    let signature = rt.block_on(gateway.sign_data(&key_id, test_data)).unwrap();
    
    group.bench_function("verify_signature", |b| {
        b.to_async(&rt).iter(|| async {
            gateway.verify_signature(&key_id, test_data, &signature).await.unwrap()
        });
    });
    
    rt.block_on(teardown_test_gateway(gateway)).unwrap();
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory");
    
    group.bench_function("gateway_startup", |b| {
        b.iter(|| {
            let gateway = rt.block_on(setup_test_gateway()).unwrap();
            rt.block_on(teardown_test_gateway(gateway)).unwrap();
        });
    });
    
    let gateway = rt.block_on(setup_test_gateway()).unwrap();
    
    group.bench_function("memory_pressure_simulation", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate memory pressure by processing many requests
            let mut handles = vec![];
            for i in 0..100 {
                let gateway_clone = gateway.clone();
                let handle = tokio::spawn(async move {
                    let request = create_test_mcp_request(&format!("Memory test {}", i));
                    gateway_clone.process_request(request).await
                });
                handles.push(handle);
            }
            futures::future::try_join_all(handles).await.unwrap()
        });
    });
    
    rt.block_on(teardown_test_gateway(gateway)).unwrap();
    group.finish();
}

// Custom benchmark configuration
fn custom_criterion() -> Criterion {
    Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100)
        .warm_up_time(Duration::from_secs(3))
        .with_plots()
}

// Helper function to create a large test request for compression benchmarks
fn create_large_test_request() -> MCPRequest {
    let large_content = "This is a large test request. ".repeat(1000);
    create_test_mcp_request(&large_content)
}

// Group all benchmarks
criterion_group!(
    name = benches;
    config = custom_criterion();
    targets = 
        benchmark_request_processing,
        benchmark_throughput,
        benchmark_model_routing,
        benchmark_queue_operations,
        benchmark_serialization,
        benchmark_memory_usage
);

#[cfg(feature = "compression")]
criterion_group!(
    name = compression_benches;
    config = custom_criterion();
    targets = benchmark_compression
);

#[cfg(feature = "hardware-security")]
criterion_group!(
    name = security_benches;
    config = custom_criterion();
    targets = benchmark_security_operations
);

// Main benchmark runner
#[cfg(all(feature = "compression", feature = "hardware-security"))]
criterion_main!(benches, compression_benches, security_benches);

#[cfg(all(feature = "compression", not(feature = "hardware-security")))]
criterion_main!(benches, compression_benches);

#[cfg(all(not(feature = "compression"), feature = "hardware-security"))]
criterion_main!(benches, security_benches);

#[cfg(all(not(feature = "compression"), not(feature = "hardware-security")))]
criterion_main!(benches);