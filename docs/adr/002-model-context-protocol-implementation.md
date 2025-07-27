# ADR-002: Model Context Protocol (MCP) Implementation Strategy

## Status
Accepted

## Date
2025-01-27

## Context
The gateway must implement the Model Context Protocol (MCP) to enable standardized communication between AI models and applications. MCP provides a unified interface for tool use, context sharing, and model interactions. Our implementation must be:

- Fully compliant with MCP specification
- Optimized for edge device constraints
- Extensible for custom tools and contexts
- Compatible with existing MCP ecosystems

## Decision
We will implement a **full MCP server and client** with edge-optimized extensions and intelligent request routing between local and cloud processing.

## MCP Protocol Overview

### Core Components
1. **Transport Layer**: JSON-RPC over WebSocket/HTTP
2. **Resource Management**: Context and file system access
3. **Tool System**: Function calling and execution
4. **Prompt Templates**: Reusable prompt structures
5. **Sampling**: Model parameter configuration

### Edge-Specific Extensions
- **Offline Queue**: Persistent request storage during disconnection
- **Resource Constraints**: Memory and CPU usage limits
- **Security Context**: Hardware-backed authentication
- **Telemetry**: Performance and usage metrics

## Implementation Architecture

### Core MCP Server
```rust
pub struct MCPServer {
    transport: Arc<dyn Transport>,
    resource_manager: Arc<ResourceManager>,
    tool_registry: Arc<ToolRegistry>,
    prompt_manager: Arc<PromptManager>,
    model_router: Arc<ModelRouter>,
}

// MCP message handling
#[async_trait]
impl MCPServer {
    async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        match request.method {
            "resources/list" => self.list_resources().await,
            "resources/read" => self.read_resource(request.params).await,
            "tools/list" => self.list_tools().await,
            "tools/call" => self.call_tool(request.params).await,
            "completion/complete" => self.complete(request.params).await,
            "sampling/createMessage" => self.create_message(request.params).await,
            _ => MCPResponse::error("method_not_found"),
        }
    }
}
```

### Edge-Optimized Transport
```rust
// Hybrid transport for edge environments
pub enum EdgeTransport {
    WebSocket(WebSocketTransport),
    HTTP(HttpTransport),
    LocalIPC(IPCTransport),
    SerialPort(SerialTransport), // For microcontrollers
}

impl EdgeTransport {
    pub async fn send_with_fallback(&self, message: MCPMessage) -> Result<MCPResponse> {
        match self {
            Self::WebSocket(ws) => {
                match ws.send(message).await {
                    Ok(response) => Ok(response),
                    Err(_) => self.queue_for_retry(message).await,
                }
            }
            Self::HTTP(http) => http.send_batch(vec![message]).await,
            // Handle other transports...
        }
    }
}
```

### Resource Management
```rust
pub struct EdgeResourceManager {
    local_resources: HashMap<String, LocalResource>,
    remote_resources: HashMap<String, RemoteResource>,
    cache: LRUCache<String, ResourceContent>,
    security_policy: SecurityPolicy,
}

impl ResourceManager for EdgeResourceManager {
    async fn list_resources(&self) -> Vec<Resource> {
        let mut resources = self.local_resources.values().cloned().collect();
        
        // Add cached remote resources if available
        if self.is_connected().await {
            resources.extend(self.fetch_remote_resources().await);
        }
        
        resources
    }
    
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        // Check local first
        if let Some(local) = self.local_resources.get(uri) {
            return Ok(local.read().await?);
        }
        
        // Check cache
        if let Some(cached) = self.cache.get(uri) {
            return Ok(cached.clone());
        }
        
        // Fetch remote if connected
        if self.is_connected().await {
            let content = self.fetch_remote_resource(uri).await?;
            self.cache.insert(uri.to_string(), content.clone());
            return Ok(content);
        }
        
        Err(ResourceError::NotAvailable)
    }
}
```

### Tool System
```rust
pub struct EdgeToolRegistry {
    local_tools: HashMap<String, Box<dyn Tool>>,
    remote_tools: HashMap<String, RemoteToolProxy>,
    execution_limits: ExecutionLimits,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn execute(&self, params: serde_json::Value) -> ToolResult;
    fn resource_requirements(&self) -> ResourceRequirements;
}

// Example edge-specific tools
pub struct SensorDataTool {
    sensor_manager: Arc<SensorManager>,
}

#[async_trait]
impl Tool for SensorDataTool {
    fn name(&self) -> &str { "read_sensor" }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let sensor_id: String = serde_json::from_value(params["sensor_id"].clone())?;
        let data = self.sensor_manager.read_sensor(&sensor_id).await?;
        
        ToolResult::success(json!({
            "sensor_id": sensor_id,
            "timestamp": SystemTime::now(),
            "data": data,
        }))
    }
}
```

