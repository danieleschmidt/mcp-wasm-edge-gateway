//! HTTP request handlers

use crate::{AppState, Gateway};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Json as ResponseJson},
};
// WebSocket imports are handled locally in the websocket function
use mcp_common::{HealthStatus, MCPRequest, MCPResponse};
use serde_json::{json, Value};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Health check endpoint
pub async fn health_check(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.health_check().await {
        Ok(health) => {
            let status_code = match health.overall_health {
                mcp_common::HealthLevel::Healthy => StatusCode::OK,
                mcp_common::HealthLevel::Warning => StatusCode::OK,
                mcp_common::HealthLevel::Critical => StatusCode::SERVICE_UNAVAILABLE,
                mcp_common::HealthLevel::Unknown => StatusCode::SERVICE_UNAVAILABLE,
            };
            (status_code, ResponseJson(health))
        },
        Err(e) => {
            error!("Health check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            )
        },
    }
}

/// Readiness check endpoint
pub async fn readiness_check(State(gateway): State<AppState>) -> impl IntoResponse {
    let state = gateway.state().await;

    if state.is_healthy {
        (
            StatusCode::OK,
            ResponseJson(json!({
                "status": "ready",
                "timestamp": chrono::Utc::now()
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            ResponseJson(json!({
                "status": "not_ready",
                "timestamp": chrono::Utc::now()
            })),
        )
    }
}

/// Liveness check endpoint
pub async fn liveness_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        ResponseJson(json!({
            "status": "alive",
            "timestamp": chrono::Utc::now()
        })),
    )
}

/// Prometheus metrics endpoint
pub async fn prometheus_metrics(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.get_metrics().await {
        Ok(metrics) => {
            // Convert metrics to Prometheus format
            let prometheus_output = format_metrics_as_prometheus(&metrics);
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                prometheus_output,
            )
        },
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                format!("# ERROR: {}", e),
            )
        },
    }
}

/// JSON metrics endpoint
pub async fn json_metrics(State(gateway): State<AppState>) -> impl IntoResponse {
    match gateway.get_metrics().await {
        Ok(metrics) => (StatusCode::OK, ResponseJson(metrics)),
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(json!({
                    "error": e.to_string()
                })),
            )
        },
    }
}

/// MCP request handler
pub async fn mcp_request(
    State(gateway): State<AppState>,
    Json(request): Json<MCPRequest>,
) -> impl IntoResponse {
    debug!("Received MCP request: {}", request.id);

    match gateway.process_request(request).await {
        Ok(response) => (StatusCode::OK, ResponseJson(response)),
        Err(e) => {
            error!("Failed to process MCP request: {}", e);
            let error_response = MCPResponse {
                id: Uuid::new_v4(),
                result: None,
                error: Some(mcp_common::MCPError {
                    code: -1,
                    message: e.to_string(),
                    data: None,
                }),
                timestamp: chrono::Utc::now(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(error_response),
            )
        },
    }
}

/// Batch MCP request handler
pub async fn mcp_batch_request(
    State(gateway): State<AppState>,
    Json(requests): Json<Vec<MCPRequest>>,
) -> impl IntoResponse {
    debug!("Received batch MCP request with {} items", requests.len());

    let mut responses = Vec::new();

    for request in requests {
        match gateway.process_request(request).await {
            Ok(response) => responses.push(response),
            Err(e) => {
                error!("Failed to process request in batch: {}", e);
                responses.push(MCPResponse {
                    id: Uuid::new_v4(),
                    result: None,
                    error: Some(mcp_common::MCPError {
                        code: -1,
                        message: e.to_string(),
                        data: None,
                    }),
                    timestamp: chrono::Utc::now(),
                });
            },
        }
    }

    (StatusCode::OK, ResponseJson(responses))
}

// WebSocket handler temporarily disabled in Generation 1 due to import complexity
// /// WebSocket handler for real-time communication
// pub async fn websocket_handler(
//     ws: WebSocketUpgrade,
//     State(gateway): State<AppState>,
// ) -> impl IntoResponse {
//     ws.on_upgrade(move |socket| handle_websocket(socket, gateway))
// }

// async fn handle_websocket(socket: axum::extract::ws::WebSocket, gateway: AppState) {
// WebSocket implementation will be added in Generation 2
// This placeholder ensures the file compiles without WebSocket dependencies

/// API info endpoint
pub async fn api_info() -> impl IntoResponse {
    (
        StatusCode::OK,
        ResponseJson(json!({
            "name": "MCP WASM Edge Gateway",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Ultra-lightweight Model Context Protocol gateway for edge devices",
            "endpoints": {
                "health": "/health",
                "metrics": "/metrics",
                "mcp": "/mcp/v1/request",
                "websocket": "/ws"
            },
            "documentation": "https://github.com/terragon-labs/mcp-wasm-edge-gateway"
        })),
    )
}

/// Version info endpoint
pub async fn version_info() -> impl IntoResponse {
    (
        StatusCode::OK,
        ResponseJson(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "build_time": env!("VERGEN_BUILD_TIMESTAMP"),
            "git_hash": env!("VERGEN_GIT_SHA"),
            "rust_version": env!("VERGEN_RUSTC_SEMVER"),
            "target": env!("VERGEN_CARGO_TARGET_TRIPLE")
        })),
    )
}

/// Convert metrics to Prometheus format
fn format_metrics_as_prometheus(metrics: &mcp_common::metrics::AggregatedMetrics) -> String {
    let mut output = String::new();

    // System metrics
    output.push_str(&format!(
        "# HELP mcp_cpu_usage_percent CPU usage percentage\n# TYPE mcp_cpu_usage_percent gauge\nmcp_cpu_usage_percent {}\n",
        metrics.system.cpu_usage_percent
    ));

    output.push_str(&format!(
        "# HELP mcp_memory_usage_mb Memory usage in MB\n# TYPE mcp_memory_usage_mb gauge\nmcp_memory_usage_mb {}\n",
        metrics.system.memory_usage_mb
    ));

    // Request metrics
    output.push_str(&format!(
        "# HELP mcp_requests_total Total number of requests\n# TYPE mcp_requests_total counter\nmcp_requests_total {}\n",
        metrics.requests.total_requests
    ));

    output.push_str(&format!(
        "# HELP mcp_requests_successful_total Successful requests\n# TYPE mcp_requests_successful_total counter\nmcp_requests_successful_total {}\n",
        metrics.requests.successful_requests
    ));

    output.push_str(&format!(
        "# HELP mcp_request_latency_ms Average request latency in milliseconds\n# TYPE mcp_request_latency_ms gauge\nmcp_request_latency_ms {}\n",
        metrics.requests.avg_latency_ms
    ));

    // Queue metrics
    output.push_str(&format!(
        "# HELP mcp_queue_size Current queue size\n# TYPE mcp_queue_size gauge\nmcp_queue_size {}\n",
        metrics.queue.queue_size
    ));

    output
}
