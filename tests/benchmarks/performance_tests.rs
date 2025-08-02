// Performance and benchmark tests for MCP WASM Edge Gateway

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use std::sync::Arc;
use futures::stream::{self, StreamExt};

use crate::utils::{TestDataGenerator, PerformanceMetrics};
use crate::utils::mock_services::{MockModelEngine, MockCloudService};

/// Benchmark single request processing
fn benchmark_single_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = MockModelEngine::new();
    
    c.bench_function("single_request", |b| {
        b.to_async(&rt).iter(|| async {
            let result = engine.process_request(
                "test-model", 
                black_box("What is edge computing?")
            ).await;
            
            black_box(result)
        })
    });
}

/// Benchmark concurrent request processing
fn benchmark_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockModelEngine::new());
    
    let sizes = vec![1, 10, 50, 100];
    
    for size in sizes {
        c.bench_with_input(
            BenchmarkId::new("concurrent_requests", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let requests = (0..size).map(|i| {
                        let engine = engine.clone();
                        async move {
                            engine.process_request(
                                "test-model",
                                &format!("Request {}", i)
                            ).await
                        }
                    });
                    
                    let results: Vec<_> = stream::iter(requests)
                        .buffer_unordered(size)
                        .collect()
                        .await;
                    
                    black_box(results)
                })
            },
        );
    }
}

/// Benchmark different model sizes
fn benchmark_model_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = MockModelEngine::new();
    
    let models = vec!["test-model", "large-model"];
    
    for model in models {
        c.bench_with_input(
            BenchmarkId::new("model_processing", model),
            &model,
            |b, &model| {
                b.to_async(&rt).iter(|| async {
                    let result = engine.process_request(
                        model,
                        black_box("Explain artificial intelligence")
                    ).await;
                    
                    black_box(result)
                })
            },
        );
    }
}

/// Benchmark memory usage patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let generator = TestDataGenerator::new();
    
    let sizes = vec!["small", "medium", "large"];
    
    for size in sizes {
        c.bench_with_input(
            BenchmarkId::new("memory_usage", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let data = generator.generate_test_data(size);
                    black_box(data)
                })
            },
        );
    }
}

/// Benchmark latency under different conditions
fn benchmark_latency_conditions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Test different latency scenarios
    let latencies = vec![10, 50, 100, 500]; // milliseconds
    
    for latency in latencies {
        let service = MockCloudService::new(latency, 1.0);
        
        c.bench_with_input(
            BenchmarkId::new("latency_impact", latency),
            &latency,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let request = serde_json::json!({
                        "messages": [{"role": "user", "content": "Test message"}]
                    });
                    
                    let result = service.handle_request(black_box(request)).await;
                    black_box(result)
                })
            },
        );
    }
}

/// Benchmark WASM compilation and execution
fn benchmark_wasm_operations(c: &mut Criterion) {
    // This would test WASM-specific operations
    // For now, we'll simulate the overhead
    
    c.bench_function("wasm_simulation", |b| {
        b.iter(|| {
            // Simulate WASM overhead
            let data = vec![1u8; 1024];
            let processed: Vec<u8> = data.iter()
                .map(|&x| black_box(x.wrapping_mul(2)))
                .collect();
            black_box(processed)
        })
    });
}

/// Throughput testing
fn benchmark_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockModelEngine::new());
    
    c.bench_function("throughput_test", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = engine.clone();
            let start = Instant::now();
            let duration = Duration::from_secs(1);
            let mut request_count = 0;
            
            while start.elapsed() < duration {
                let _ = engine.process_request(
                    "test-model",
                    "Quick throughput test"
                ).await;
                request_count += 1;
                
                if request_count > 1000 {
                    break; // Safety limit
                }
            }
            
            black_box(request_count)
        })
    });
}

/// Load testing simulation
fn benchmark_load_testing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockModelEngine::new());
    
    // Simulate different load levels
    let load_levels = vec![10, 25, 50, 100]; // requests per second
    
    for rps in load_levels {
        c.bench_with_input(
            BenchmarkId::new("load_test", rps),
            &rps,
            |b, &rps| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let interval = Duration::from_millis(1000 / rps as u64);
                    let mut tasks = vec![];
                    
                    for i in 0..rps {
                        let engine = engine.clone();
                        let task = tokio::spawn(async move {
                            tokio::time::sleep(interval * i as u32).await;
                            engine.process_request(
                                "test-model",
                                &format!("Load test request {}", i)
                            ).await
                        });
                        tasks.push(task);
                    }
                    
                    let results = futures::future::join_all(tasks).await;
                    black_box(results)
                })
            },
        );
    }
}

