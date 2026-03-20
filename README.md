# mcp-wasm-edge-gateway

Ultra-lightweight MCP (Model Context Protocol) gateway for edge devices.

## Features
- JSON-RPC 2.0 message handling
- Tool registry with dynamic dispatch
- In-memory routing
- Designed for WASM/edge deployment (native build for tests)

## Usage
```rust
use mcp_gateway::McpGateway;
use serde_json::json;

let mut gateway = McpGateway::new();
gateway.register_tool("echo", "Echo input back", Box::new(|params| {
    Ok(params.unwrap_or(json!(null)))
}));

let response = gateway.handle_jsonrpc(r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":{"msg":"hi"}}"#);
```

## Build
```bash
cargo build
cargo test
```

## License
MIT
