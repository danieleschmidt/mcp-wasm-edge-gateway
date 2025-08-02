# Getting Started with MCP WASM Edge Gateway

Welcome to the MCP WASM Edge Gateway! This guide will help you get up and running quickly with our ultra-lightweight AI inference solution for edge devices.

## 🚀 Quick Start (5 minutes)

### Prerequisites

- **Rust**: Version 1.70+ with WASM target support
- **Docker**: For containerized development (optional)
- **Git**: For version control

### 1. Clone and Setup

```bash
# Clone the repository
git clone https://github.com/your-org/mcp-wasm-edge-gateway
cd mcp-wasm-edge-gateway

# Install Rust WASM targets
rustup target add wasm32-wasi
rustup target add wasm32-unknown-unknown

# Install additional tools
cargo install wasm-pack
cargo install just  # Task runner
```

### 2. Build the Gateway

```bash
# Build for your current platform
cargo build --release

# Build WASM version
just build-wasm

# Build for specific target (e.g., Raspberry Pi)
just build-arm64
```

### 3. Run Basic Example

```bash
# Start the gateway with default configuration
./target/release/mcp-gateway

# Or using Docker
docker run -p 8080:8080 mcp-edge-gateway:latest
```

### 4. Test the Gateway

```bash
# Health check
curl http://localhost:8080/health

# Simple MCP request
curl -X POST http://localhost:8080/v1/mcp/completions \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Hello, world!"}],
    "temperature": 0.7
  }'
```

## 📋 Platform-Specific Guides

### Raspberry Pi Setup

```bash
# On your Raspberry Pi (Raspberry Pi OS)
sudo apt-get update
sudo apt-get install -y libssl-dev

# Download pre-built binary
wget https://github.com/your-org/mcp-wasm-edge-gateway/releases/latest/download/mcp-gateway-arm64
chmod +x mcp-gateway-arm64
sudo mv mcp-gateway-arm64 /usr/local/bin/mcp-gateway

# Create configuration
sudo mkdir -p /etc/mcp
sudo tee /etc/mcp/config.toml > /dev/null <<EOF
[gateway]
bind_address = "0.0.0.0:8080"
max_connections = 50

[models]
local_model = "phi-3-mini-q4"
max_memory_mb = 512

[offline]
queue_size = 500
persistence_path = "/var/lib/mcp/queue"
EOF

# Install as systemd service
sudo tee /etc/systemd/system/mcp-gateway.service > /dev/null <<EOF
[Unit]
Description=MCP WASM Edge Gateway
After=network.target

[Service]
Type=simple
User=pi
ExecStart=/usr/local/bin/mcp-gateway --config /etc/mcp/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable mcp-gateway
sudo systemctl start mcp-gateway
```

### ESP32/Arduino Setup

```cpp
// Arduino IDE: Install "MCP Edge Gateway" library
#include <MCPEdgeGateway.h>
#include <WiFi.h>

const char* ssid = "your-wifi";
const char* password = "your-password";

MCPGateway gateway;

void setup() {
    Serial.begin(115200);
    
    // Connect to WiFi
    WiFi.begin(ssid, password);
    while (WiFi.status() != WL_CONNECTED) {
        delay(1000);
        Serial.println("Connecting to WiFi...");
    }
    
    // Configure gateway for ESP32
    GatewayConfig config;
    config.localModel = "tinyllama-q4";
    config.maxMemoryMB = 100;  // ESP32 memory limit
    config.enableCompression = true;
    config.powerProfile = ULTRA_LOW_POWER;
    
    // Initialize gateway
    if (gateway.begin(config)) {
        Serial.println("MCP Gateway initialized successfully");
        Serial.print("IP address: ");
        Serial.println(WiFi.localIP());
    } else {
        Serial.println("Failed to initialize MCP Gateway");
    }
}

void loop() {
    // Process incoming requests
    gateway.handleRequests();
    
    // Your application logic here
    if (Serial.available()) {
        String input = Serial.readString();
        MCPResponse response = gateway.processText(input);
        Serial.println(response.text);
    }
    
    delay(10);  // Small delay for other tasks
}
```

### Docker Deployment

```bash
# Basic deployment
docker run -d \
  --name mcp-gateway \
  -p 8080:8080 \
  -v $(pwd)/config:/etc/mcp \
  mcp-edge-gateway:latest

# With persistent storage and monitoring
docker-compose up -d
```

### WASM/Browser Integration

```javascript
// Install via npm
npm install @mcp/edge-gateway-wasm

// Basic usage in browser
import init, { MCPGateway } from '@mcp/edge-gateway-wasm';

async function initializeGateway() {
    await init();
    
    const gateway = new MCPGateway({
        localModel: 'phi-3-mini-q4',
        maxMemoryMB: 256,
        enableSIMD: true
    });
    
    return gateway;
}

// Process requests
const gateway = await initializeGateway();
const response = await gateway.complete({
    messages: [{ role: 'user', content: 'Hello!' }],
    temperature: 0.7
});

console.log(response.text);
```

