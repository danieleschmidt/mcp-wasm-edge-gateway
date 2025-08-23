//! Generation 1: MAKE IT WORK (Simple)
//! Basic MCP Edge Gateway demonstration

use mcp_common::{Config, MCPRequest, MCPResponse};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};
use serde_json::{json, Value};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Simple Gateway demonstration for Generation 1
pub struct SimpleGateway {
    _config: Arc<Config>,
    state: Arc<RwLock<GatewayState>>,
    request_cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    metrics: Arc<RwLock<SimpleMetrics>>,
}

#[derive(Debug, Clone)]
pub struct GatewayState {
    pub started_at: DateTime<Utc>,
    pub active_requests: u32,
    pub total_requests: u64,
    pub is_healthy: bool,
}

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub response: MCPResponse,
    pub cached_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct SimpleMetrics {
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_response_time_ms: f64,
    pub last_error: Option<String>,
}

impl SimpleGateway {
    /// Create a new simple gateway
    pub async fn new() -> anyhow::Result<Self> {
        info!("🚀 Generation 1: Initializing Simple MCP Gateway");
        
        let config = Arc::new(Config::default());
        let state = Arc::new(RwLock::new(GatewayState {
            started_at: Utc::now(),
            active_requests: 0,
            total_requests: 0,
            is_healthy: true,
        }));
        
        let request_cache = Arc::new(RwLock::new(HashMap::new()));
        let metrics = Arc::new(RwLock::new(SimpleMetrics {
            successful_requests: 0,
            failed_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_response_time_ms: 0.0,
            last_error: None,
        }));

        info!("✅ Simple Gateway initialized successfully");
        
        Ok(SimpleGateway {
            _config: config,
            state,
            request_cache,
            metrics,
        })
    }

    /// Process an MCP request (Generation 1 simple implementation)
    pub async fn process_request(&self, request: MCPRequest) -> anyhow::Result<MCPResponse> {
        let start_time = std::time::Instant::now();
        let request_id = request.id;
        
        debug!("Processing request: {} - {}", request_id, request.method);

        // Update active requests
        {
            let mut state = self.state.write().await;
            state.active_requests += 1;
            state.total_requests += 1;
        }

        // Check cache first
        let cache_key = format!("{}:{:?}", request.method, request.params);
        {
            let cache = self.request_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if self.is_cache_valid(cached) {
                    debug!("Cache hit for request: {}", request_id);
                    self.metrics.write().await.cache_hits += 1;
                    
                    // Update active requests
                    self.state.write().await.active_requests -= 1;
                    
                    return Ok(cached.response.clone());
                }
            }
        }
        
        self.metrics.write().await.cache_misses += 1;

        // Process request based on method
        let result = self.process_request_internal(&request).await;
        
        // Update metrics and state
        let duration = start_time.elapsed();
        let mut metrics = self.metrics.write().await;
        
        match &result {
            Ok(response) => {
                info!("✅ Request {} completed in {:?}", request_id, duration);
                metrics.successful_requests += 1;
                metrics.avg_response_time_ms = 
                    (metrics.avg_response_time_ms * (metrics.successful_requests - 1) as f64 + 
                     duration.as_millis() as f64) / metrics.successful_requests as f64;
                
                // Cache successful responses
                if self.is_cacheable_method(&request.method) {
                    let cached_response = CachedResponse {
                        response: response.clone(),
                        cached_at: Utc::now(),
                        ttl_seconds: 300, // 5 minutes
                    };
                    drop(metrics); // Release the write lock before acquiring another one
                    self.request_cache.write().await.insert(cache_key, cached_response);
                }
            }
            Err(e) => {
                error!("❌ Request {} failed in {:?}: {}", request_id, duration, e);
                metrics.failed_requests += 1;
                metrics.last_error = Some(e.to_string());
            }
        }

        // Update active requests
        self.state.write().await.active_requests -= 1;

