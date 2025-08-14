//! MCP Gateway - Core gateway component for the MCP WASM Edge Gateway
//!
//! This crate provides the main gateway functionality including request handling,
//! component orchestration, and the REST/WebSocket APIs.

pub mod gateway;
pub mod handlers;
pub mod health;
pub mod middleware;
pub mod server;

pub use gateway::Gateway;
pub use server::{AppState, Server};

use mcp_common::{Error, Result};

/// Initialize the gateway with configuration
pub async fn init_gateway(config: mcp_common::Config) -> Result<Gateway> {
    let gateway = Gateway::new(config).await?;
    Ok(gateway)
}

/// Start the gateway server
pub async fn start_server(gateway: Gateway, bind_addr: &str) -> Result<()> {
    let server = Server::new(gateway);
    server.run(bind_addr).await
}