### Model Router
```rust
pub struct EdgeModelRouter {
    local_models: HashMap<String, Arc<dyn Model>>,
    cloud_endpoints: HashMap<String, CloudEndpoint>,
    routing_policy: RoutingPolicy,
    performance_tracker: PerformanceTracker,
}

impl EdgeModelRouter {
    pub async fn route_completion(&self, request: CompletionRequest) -> CompletionResponse {
        let routing_decision = self.decide_routing(&request).await;
        
        match routing_decision {
            RoutingDecision::Local(model_id) => {
                self.execute_local_completion(model_id, request).await
            }
            RoutingDecision::Cloud(endpoint) => {
                self.execute_cloud_completion(endpoint, request).await
            }
            RoutingDecision::Queue => {
                self.queue_for_later_processing(request).await
            }
        }
    }
    
    async fn decide_routing(&self, request: &CompletionRequest) -> RoutingDecision {
        // Analyze request complexity
        let complexity = self.analyze_complexity(request);
        
        // Check resource availability
        let resources = self.get_available_resources().await;
        
        // Consider performance history
        let performance = self.performance_tracker.get_model_performance().await;
        
        // Apply routing policy
        self.routing_policy.decide(complexity, resources, performance)
    }
}
```

## Edge-Specific Optimizations

### Message Compression
```rust
pub struct CompressedTransport {
    inner: Box<dyn Transport>,
    compressor: Compressor,
}

impl CompressedTransport {
    pub async fn send(&self, message: MCPMessage) -> Result<MCPResponse> {
        let serialized = serde_json::to_vec(&message)?;
        let compressed = self.compressor.compress(&serialized)?;
        
        // Send compressed with content-encoding header
        self.inner.send_bytes(compressed).await
    }
}
```

### Batching and Queuing
```rust
pub struct BatchingTransport {
    inner: Arc<dyn Transport>,
    batch_buffer: Arc<Mutex<Vec<MCPMessage>>>,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchingTransport {
    pub async fn send_with_batching(&self, message: MCPMessage) -> Result<MCPResponse> {
        let mut buffer = self.batch_buffer.lock().await;
        buffer.push(message);
        
        if buffer.len() >= self.batch_size {
            let batch = buffer.drain(..).collect();
            drop(buffer);
            self.send_batch(batch).await
        } else {
            // Set timeout for partial batch
            self.schedule_batch_timeout().await;
            Ok(MCPResponse::queued())
        }
    }
}
```

### Security Integration
```rust
pub struct SecureMCPServer {
    inner: MCPServer,
    attestation: DeviceAttestation,
    crypto: CryptoProvider,
}

impl SecureMCPServer {
    pub async fn handle_secure_request(&self, request: SecureMCPRequest) -> SecureMCPResponse {
        // Verify device attestation
        self.attestation.verify(&request.device_id).await?;
        
        // Decrypt request
        let decrypted = self.crypto.decrypt(&request.encrypted_payload).await?;
        let mcp_request: MCPRequest = serde_json::from_slice(&decrypted)?;
        
        // Process normally
        let response = self.inner.handle_request(mcp_request).await;
        
        // Encrypt response
        let response_bytes = serde_json::to_vec(&response)?;
        let encrypted = self.crypto.encrypt(&response_bytes).await?;
        
        SecureMCPResponse {
            device_id: request.device_id,
            encrypted_payload: encrypted,
            signature: self.crypto.sign(&encrypted).await?,
        }
    }
}
```

## Protocol Extensions

### Edge Resource Types
```json
{
  "resources": [
    {
      "uri": "edge://sensors/temperature",
      "name": "Temperature Sensor",
      "description": "Real-time temperature readings",
      "mimeType": "application/json",
      "capabilities": ["read", "subscribe"]
    },
    {
      "uri": "edge://models/local/phi-3-mini",
      "name": "Local Phi-3 Model",
      "description": "Quantized model for edge inference",
      "mimeType": "application/octet-stream",
      "capabilities": ["inference"]
    }
  ]
}
```

### Edge Tool Schemas
```json
{
  "tools": [
    {
      "name": "sensor_read",
      "description": "Read data from connected sensors",
      "inputSchema": {
        "type": "object",
        "properties": {
          "sensor_id": {"type": "string"},
          "duration_ms": {"type": "number", "default": 1000}
        }
      },
      "resourceRequirements": {
        "memory_mb": 1,
        "cpu_percent": 5,
        "network": false
      }
    }
  ]
}
```

## Implementation Phases

### Phase 1: Core MCP Implementation
- Basic JSON-RPC transport
- Resource management framework
- Tool registry and execution
- Prompt template system

### Phase 2: Edge Optimizations
- Message compression and batching
- Offline queue implementation
- Local model integration
- Resource constraint handling

### Phase 3: Security & Production
- Device attestation integration
- Encrypted communication
- Comprehensive testing
- Performance optimization

## Success Criteria
- [ ] Full MCP protocol compliance
- [ ] < 50ms message processing latency
- [ ] Offline operation for 24+ hours
- [ ] Security audit passing grade
- [ ] Integration with 3+ MCP clients

## Risks & Mitigations
- **Risk**: Protocol complexity increasing binary size
  **Mitigation**: Feature flags for optional MCP capabilities
- **Risk**: Performance overhead of JSON-RPC
  **Mitigation**: Binary protocol fallback for high-throughput scenarios
- **Risk**: Security vulnerabilities in protocol implementation
  **Mitigation**: Comprehensive security review and fuzzing

## Related ADRs
- ADR-001: Rust + WASM architecture decision
- ADR-003: Security architecture (pending)
- ADR-004: Performance optimization strategy (pending)