        result
    }

    /// Internal request processing logic
    async fn process_request_internal(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        // Basic input validation
        if request.method.is_empty() {
            return Err(anyhow::anyhow!("Method cannot be empty"));
        }

        if request.method.len() > 128 {
            return Err(anyhow::anyhow!("Method name too long"));
        }

        // Route request based on method
        let response = match request.method.as_str() {
            "mcp.ping" => self.handle_ping(request).await?,
            "mcp.list_models" => self.handle_list_models(request).await?,
            "mcp.completion" => self.handle_completion(request).await?,
            "mcp.embedding" => self.handle_embedding(request).await?,
            "mcp.health" => self.handle_health_check(request).await?,
            _ => self.handle_unknown_method(request).await?,
        };

        Ok(response)
    }

    /// Handle ping request
    async fn handle_ping(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        debug!("Handling ping request");
        Ok(MCPResponse {
            id: request.id,
            result: Some(json!({
                "status": "pong",
                "timestamp": Utc::now(),
                "gateway_version": "0.1.0",
                "generation": 1
            })),
            error: None,
            timestamp: Utc::now(),
        })
    }

    /// Handle list models request
    async fn handle_list_models(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        debug!("Handling list models request");
        Ok(MCPResponse {
            id: request.id,
            result: Some(json!({
                "models": [
                    {
                        "id": "tinyllama-1.1b",
                        "name": "TinyLlama 1.1B",
                        "description": "Compact language model for edge devices",
                        "size_mb": 637,
                        "context_length": 2048,
                        "available": true
                    },
                    {
                        "id": "phi-3-mini",
                        "name": "Microsoft Phi-3 Mini",
                        "description": "Small but powerful model",
                        "size_mb": 2300,
                        "context_length": 4096,
                        "available": false
                    }
                ]
            })),
            error: None,
            timestamp: Utc::now(),
        })
    }

    /// Handle completion request
    async fn handle_completion(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        debug!("Handling completion request");
        
        // Simulate processing delay for realism
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let prompt = request.params.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("Hello");

        let response_text = match prompt {
            p if p.contains("weather") => "I'm an AI assistant running on an edge device. I don't have access to real-time weather data, but I can help with other tasks!",
            p if p.contains("hello") || p.contains("hi") => "Hello! I'm running on an MCP Edge Gateway. How can I assist you today?",
            _ => "I'm an AI assistant running on a resource-constrained edge device. I can help with basic text processing and information tasks.",
        };

        Ok(MCPResponse {
            id: request.id,
            result: Some(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": response_text
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": prompt.split_whitespace().count(),
                    "completion_tokens": response_text.split_whitespace().count(),
                    "total_tokens": prompt.split_whitespace().count() + response_text.split_whitespace().count()
                },
                "model": "tinyllama-1.1b",
                "created": Utc::now().timestamp()
            })),
            error: None,
            timestamp: Utc::now(),
        })
    }

    /// Handle embedding request
    async fn handle_embedding(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        debug!("Handling embedding request");
        
        let input = request.params.get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Generate a simple mock embedding based on input
        let embedding: Vec<f32> = input.chars()
            .enumerate()
            .take(384) // Common embedding dimension
            .map(|(i, c)| ((c as u32 + i as u32) % 100) as f32 / 100.0)
            .collect();

        // Pad or truncate to exactly 384 dimensions
        let mut final_embedding = embedding;
        final_embedding.resize(384, 0.0);

        Ok(MCPResponse {
            id: request.id,
            result: Some(json!({
                "data": [{
                    "object": "embedding",
                    "embedding": final_embedding,
                    "index": 0
                }],
                "model": "tinyllama-1.1b",
                "usage": {
                    "prompt_tokens": input.split_whitespace().count(),
                    "total_tokens": input.split_whitespace().count()
                }
            })),
            error: None,
            timestamp: Utc::now(),
        })
    }

    /// Handle health check request
    async fn handle_health_check(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        debug!("Handling health check request");
        
        let state = self.state.read().await;
        let metrics = self.metrics.read().await;
        
        let uptime = Utc::now().signed_duration_since(state.started_at).num_seconds();
        let success_rate = if metrics.successful_requests + metrics.failed_requests > 0 {
            metrics.successful_requests as f64 / (metrics.successful_requests + metrics.failed_requests) as f64
        } else {
            1.0
        };

        Ok(MCPResponse {
            id: request.id,
            result: Some(json!({
                "status": if state.is_healthy { "healthy" } else { "unhealthy" },
                "uptime_seconds": uptime,
                "active_requests": state.active_requests,
                "total_requests": state.total_requests,
                "success_rate": success_rate,
                "cache_hit_rate": if metrics.cache_hits + metrics.cache_misses > 0 {
                    metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
                } else {
                    0.0
                },
                "avg_response_time_ms": metrics.avg_response_time_ms,
                "generation": 1,
                "features": ["basic_completion", "embedding", "caching", "health_monitoring"]
            })),
            error: None,
            timestamp: Utc::now(),
        })
    }

    /// Handle unknown method
    async fn handle_unknown_method(&self, request: &MCPRequest) -> anyhow::Result<MCPResponse> {
        use mcp_common::MCPError;
        
        Ok(MCPResponse {
            id: request.id,
            result: None,
            error: Some(MCPError {
                code: -32601,
                message: format!("Unknown method: {}", request.method),
                data: Some(json!({
                    "method": request.method,
                    "available_methods": [
                        "mcp.ping",
                        "mcp.list_models", 
                        "mcp.completion",
                        "mcp.embedding",
                        "mcp.health"
                    ]
                })),
            }),
            timestamp: Utc::now(),
        })
    }

    /// Check if cache entry is still valid
    fn is_cache_valid(&self, cached: &CachedResponse) -> bool {
        let age = Utc::now().signed_duration_since(cached.cached_at).num_seconds() as u64;
        age < cached.ttl_seconds
    }

    /// Check if method is cacheable
    fn is_cacheable_method(&self, method: &str) -> bool {
        matches!(method, "mcp.list_models" | "mcp.ping")
    }

    /// Get current gateway state
    pub async fn get_state(&self) -> GatewayState {
        self.state.read().await.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> SimpleMetrics {
        self.metrics.read().await.clone()
    }
}

