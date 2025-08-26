//! Generation 1: Simple Working Implementation
//! 
//! This is a minimal, functional implementation of the MCP WASM Edge Gateway
//! that demonstrates core functionality without complex dependencies.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Simple MCP Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMCPRequest {
    pub id: Uuid,
    pub method: String,
    pub params: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Simple MCP Response structure  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMCPResponse {
    pub id: Uuid,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Basic configuration for the edge gateway
#[derive(Debug, Clone)]
pub struct SimpleGatewayConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_ms: u64,
    pub local_model_enabled: bool,
    pub cloud_fallback_enabled: bool,
}

impl Default for SimpleGatewayConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            max_connections: 100,
            request_timeout_ms: 5000,
            local_model_enabled: true,
            cloud_fallback_enabled: false,
        }
    }
}

/// Simple edge gateway implementation
pub struct SimpleEdgeGateway {
    config: SimpleGatewayConfig,
    request_cache: Arc<RwLock<HashMap<Uuid, SimpleMCPResponse>>>,
    metrics: Arc<RwLock<GatewayMetrics>>,
}

/// Basic metrics tracking
#[derive(Debug, Default)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub local_model_requests: u64,
    pub cloud_fallback_requests: u64,
    pub avg_response_time_ms: f64,
}

impl SimpleEdgeGateway {
    /// Create a new simple edge gateway instance
    pub fn new(config: SimpleGatewayConfig) -> Self {
        Self {
            config,
            request_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(GatewayMetrics::default())),
        }
    }

    /// Process an MCP request
    pub async fn process_request(&self, request: SimpleMCPRequest) -> SimpleMCPResponse {
        let start_time = std::time::Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
        }

        // Simple request routing logic
        let response = if self.config.local_model_enabled {
            self.process_local_request(&request).await
        } else if self.config.cloud_fallback_enabled {
            self.process_cloud_request(&request).await
        } else {
            SimpleMCPResponse {
                id: request.id,
                result: serde_json::json!({"message": "No processing backend available"}),
                error: Some("No backend configured".to_string()),
                metadata: HashMap::new(),
            }
        };

        // Update response time metrics
        let duration = start_time.elapsed();
        {
            let mut metrics = self.metrics.write().await;
            if response.error.is_none() {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }
            
            // Simple moving average update
            metrics.avg_response_time_ms = 
                (metrics.avg_response_time_ms * 0.9) + (duration.as_millis() as f64 * 0.1);
        }

        // Cache the response
        {
            let mut cache = self.request_cache.write().await;
            cache.insert(request.id, response.clone());
            
            // Simple cache size management - keep only last 1000 responses
            if cache.len() > 1000 {
                let oldest_key = *cache.keys().next().unwrap();
                cache.remove(&oldest_key);
            }
        }

        response
    }

    /// Process request using local model (simulated)
    async fn process_local_request(&self, request: &SimpleMCPRequest) -> SimpleMCPResponse {
        // Update local model metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.local_model_requests += 1;
        }

        // Simulate local processing based on method
        let result = match request.method.as_str() {
            "completion" => {
                serde_json::json!({
                    "text": "This is a local model response",
                    "model": "local-edge-model",
                    "confidence": 0.85,
                    "processing_time_ms": 150
                })
            }
            "embedding" => {
                serde_json::json!({
                    "embeddings": vec![0.1, 0.2, 0.3, 0.4, 0.5],
                    "model": "local-embedding-model",
                    "dimensions": 5
                })
            }
            "health" => {
                serde_json::json!({
                    "status": "healthy",
                    "timestamp": Utc::now(),
                    "local_model": "available",
                    "memory_usage": "45%",
                    "cpu_usage": "12%"
                })
            }
            _ => {
                serde_json::json!({
                    "message": format!("Unknown method: {}", request.method),
                    "supported_methods": ["completion", "embedding", "health"]
                })
            }
        };

        let mut metadata = HashMap::new();
        metadata.insert("processor".to_string(), "local_edge".to_string());
        metadata.insert("timestamp".to_string(), Utc::now().to_rfc3339());

        SimpleMCPResponse {
            id: request.id,
            result,
            error: None,
            metadata,
        }
    }

    /// Process request using cloud fallback (simulated)
    async fn process_cloud_request(&self, request: &SimpleMCPRequest) -> SimpleMCPResponse {
        // Update cloud fallback metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.cloud_fallback_requests += 1;
        }

        // Simulate cloud processing delay
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let result = serde_json::json!({
            "text": "This is a cloud fallback response",
            "model": "cloud-model-gpt-4",
            "confidence": 0.95,
            "processing_time_ms": 800
        });

        let mut metadata = HashMap::new();
        metadata.insert("processor".to_string(), "cloud_fallback".to_string());
        metadata.insert("timestamp".to_string(), Utc::now().to_rfc3339());

        SimpleMCPResponse {
            id: request.id,
            result,
            error: None,
            metadata,
        }
    }

    /// Get current gateway metrics
    pub async fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().await.clone()
    }

    /// Get cached response if available
    pub async fn get_cached_response(&self, request_id: &Uuid) -> Option<SimpleMCPResponse> {
        self.request_cache.read().await.get(request_id).cloned()
    }

    /// Simple health check
    pub async fn health_check(&self) -> serde_json::Value {
        let metrics = self.get_metrics().await;
        
        serde_json::json!({
            "status": "healthy",
            "timestamp": Utc::now(),
            "config": {
                "port": self.config.port,
                "local_model_enabled": self.config.local_model_enabled,
                "cloud_fallback_enabled": self.config.cloud_fallback_enabled
            },
            "metrics": {
                "total_requests": metrics.total_requests,
                "success_rate": if metrics.total_requests > 0 {
                    (metrics.successful_requests as f64 / metrics.total_requests as f64 * 100.0)
                } else { 0.0 },
                "avg_response_time_ms": metrics.avg_response_time_ms,
                "local_requests": metrics.local_model_requests,
                "cloud_requests": metrics.cloud_fallback_requests
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    println!("🚀 Starting MCP WASM Edge Gateway - Generation 1 (Simple)");
    
    // Create configuration
    let mut config = SimpleGatewayConfig::default();
    config.local_model_enabled = true;
    config.cloud_fallback_enabled = false;
    
    println!("📋 Configuration: {:#?}", config);
    
    // Create gateway instance
    let gateway = SimpleEdgeGateway::new(config);
    
    // Demonstrate basic functionality
    println!("\n🔧 Testing basic functionality...");
    
    // Test health check
    let health = gateway.health_check().await;
    println!("💚 Health Check: {}", serde_json::to_string_pretty(&health)?);
    
    // Test different request types
    let test_requests = vec![
        SimpleMCPRequest {
            id: Uuid::new_v4(),
            method: "completion".to_string(),
            params: serde_json::json!({"prompt": "Hello, world!"}),
            timestamp: Utc::now(),
        },
        SimpleMCPRequest {
            id: Uuid::new_v4(),
            method: "embedding".to_string(),
            params: serde_json::json!({"text": "Sample text for embedding"}),
            timestamp: Utc::now(),
        },
        SimpleMCPRequest {
            id: Uuid::new_v4(),
            method: "health".to_string(),
            params: serde_json::json!({}),
            timestamp: Utc::now(),
        },
    ];
    
    println!("\n🎯 Processing test requests...");
    for (i, request) in test_requests.iter().enumerate() {
        let response = gateway.process_request(request.clone()).await;
        println!("📨 Request {}: {} -> Success: {}", 
                 i + 1, 
                 request.method, 
                 response.error.is_none());
        
        if let Some(error) = &response.error {
            println!("   ❌ Error: {}", error);
        } else {
            println!("   ✅ Response: {}", 
                     serde_json::to_string_pretty(&response.result)?);
        }
    }
    
    // Show final metrics
    println!("\n📊 Final Metrics:");
    let final_health = gateway.health_check().await;
    println!("{}", serde_json::to_string_pretty(&final_health)?);
    
    println!("\n🎉 Generation 1 Implementation Complete!");
    println!("✨ Key Features Demonstrated:");
    println!("   • Basic request routing and processing");
    println!("   • Local model simulation");
    println!("   • Response caching"); 
    println!("   • Metrics collection");
    println!("   • Health monitoring");
    println!("   • Error handling");
    
    Ok(())
}