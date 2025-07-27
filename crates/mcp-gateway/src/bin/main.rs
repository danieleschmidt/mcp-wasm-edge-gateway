//! MCP Gateway main executable

use clap::{Arg, Command};
use mcp_common::Config;
use mcp_gateway::{init_gateway, start_server};
use std::path::PathBuf;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_tracing()?;
    
    // Parse command line arguments
    let matches = Command::new("mcp-gateway")
        .version(env!("CARGO_PKG_VERSION"))
        .about("MCP WASM Edge Gateway - Ultra-lightweight AI gateway for edge devices")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .required(false)
        )
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .value_name("ADDRESS")
                .help("Bind address (default: 0.0.0.0:8080)")
                .required(false)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();
    
    // Load configuration
    let config = load_config(matches.get_one::<String>("config"))?;
    
    // Get bind address
    let bind_addr = matches
        .get_one::<String>("bind")
        .map(|s| s.as_str())
        .unwrap_or("0.0.0.0:8080");
    
    info!("Starting MCP Gateway v{}", env!("CARGO_PKG_VERSION"));
    info!("Binding to: {}", bind_addr);
    
    // Initialize and start the gateway
    match init_gateway(config).await {
        Ok(gateway) => {
            info!("Gateway initialized successfully");
            
            // Set up graceful shutdown
            let gateway_clone = gateway.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                info!("Received shutdown signal, gracefully shutting down...");
                if let Err(e) = gateway_clone.shutdown().await {
                    error!("Error during shutdown: {}", e);
                }
                std::process::exit(0);
            });
            
            // Start the server
            if let Err(e) = start_server(gateway, bind_addr).await {
                error!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Failed to initialize gateway: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Set up tracing with environment filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mcp_gateway=info,mcp_router=info,mcp_models=info"));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    Ok(())
}

fn load_config(config_path: Option<&String>) -> Result<Config, Box<dyn std::error::Error>> {
    if let Some(path) = config_path {
        info!("Loading configuration from: {}", path);
        let config_str = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    } else {
        info!("Using default configuration");
        Ok(Config::default())
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    // Initialize WASM-specific components
    console_error_panic_hook::set_once();
    
    wasm_bindgen_futures::spawn_local(async {
        let config = Config::default();
        if let Ok(gateway) = init_gateway(config).await {
            // In WASM, we don't start a traditional server
            // Instead, we expose the gateway through WASM bindings
            info!("WASM Gateway initialized and ready");
        }
    });
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn process_wasm_request(request_json: &str) -> Result<String, JsValue> {
    use mcp_common::MCPRequest;
    
    let request: MCPRequest = serde_json::from_str(request_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse request: {}", e)))?;
    
    // This would need access to the gateway instance
    // For now, return a placeholder response
    let response = serde_json::json!({
        "id": request.id,
        "result": {
            "status": "success",
            "message": "WASM processing not yet implemented"
        },
        "timestamp": chrono::Utc::now()
    });
    
    Ok(response.to_string())
}