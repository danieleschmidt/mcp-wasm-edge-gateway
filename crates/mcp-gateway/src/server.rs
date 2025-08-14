//! HTTP/WebSocket server implementation

use crate::handlers;
use crate::middleware;
use crate::Gateway;
use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use mcp_common::{Error, Result};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

/// HTTP server wrapper for the gateway
pub struct Server {
    gateway: Arc<Gateway>,
}

impl Server {
    pub fn new(gateway: Gateway) -> Self {
        Self {
            gateway: Arc::new(gateway),
        }
    }

    /// Run the server on the specified address
    pub async fn run(&self, bind_addr: &str) -> Result<()> {
        let app = self.create_app();

        info!("Starting server on {}", bind_addr);

        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|e| Error::Network(format!("Failed to bind to {}: {}", bind_addr, e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| Error::Network(format!("Server error: {}", e)))?;

        Ok(())
    }

    fn create_app(&self) -> Router {
        // Create the router with all routes
        Router::new()
            // Health endpoints
            .route("/health", get(handlers::health_check))
            .route("/health/ready", get(handlers::readiness_check))
            .route("/health/live", get(handlers::liveness_check))
            // Metrics endpoints
            .route("/metrics", get(handlers::prometheus_metrics))
            .route("/metrics/json", get(handlers::json_metrics))
            // MCP endpoints
            .route("/mcp/v1/request", post(handlers::mcp_request))
            .route("/mcp/v1/batch", post(handlers::mcp_batch_request))
            // WebSocket endpoint (temporarily disabled in Generation 1)
            // .route("/ws", get(handlers::websocket_handler))
            // API info
            .route("/", get(handlers::api_info))
            .route("/version", get(handlers::version_info))
            // Add middleware stack
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(CorsLayer::permissive()) // TODO: Configure CORS properly
                    .layer(middleware::RequestIdLayer::new())
                    .layer(middleware::RateLimitLayer::new(100, 60)) // 100 requests per minute
                    .layer(middleware::MetricsLayer::new()),
            )
            .with_state(self.gateway.clone())
    }
}

/// Server state shared across handlers
pub type AppState = Arc<Gateway>;
