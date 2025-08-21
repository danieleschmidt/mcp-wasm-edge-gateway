//! HTTP handlers for the MCP Gateway

use axum::{
    extract::{Json as ExtractJson, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use mcp_common::{MCPRequest, MCPResponse, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::gateway::Gateway;

/// Application state for handlers
pub type AppState = Arc<Gateway>;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    components: std::collections::HashMap<String, ComponentStatusInfo>,
}

/// Component status information
#[derive(Serialize)]
pub struct ComponentStatusInfo {
    status: String,
    message: String,
    last_check: String,
}

/// MCP request wrapper for HTTP
#[derive(Deserialize)]
pub struct HttpMCPRequest {
    method: String,
    params: Value,
    #[serde(default)]
    context: Option<Value>,
}

/// Create the router with all endpoints
pub fn create_router(gateway: Arc<Gateway>) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        
        // MCP endpoints
        .route("/v1/mcp/completions", post(handle_mcp_request))
        
        // Pipeline guard endpoints
        .route("/v1/pipeline/health", get(pipeline_health))
        .route("/v1/pipeline/metrics", get(pipeline_metrics))
        .route("/v1/pipeline/status", get(pipeline_status))
        .route("/v1/pipeline/components", get(pipeline_components))
        .route("/v1/pipeline/recover/:component_id", post(recover_component))
        .route("/v1/pipeline/force-check", post(force_health_check))
        
        // Metrics endpoints
        .route("/metrics", get(get_metrics))
        .route("/v1/metrics/performance", get(performance_metrics))
        
        .with_state(gateway)
}

/// Basic health check endpoint
pub async fn health_check(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.health_check().await {
        Ok(health) => {
            let status = match health.overall_health {
                mcp_common::HealthLevel::Healthy => "healthy",
                mcp_common::HealthLevel::Degraded => "degraded",
                mcp_common::HealthLevel::Critical => "critical",
                mcp_common::HealthLevel::Unknown => "unknown",
            };
            
            Json(serde_json::json!({
                "status": status,
                "timestamp": health.last_check,
                "uptime_seconds": health.uptime_seconds
            })).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Detailed health check endpoint
pub async fn detailed_health_check(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.health_check().await {
        Ok(health) => {
            let components = health.components
                .into_iter()
                .map(|(name, comp)| {
                    let status_str = match comp.status {
                        mcp_common::HealthLevel::Healthy => "healthy",
                        mcp_common::HealthLevel::Degraded => "degraded",
                        mcp_common::HealthLevel::Critical => "critical",
                        mcp_common::HealthLevel::Unknown => "unknown",
                    };
                    
                    (name, ComponentStatusInfo {
                        status: status_str.to_string(),
                        message: comp.message,
                        last_check: comp.last_check.to_rfc3339(),
                    })
                })
                .collect();

            let _gateway_state = gateway.state().await;
            
            Json(HealthResponse {
                status: match health.overall_health {
                    mcp_common::HealthLevel::Healthy => "healthy",
                    mcp_common::HealthLevel::Degraded => "degraded",
                    mcp_common::HealthLevel::Critical => "critical",
                    mcp_common::HealthLevel::Unknown => "unknown",
                }.to_string(),
                version: "0.1.0".to_string(),
                uptime_seconds: health.uptime_seconds,
                components,
            }).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Handle MCP requests with comprehensive validation and error handling
pub async fn handle_mcp_request(
    State(gateway): State<AppState>,
    ExtractJson(payload): ExtractJson<HttpMCPRequest>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4();
    
    // Input validation
    if payload.method.is_empty() {
        warn!("Rejected MCP request with empty method");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Method cannot be empty",
                    "request_id": request_id
                }
            }))
        ).into_response();
    }
    
    if payload.method.len() > 128 {
        warn!("Rejected MCP request with oversized method: {} chars", payload.method.len());
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST", 
                    "message": "Method name too long (max 128 characters)",
                    "request_id": request_id
                }
            }))
        ).into_response();
    }

    info!("Processing MCP request: method={}, id={}", payload.method, request_id);

    let request = MCPRequest {
        id: request_id,
        device_id: "http_client".to_string(),
        method: payload.method.clone(),
        params: payload.params.as_object()
            .map(|obj| obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
            .unwrap_or_default(),
        context: None, // Will be populated by the gateway if needed
        timestamp: chrono::Utc::now(),
    };

    match gateway.process_request(request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("MCP request completed: method={}, id={}, duration={:?}", 
                  payload.method, request_id, duration);
            Json(response).into_response()
        }
        Err(e) => {
            let duration = start_time.elapsed();
            error!("MCP request failed: method={}, id={}, duration={:?}, error={}", 
                   payload.method, request_id, duration, e);
            
            // Return structured error response
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "PROCESSING_FAILED",
                        "message": "Request processing failed",
                        "request_id": request_id,
                        "details": e.to_string()
                    }
                }))
            ).into_response()
        }
    }
}

