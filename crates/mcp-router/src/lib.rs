//! MCP Router - Request routing and load balancing for the MCP Edge Gateway
//!
//! This crate handles intelligent routing of MCP requests based on complexity,
//! resource availability, and performance metrics.

use async_trait::async_trait;
use mcp_common::{Result, Error, MCPRequest, MCPResponse, RoutingDecision, ComponentHealth, Config};
use std::sync::Arc;

/// Router trait for request routing decisions
#[async_trait]
pub trait Router {
    /// Route a request to the appropriate processor
    async fn route(&self, request: &MCPRequest) -> Result<RoutingDecision>;
    
    /// Forward request to cloud endpoint
    async fn forward_to_cloud(&self, request: &MCPRequest, endpoint: &str) -> Result<MCPResponse>;
    
    /// Update performance metrics for routing decisions
    async fn update_metrics(&self, metrics: &mcp_common::PerformanceMetrics) -> Result<()>;
    
    /// Get health status
    async fn health_check(&self) -> Result<ComponentHealth>;
    
    /// Shutdown the router
    async fn shutdown(&self) -> Result<()>;
}

mod intelligent_router;
mod load_balancer;
mod cloud_client;

pub use intelligent_router::IntelligentRouter;

/// Create a new router instance
pub async fn create_router(config: Arc<Config>) -> Result<Arc<dyn Router + Send + Sync>> {
    let router = IntelligentRouter::new(config).await?;
    Ok(Arc::new(router))
}