## ⚙️ Configuration

### Basic Configuration File

Create `mcp-config.toml`:

```toml
[gateway]
# Network settings
bind_address = "0.0.0.0:8080"
max_connections = 100
request_timeout_ms = 5000

[models]
# Local model configuration
local_model = "phi-3-mini-q4"
model_path = "./models/"
max_memory_mb = 512
max_tokens = 1024

# Cloud fallback
cloud_endpoint = "https://api.openai.com/v1/chat/completions"
cloud_api_key = "${OPENAI_API_KEY}"  # Use environment variable
fallback_threshold_ms = 2000

[offline]
# Offline-first configuration
queue_size = 1000
persistence_path = "./data/queue"
sync_interval_seconds = 300
compression = "zstd"

[security]
# Security settings
use_tls = true
tls_cert = "./certs/cert.pem"
tls_key = "./certs/key.pem"

[telemetry]
# Monitoring
enabled = true
export_interval_seconds = 60
endpoint = "http://localhost:9090/metrics"
```

### Environment Variables

```bash
# Required
export MCP_CONFIG_PATH="./mcp-config.toml"

# Optional cloud integration
export OPENAI_API_KEY="your-api-key"
export ANTHROPIC_API_KEY="your-api-key"

# Security
export MCP_TLS_CERT_PATH="./certs/cert.pem"
export MCP_TLS_KEY_PATH="./certs/key.pem"

# Monitoring
export MCP_METRICS_ENDPOINT="http://localhost:9090/metrics"
export MCP_LOG_LEVEL="info"
```

## 🔧 Development Setup

### Local Development Environment

```bash
# Option 1: VS Code with devcontainer (recommended)
code .  # Will prompt to reopen in container

# Option 2: Manual setup
rustup target add wasm32-wasi wasm32-unknown-unknown
cargo install wasm-pack just cargo-watch

# Install development dependencies
just setup-dev

# Run in development mode with auto-reload
just dev
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --package mcp-gateway
cargo test --package mcp-models

# Run integration tests
cargo test --test integration

# Run WASM tests
wasm-pack test --node
```

### Building for Different Platforms

```bash
# Native (current platform)
just build

# WASM for web
just build-wasm-web

# WASM for Node.js
just build-wasm-node

# Cross-compilation
just build-arm64      # Raspberry Pi, ARM servers
just build-armv7      # Older ARM devices
just build-x86_64     # Intel/AMD
just build-riscv64    # RISC-V devices

# All platforms
just build-all
```

## 📊 Monitoring and Debugging

### Health Checks

```bash
# Basic health check
curl http://localhost:8080/health

# Detailed health information
curl http://localhost:8080/health/detailed

# Metrics endpoint
curl http://localhost:8080/metrics
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG="mcp_gateway=debug"
./target/release/mcp-gateway

# Or via configuration
[logging]
level = "debug"
format = "json"
```

### Performance Monitoring

```bash
# Enable performance profiling
just profile

# Memory usage analysis
just memory-profile

# CPU usage monitoring
just cpu-profile
```

## 🚨 Troubleshooting

### Common Issues

#### "Model not found" Error
```bash
# Check model path
ls -la ./models/
# Download models if needed
just download-models
```

#### Memory Issues on ESP32
```cpp
// Reduce model size
config.localModel = "tinyllama-q4";  // Smaller model
config.maxMemoryMB = 64;            // Reduce memory limit
config.enablePSRAM = true;          // Use external PSRAM
```

#### Performance Issues
```bash
# Check system resources
just system-info

# Enable optimizations
export RUSTFLAGS="-C target-cpu=native"
cargo build --release
```

#### Connection Issues
```bash
# Check network connectivity
curl -v http://localhost:8080/health

# Verify configuration
just check-config

# Check logs
just logs
```

### Getting Help

- **Documentation**: [Full documentation](https://docs.your-org.com/mcp-edge)
- **Examples**: Check the `examples/` directory
- **Issues**: [GitHub Issues](https://github.com/your-org/mcp-wasm-edge-gateway/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/mcp-wasm-edge-gateway/discussions)
- **Discord**: [Community Discord](https://discord.gg/your-org)

## 🎯 Next Steps

1. **Explore Examples**: Check out `examples/` for specific use cases
2. **Read the Architecture**: Understand the system design in `ARCHITECTURE.md`
3. **Contribute**: See `CONTRIBUTING.md` for development guidelines
4. **Deploy to Production**: Follow our production deployment guide
5. **Join the Community**: Connect with other developers

## 📚 Additional Resources

- [API Reference](./docs/api/)
- [Configuration Guide](./docs/configuration/)
- [Performance Tuning](./docs/performance/)
- [Security Best Practices](./docs/security/)
- [Hardware Compatibility](./docs/hardware/)

---

**Welcome to the MCP WASM Edge Gateway community! 🎉**

Start building amazing edge AI applications today. If you run into any issues, don't hesitate to reach out to our community for help.