/// Performance regression testing
fn benchmark_regression_tests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = MockModelEngine::new();
    
    c.bench_function("regression_baseline", |b| {
        b.to_async(&rt).iter(|| async {
            // This test ensures performance doesn't regress
            let start = Instant::now();
            
            let result = engine.process_request(
                "test-model",
                black_box("Standard regression test prompt")
            ).await;
            
            let duration = start.elapsed();
            
            // Assert maximum acceptable latency
            assert!(
                duration < Duration::from_millis(200),
                "Performance regression detected: {}ms > 200ms",
                duration.as_millis()
            );
            
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    benchmark_single_request,
    benchmark_concurrent_requests,
    benchmark_model_sizes,
    benchmark_memory_usage,
    benchmark_latency_conditions,
    benchmark_wasm_operations,
    benchmark_throughput,
    benchmark_load_testing,
    benchmark_regression_tests
);

criterion_main!(benches);

/// Performance test runner for integration testing
pub struct PerformanceTestRunner {
    engine: Arc<MockModelEngine>,
    cloud_service: Arc<MockCloudService>,
}

impl PerformanceTestRunner {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(MockModelEngine::new()),
            cloud_service: Arc::new(MockCloudService::new(100, 0.95)),
        }
    }
    
    /// Run comprehensive performance test suite
    pub async fn run_performance_suite(&self) -> PerformanceMetrics {
        let start = Instant::now();
        let test_duration = Duration::from_secs(30);
        let mut request_count = 0;
        let mut error_count = 0;
        let mut latencies = Vec::new();
        
        while start.elapsed() < test_duration {
            let request_start = Instant::now();
            
            let result = self.engine.process_request(
                "test-model",
                &format!("Performance test request {}", request_count)
            ).await;
            
            let latency = request_start.elapsed();
            latencies.push(latency.as_millis() as f64);
            
            match result {
                Ok(_) => request_count += 1,
                Err(_) => error_count += 1,
            }
            
            // Small delay to prevent overwhelming the system
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        self.calculate_metrics(request_count, error_count, latencies, start.elapsed())
    }
    
    /// Run stress test with high concurrency
    pub async fn run_stress_test(&self, concurrent_requests: usize) -> PerformanceMetrics {
        let start = Instant::now();
        let requests = (0..concurrent_requests).map(|i| {
            let engine = self.engine.clone();
            async move {
                let request_start = Instant::now();
                let result = engine.process_request(
                    "test-model",
                    &format!("Stress test request {}", i)
                ).await;
                (result, request_start.elapsed())
            }
        });
        
        let results: Vec<_> = stream::iter(requests)
            .buffer_unordered(concurrent_requests)
            .collect()
            .await;
        
        let total_duration = start.elapsed();
        let mut request_count = 0;
        let mut error_count = 0;
        let mut latencies = Vec::new();
        
        for (result, latency) in results {
            latencies.push(latency.as_millis() as f64);
            match result {
                Ok(_) => request_count += 1,
                Err(_) => error_count += 1,
            }
        }
        
        self.calculate_metrics(request_count, error_count, latencies, total_duration)
    }
    
    /// Calculate performance metrics from test results
    fn calculate_metrics(
        &self,
        request_count: usize,
        error_count: usize,
        mut latencies: Vec<f64>,
        total_duration: Duration,
    ) -> PerformanceMetrics {
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let total_requests = request_count + error_count;
        let rps = total_requests as f64 / total_duration.as_secs_f64();
        let error_rate = if total_requests > 0 {
            error_count as f64 / total_requests as f64
        } else {
            0.0
        };
        
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };
        
        let p95_latency = if !latencies.is_empty() {
            let index = (latencies.len() as f64 * 0.95) as usize;
            latencies.get(index.min(latencies.len() - 1)).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        
        let p99_latency = if !latencies.is_empty() {
            let index = (latencies.len() as f64 * 0.99) as usize;
            latencies.get(index.min(latencies.len() - 1)).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        
        PerformanceMetrics {
            requests_per_second: rps,
            average_latency_ms: avg_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            error_rate,
            memory_usage_mb: self.estimate_memory_usage(),
            cpu_usage_percent: 50.0, // Mock value
        }
    }
    
    /// Estimate memory usage (mock implementation)
    fn estimate_memory_usage(&self) -> f64 {
        // This would integrate with actual memory monitoring
        // For testing, return a mock value
        64.0 + fastrand::f64() * 32.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_performance_suite() {
        let runner = PerformanceTestRunner::new();
        let metrics = runner.run_performance_suite().await;
        
        assert!(metrics.requests_per_second > 0.0);
        assert!(metrics.average_latency_ms > 0.0);
        assert!(metrics.error_rate >= 0.0 && metrics.error_rate <= 1.0);
    }
    
    #[tokio::test]
    async fn test_stress_test() {
        let runner = PerformanceTestRunner::new();
        let metrics = runner.run_stress_test(10).await;
        
        assert!(metrics.requests_per_second > 0.0);
        assert!(metrics.p95_latency_ms >= metrics.average_latency_ms);
        assert!(metrics.p99_latency_ms >= metrics.p95_latency_ms);
    }
}