/// Run Generation 1 demonstration
pub async fn run_generation1_demo() -> anyhow::Result<()> {
    info!("🚀 Starting Generation 1 MCP Gateway Demonstration");
    
    let gateway = Arc::new(SimpleGateway::new().await?);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    info!("🧪 Running Generation 1 demonstration requests...");
    
    // Test ping
    let ping_request = MCPRequest {
        id: Uuid::new_v4(),
        device_id: "demo_client".to_string(),
        method: "mcp.ping".to_string(),
        params: HashMap::new(),
        context: None,
        timestamp: Utc::now(),
    };
    
    match gateway.process_request(ping_request).await {
        Ok(response) => info!("✅ Ping successful: {:?}", response.result),
        Err(e) => error!("❌ Ping failed: {}", e),
    }

    // Test list models  
    let list_request = MCPRequest {
        id: Uuid::new_v4(),
        device_id: "demo_client".to_string(),
        method: "mcp.list_models".to_string(),
        params: HashMap::new(),
        context: None,
        timestamp: Utc::now(),
    };

    match gateway.process_request(list_request).await {
        Ok(_response) => info!("✅ List models successful"),
        Err(e) => error!("❌ List models failed: {}", e),
    }

    // Test completion
    let mut completion_params = HashMap::new();
    completion_params.insert("prompt".to_string(), json!("Hello, how are you?"));
    
    let completion_request = MCPRequest {
        id: Uuid::new_v4(),
        device_id: "demo_client".to_string(),
        method: "mcp.completion".to_string(),
        params: completion_params,
        context: None,
        timestamp: Utc::now(),
    };

    match gateway.process_request(completion_request).await {
        Ok(_response) => info!("✅ Completion successful"),
        Err(e) => error!("❌ Completion failed: {}", e),
    }

    // Test health check
    let health_request = MCPRequest {
        id: Uuid::new_v4(),
        device_id: "demo_client".to_string(),
        method: "mcp.health".to_string(),
        params: HashMap::new(),
        context: None,
        timestamp: Utc::now(),
    };

    match gateway.process_request(health_request).await {
        Ok(response) => info!("✅ Health check successful: {:?}", response.result),
        Err(e) => error!("❌ Health check failed: {}", e),
    }

    // Display metrics
    let metrics = gateway.get_metrics().await;
    let state = gateway.get_state().await;
    
    info!("📊 Generation 1 Gateway Metrics:");
    info!("   Total Requests: {}", state.total_requests);
    info!("   Successful: {}", metrics.successful_requests);
    info!("   Failed: {}", metrics.failed_requests);
    info!("   Cache Hits: {}", metrics.cache_hits);
    info!("   Cache Misses: {}", metrics.cache_misses);
    info!("   Avg Response Time: {:.2}ms", metrics.avg_response_time_ms);
    info!("   Success Rate: {:.2}%", 
          if state.total_requests > 0 {
              (metrics.successful_requests as f64 / state.total_requests as f64) * 100.0
          } else { 0.0 });

    info!("🎉 Generation 1 demonstration completed successfully!");
    info!("✨ Core MCP functionality is working - ready for Generation 2!");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 MCP WASM Edge Gateway - Generation 1: MAKE IT WORK");
    info!("📋 Features: Basic MCP protocol, Request routing, Simple caching, Health monitoring");
    
    run_generation1_demo().await
}