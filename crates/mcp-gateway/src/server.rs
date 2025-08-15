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
use tower_http::{cors::CorsLayer, trace::TraceLayer};
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
        // Use the handlers module to create the complete router
        let app = handlers::create_router(self.gateway.clone());
        
        // Add middleware stack
        app.layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()) // TODO: Configure CORS properly
                .layer(middleware::RequestIdLayer::new())
                .layer(middleware::RateLimitLayer::new(100, 60)) // 100 requests per minute
                .layer(middleware::MetricsLayer::new()),
        )
    }
}

/// Server state shared across handlers
pub type AppState = Arc<Gateway>;
