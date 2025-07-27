//! Cloud client for forwarding requests to external MCP services

use mcp_common::{Config, Error, MCPRequest, MCPResponse, Result};
use reqwest::{Client, ClientBuilder};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Client for forwarding requests to cloud MCP services
pub struct CloudClient {
    client: Client,
    config: Arc<Config>,
}

impl CloudClient {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(30000)) // 30 second timeout
            .user_agent("MCP-WASM-Edge-Gateway/0.1.0")
            .build()
            .map_err(|e| Error::Network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
        })
    }

    pub async fn forward_request(
        &self,
        request: &MCPRequest,
        endpoint: &str,
    ) -> Result<MCPResponse> {
        debug!(
            "Forwarding request {} to cloud endpoint: {}",
            request.id, endpoint
        );

        // Find the endpoint configuration
        let endpoint_config = self
            .config
            .router
            .cloud_endpoints
            .iter()
            .find(|e| e.url == endpoint)
            .ok_or_else(|| Error::Routing(format!("Unknown endpoint: {}", endpoint)))?;

        // Prepare the request
        let mut req_builder = self
            .client
            .post(&endpoint_config.url)
            .json(request)
            .timeout(Duration::from_millis(endpoint_config.timeout_ms));

        // Add API key if configured
        if let Some(api_key) = &endpoint_config.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        // Send the request
        let response = req_builder
            .send()
            .await
            .map_err(|e| Error::Network(format!("Request failed: {}", e)))?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Network(format!(
                "Cloud request failed with status {}: {}",
                status, error_body
            )));
        }

        // Parse response
        let mcp_response: MCPResponse = response
            .json()
            .await
            .map_err(|e| Error::Network(format!("Failed to parse response: {}", e)))?;

        debug!("Cloud request {} completed successfully", request.id);
        Ok(mcp_response)
    }

    /// Test connectivity to a cloud endpoint
    pub async fn test_endpoint(&self, endpoint: &str) -> bool {
        let endpoint_config = match self
            .config
            .router
            .cloud_endpoints
            .iter()
            .find(|e| e.url == endpoint)
        {
            Some(config) => config,
            None => return false,
        };

        let mut req_builder = self
            .client
            .get(&format!("{}/health", endpoint_config.url))
            .timeout(Duration::from_millis(5000));

        if let Some(api_key) = &endpoint_config.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        match req_builder.send().await {
            Ok(response) => response.status().is_success(),
            Err(e) => {
                warn!("Health check failed for endpoint {}: {}", endpoint, e);
                false
            },
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        debug!("Shutting down cloud client");
        // HTTP client will be dropped automatically
        Ok(())
    }
}
