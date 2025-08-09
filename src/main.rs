use mcp_common::Config;
use mcp_gateway::Gateway;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting MCP WASM Edge Gateway v0.1.0");

    // Load configuration
    let config = Arc::new(Config::default());
    
    info!("Loaded configuration: bind_address={}:{}", 
          config.gateway.bind_address, config.gateway.port);

    // Initialize gateway
    let gateway = match Gateway::new(config.clone()).await {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to initialize gateway: {}", e);
            return Err(anyhow::anyhow!("Gateway initialization failed: {}", e));
        }
    };

    info!("Gateway initialized successfully");

    // Start the server
    let bind_addr = format!("{}:{}", config.gateway.bind_address, config.gateway.port);
    
    info!("Starting server on {}", bind_addr);
    
    match mcp_gateway::start_server(gateway, &bind_addr).await {
        Ok(_) => {
            info!("Server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(anyhow::anyhow!("Server failed: {}", e))
        }
    }
}