/// Get pipeline health status
pub async fn pipeline_health(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.pipeline_guard().get_health_status().await {
        Ok(health) => {
            Json(serde_json::json!({
                "status": match health.status {
                    mcp_common::HealthLevel::Healthy => "healthy",
                    mcp_common::HealthLevel::Degraded => "degraded", 
                    mcp_common::HealthLevel::Critical => "critical",
                    mcp_common::HealthLevel::Unknown => "unknown",
                },
                "message": health.message,
                "last_check": health.last_check,
                "metrics": health.metrics
            })).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Get pipeline metrics
pub async fn pipeline_metrics(State(gateway): State<AppState>) -> impl IntoResponse {
    let metrics = gateway.pipeline_guard().get_pipeline_metrics().await;
    Json(serde_json::json!(metrics))
}

/// Get pipeline status and summary
pub async fn pipeline_status(State(gateway): State<AppState>) -> impl IntoResponse {
    let metrics = gateway.pipeline_guard().get_pipeline_metrics().await;
    let health = gateway.pipeline_guard().get_health_status().await;
    
    match health {
        Ok(health) => {
            Json(serde_json::json!({
                "health": {
                    "status": match health.status {
                        mcp_common::HealthLevel::Healthy => "healthy",
                        mcp_common::HealthLevel::Degraded => "degraded",
                        mcp_common::HealthLevel::Critical => "critical",
                        mcp_common::HealthLevel::Unknown => "unknown",
                    },
                    "message": health.message,
                    "last_check": health.last_check
                },
                "metrics": metrics,
                "timestamp": chrono::Utc::now()
            })).into_response()
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Get all pipeline components
pub async fn pipeline_components(State(gateway): State<AppState>) -> impl IntoResponse {
    let metrics = gateway.pipeline_guard().get_pipeline_metrics().await;
    Json(serde_json::json!({
        "total_components": metrics.get("total_components").unwrap_or(&0.0),
        "healthy_components": metrics.get("healthy_components").unwrap_or(&0.0),
        "failed_components": metrics.get("failed_components").unwrap_or(&0.0),
        "timestamp": chrono::Utc::now()
    }))
}

/// Manually trigger recovery for a component
pub async fn recover_component(
    State(gateway): State<AppState>,
    Path(component_id): Path<String>,
) -> impl IntoResponse {
    info!("Manual recovery requested for component: {}", component_id);
    
    match gateway.pipeline_guard().recover_component(&component_id).await {
        Ok(_) => {
            Json(serde_json::json!({
                "status": "success",
                "message": format!("Recovery initiated for component: {}", component_id),
                "timestamp": chrono::Utc::now()
            }))
        }
        Err(e) => {
            error!("Recovery failed for component {}: {}", component_id, e);
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Recovery failed: {}", e),
                "timestamp": chrono::Utc::now()
            }))
        }
    }
}

/// Force a health check cycle
pub async fn force_health_check(State(gateway): State<AppState>) -> impl IntoResponse {
    info!("Manual health check requested");
    
    match gateway.pipeline_guard().force_health_check().await {
        Ok(_) => {
            Json(serde_json::json!({
                "status": "success",
                "message": "Health check completed",
                "timestamp": chrono::Utc::now()
            })).into_response()
        }
        Err(e) => {
            error!("Forced health check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Get system metrics
pub async fn get_metrics(State(gateway): State<AppState>) -> Response {
    let result = async {
        // Convert to Prometheus format
        let mut output = String::new();
        
        // Add basic gateway metrics
        let state = gateway.state().await;
        output.push_str(&format!("mcp_gateway_active_requests {}\n", state.active_requests));
        output.push_str(&format!("mcp_gateway_total_requests {}\n", state.total_requests));
        output.push_str(&format!("mcp_gateway_uptime_seconds {}\n", 
            chrono::Utc::now().signed_duration_since(state.started_at).num_seconds()));
        
        // Add pipeline guard metrics
        let pipeline_metrics = gateway.pipeline_guard().get_pipeline_metrics().await;
        for (key, value) in pipeline_metrics {
            output.push_str(&format!("mcp_pipeline_{} {}\n", key, value));
        }
        
        output
    }.await;
    
    Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(result.into())
        .unwrap_or_else(|e| {
            error!("Failed to build metrics response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Failed to generate metrics".into())
                .unwrap_or_default()
        })
}

/// Get detailed performance metrics
pub async fn performance_metrics(State(gateway): State<AppState>) -> impl IntoResponse {
    let metrics = gateway.get_performance_metrics().await;
    Json(serde_json::json!({
        "performance": {
            "total_requests": metrics.total_requests,
            "successful_requests": metrics.successful_requests,
            "failed_requests": metrics.failed_requests,
            "success_rate": if metrics.total_requests > 0 {
                metrics.successful_requests as f32 / metrics.total_requests as f32
            } else { 0.0 },
            "avg_response_time_ms": metrics.avg_response_time_ms,
            "peak_response_time_ms": metrics.peak_response_time_ms,
            "requests_per_second": metrics.requests_per_second,
            "cache_hit_rate": metrics.cache_hit_rate,
            "cache_size": metrics.cache_size,
            "resource_usage": {
                "cpu_usage": metrics.current_cpu_usage,
                "memory_usage": metrics.current_memory_usage,
                "active_connections": metrics.active_connections
            }
        },
        "auto_scaling": {
            "should_scale_up": gateway.should_scale_up().await,
            "thresholds": {
                "cpu": 0.75,
                "memory": 0.8,
                "request_rate": 100.0
            }
        },
        "timestamp": chrono::Utc::now()
    })).into_response()
}