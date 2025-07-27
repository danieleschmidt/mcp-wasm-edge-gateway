// MCP WASM Edge Gateway Library
// This is a skeleton implementation to demonstrate the structure

//! MCP WASM Edge Gateway
//! 
//! Ultra-lightweight Model Context Protocol gateway for edge devices.
//! Written in Rust, compiled to WASM with SIMD optimizations.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod gateway;
pub mod router;
pub mod models;
pub mod queue;
pub mod security;
pub mod telemetry;
pub mod error;

// Re-exports for convenience
pub use config::Config;
pub use gateway::Gateway;
pub use error::{Result, GatewayError};

// Mock types for compilation - these would be real implementations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub device_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPResponse {
    pub id: String,
    pub status: String,
    pub content: String,
    pub routing_decision: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}