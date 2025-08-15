//! HTTP handlers for the MCP Gateway

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use mcp_common::{MCPRequest, MCPResponse, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info, debug};

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
        
        // Metrics endpoint
        .route("/metrics", get(get_metrics))
        
        .with_state(gateway)
}

/// Basic health check endpoint
pub async fn health_check(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match gateway.health_check().await {
        Ok(health) => {
            let status = match health.overall_health {
                mcp_common::HealthLevel::Healthy => "healthy",
                mcp_common::HealthLevel::Degraded => "degraded",
                mcp_common::HealthLevel::Critical => "critical",
            };
            
            Ok(Json(serde_json::json!({
                "status": status,
                "timestamp": health.last_check,
                "uptime_seconds": health.uptime_seconds
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Detailed health check endpoint
pub async fn detailed_health_check(State(gateway): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    match gateway.health_check().await {
        Ok(health) => {
            let components = health.components
                .into_iter()
                .map(|(name, comp)| {
                    let status_str = match comp.status {
                        mcp_common::HealthLevel::Healthy => "healthy",
                        mcp_common::HealthLevel::Degraded => "degraded",
                        mcp_common::HealthLevel::Critical => "critical",
                    };
                    
                    (name, ComponentStatusInfo {
                        status: status_str.to_string(),
                        message: comp.message,
                        last_check: comp.last_check.to_rfc3339(),
                    })
                })
                .collect();

            let gateway_state = gateway.state().await;
            
            Ok(Json(HealthResponse {
                status: match health.overall_health {
                    mcp_common::HealthLevel::Healthy => "healthy",
                    mcp_common::HealthLevel::Degraded => "degraded",
                    mcp_common::HealthLevel::Critical => "critical",
                }.to_string(),
                version: "0.1.0".to_string(),
                uptime_seconds: health.uptime_seconds,
                components,
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Handle MCP requests
pub async fn handle_mcp_request(
    State(gateway): State<AppState>,
    Json(payload): Json<HttpMCPRequest>,
) -> Result<Json<MCPResponse>, StatusCode> {
    debug!("Received MCP request: {}", payload.method);

    let request = MCPRequest {
        id: uuid::Uuid::new_v4(),
        device_id: "http_client".to_string(),
        method: payload.method,
        params: payload.params.as_object()
            .map(|obj| obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
            .unwrap_or_default(),
        context: None, // Will be populated by the gateway if needed
        timestamp: chrono::Utc::now(),
    };

    match gateway.process_request(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("MCP request processing failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get pipeline health status
pub async fn pipeline_health(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match gateway.pipeline_guard().get_health_status().await {
        Ok(health) => {
            Ok(Json(serde_json::json!({
                "status": match health.status {
                    mcp_common::HealthLevel::Healthy => "healthy",
                    mcp_common::HealthLevel::Degraded => "degraded", 
                    mcp_common::HealthLevel::Critical => "critical",
                },
                "message": health.message,
                "last_check": health.last_check,
                "metrics": health.metrics
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get pipeline metrics
pub async fn pipeline_metrics(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match gateway.pipeline_guard().get_pipeline_metrics().await {
        Ok(metrics) => Ok(Json(serde_json::json!(metrics))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get pipeline status and summary
pub async fn pipeline_status(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let metrics = gateway.pipeline_guard().get_pipeline_metrics().await;
    let health = gateway.pipeline_guard().get_health_status().await;
    
    match (metrics, health) {
        (Ok(metrics), Ok(health)) => {
            Ok(Json(serde_json::json!({
                "health": {
                    "status": match health.status {
                        mcp_common::HealthLevel::Healthy => "healthy",
                        mcp_common::HealthLevel::Degraded => "degraded",
                        mcp_common::HealthLevel::Critical => "critical",
                    },
                    "message": health.message,
                    "last_check": health.last_check
                },
                "metrics": metrics,
                "timestamp": chrono::Utc::now()
            })))
        }
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get all pipeline components
pub async fn pipeline_components(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    // This would return detailed component information
    // For now, return basic metrics
    match gateway.pipeline_guard().get_pipeline_metrics().await {
        Ok(metrics) => {
            Ok(Json(serde_json::json!({
                "total_components": metrics.get("total_components").unwrap_or(&0.0),
                "healthy_components": metrics.get("healthy_components").unwrap_or(&0.0),
                "failed_components": metrics.get("failed_components").unwrap_or(&0.0),
                "timestamp": chrono::Utc::now()
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Manually trigger recovery for a component
pub async fn recover_component(
    State(gateway): State<AppState>,
    Path(component_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    info!("Manual recovery requested for component: {}", component_id);
    
    match gateway.pipeline_guard().recover_component(&component_id).await {
        Ok(_) => {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": format!("Recovery initiated for component: {}", component_id),
                "timestamp": chrono::Utc::now()
            })))
        }
        Err(e) => {
            error!("Recovery failed for component {}: {}", component_id, e);
            Ok(Json(serde_json::json!({
                "status": "error",
                "message": format!("Recovery failed: {}", e),
                "timestamp": chrono::Utc::now()
            })))
        }
    }
}

/// Force a health check cycle
pub async fn force_health_check(State(gateway): State<AppState>) -> Result<Json<Value>, StatusCode> {
    info!("Manual health check requested");
    
    match gateway.pipeline_guard().force_health_check().await {
        Ok(_) => {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Health check completed",
                "timestamp": chrono::Utc::now()
            })))
        }
        Err(e) => {
            error!("Forced health check failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get system metrics
pub async fn get_metrics(State(gateway): State<AppState>) -> Result<String, StatusCode> {
    match gateway.get_metrics().await {
        Ok(metrics) => {
            // Convert to Prometheus format
            let mut output = String::new();
            
            // Add basic gateway metrics
            let state = gateway.state().await;
            output.push_str(&format!("mcp_gateway_active_requests {}\n", state.active_requests));
            output.push_str(&format!("mcp_gateway_total_requests {}\n", state.total_requests));
            output.push_str(&format!("mcp_gateway_uptime_seconds {}\n", 
                chrono::Utc::now().signed_duration_since(state.started_at).num_seconds()));
            
            // Add pipeline guard metrics
            if let Ok(pipeline_metrics) = gateway.pipeline_guard().get_pipeline_metrics().await {
                for (key, value) in pipeline_metrics {
                    output.push_str(&format!("mcp_pipeline_{} {}\n", key, value));
                }
            }
            
            Ok(output)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}