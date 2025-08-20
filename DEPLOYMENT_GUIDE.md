# MCP WASM Edge Gateway - Production Deployment Guide

## 🚀 Quick Start

The MCP WASM Edge Gateway is ready for production deployment across multiple edge platforms. This guide covers deployment scenarios from constrained IoT devices to cloud-scale orchestration.

## 📋 Prerequisites

### System Requirements
- **Minimum**: 512MB RAM, 100MB storage, ARM/x86_64 CPU
- **Recommended**: 2GB RAM, 500MB storage, multi-core CPU
- **Optimal**: 4GB+ RAM, 1GB+ storage, GPU acceleration (Jetson)

### Dependencies
```bash
# Core dependencies (automatically handled by deployment)
- Rust 1.75+ (compilation only)
- OpenSSL 1.1+
- libc compatible system
- Optional: TPM 2.0 hardware for attestation
```

## 🌍 Platform-Specific Deployments

### 1. Raspberry Pi Deployment
```bash
# Cross-compile for ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Deploy to Pi
scp target/aarch64-unknown-linux-gnu/release/mcp-gateway pi@raspberrypi.local:/opt/mcp/
ssh pi@raspberrypi.local "sudo systemctl enable mcp-gateway"
```

**Configuration** (`/opt/mcp/config.toml`):
```toml
[gateway]
bind_address = "0.0.0.0"
port = 8080
max_connections = 100
enable_gpu = false

[security]
level = "high"
hardware_attestation = true
rate_limit_per_minute = 60

[edge]
local_models_path = "/opt/mcp/models"
cache_size_mb = 256
offline_mode = true
```

### 2. NVIDIA Jetson Deployment
```bash
# GPU-optimized build
cargo build --release --features gpu-acceleration

# Deploy with CUDA support
docker build -t mcp-edge-jetson -f Dockerfile.jetson .
docker run -d --gpus all --name mcp-gateway \
  -p 8080:8080 -v /opt/mcp:/config mcp-edge-jetson
```

**GPU Configuration**:
```toml
[edge]
gpu_enabled = true
gpu_memory_mb = 2048
tensor_cores = true
mixed_precision = true
```

### 3. ESP32-S3 Deployment (Constrained)
```bash
# Ultra-minimal WASM build
cargo build --release --target wasm32-unknown-unknown \
  --features minimal,no-std --no-default-features

# Deploy to ESP32
espflash write-bin target/wasm32-unknown-unknown/release/mcp-gateway.wasm
```

**Constrained Configuration**:
```toml
[gateway]
max_connections = 10
buffer_size = 1024

[edge]
cache_size_mb = 4
local_models_path = "/spiffs/models"
compression_enabled = true
```

### 4. Mobile App Integration (iOS/Android)
```javascript
// WASM integration
import init, { MCPGateway } from './pkg/mcp_gateway.js';

async function initializeGateway() {
  await init();
  const gateway = new MCPGateway({
    maxConnections: 50,
    offlineFirst: true,
    securityLevel: 'medium'
  });
  
  return gateway;
}
```

### 5. Docker Container Deployment
```dockerfile
# Production Dockerfile
FROM rust:1.75-alpine as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates openssl
COPY --from=builder /build/target/release/mcp-gateway /usr/local/bin/
EXPOSE 8080
CMD ["mcp-gateway"]
```

```bash
# Build and deploy
docker build -t mcp-edge-gateway .
docker run -d -p 8080:8080 \
  -v ./config:/config \
  -v ./models:/models \
  mcp-edge-gateway
```

### 6. Kubernetes Orchestration
```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mcp-edge-gateway
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mcp-gateway
  template:
    metadata:
      labels:
        app: mcp-gateway
    spec:
      containers:
      - name: mcp-gateway
        image: mcp-edge-gateway:latest
        ports:
        - containerPort: 8080
        env:
        - name: MCP_CONFIG_PATH
          value: "/config/production.toml"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: mcp-gateway-service
spec:
  selector:
    app: mcp-gateway
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

## ⚙️ Configuration Management

### Production Configuration Template
```toml
# production.toml
[gateway]
bind_address = "0.0.0.0"
port = 8080
max_connections = 1000
worker_threads = 4
keep_alive_timeout = 60

[security]
level = "critical"
hardware_attestation = true
rate_limit_per_minute = 100
blocked_ips_file = "/config/blocked_ips.txt"
ssl_cert = "/config/server.crt"
ssl_key = "/config/server.key"

[edge]
local_models_path = "/models"
cache_size_mb = 1024
offline_mode = true
sync_interval_minutes = 15
compression_level = 6

[telemetry]
enabled = true
metrics_endpoint = "http://prometheus:9090"
log_level = "info"
trace_sampling = 0.1

[auto_scaling]
enabled = true
min_instances = 2
max_instances = 20
cpu_threshold = 70.0
memory_threshold = 80.0
scale_up_cooldown = 300
scale_down_cooldown = 600
```

### Environment Variables
```bash
# Required
export MCP_CONFIG_PATH="/config/production.toml"
export MCP_LOG_LEVEL="info"

# Optional
export MCP_BIND_ADDRESS="0.0.0.0"
export MCP_PORT="8080"
export MCP_SECURITY_LEVEL="high"
export MCP_ENABLE_GPU="true"
export MCP_CACHE_SIZE_MB="1024"
```

## 🛡️ Security Hardening

### 1. TLS/SSL Configuration
```bash
# Generate production certificates
openssl req -x509 -newkey rsa:4096 -keyout server.key \
  -out server.crt -days 365 -nodes
```

### 2. Hardware Security Module (HSM)
```toml
[security]
hsm_enabled = true
hsm_slot_id = 0
key_derivation = "pbkdf2"
encryption_algorithm = "aes-256-gcm"
```

### 3. Network Security
```bash
# Firewall rules
sudo ufw allow 8080/tcp
sudo ufw deny from 192.168.1.100  # Block specific IPs
sudo ufw enable
```

### 4. Container Security
```bash
# Run as non-root user
docker run -d --user 1001:1001 \
  --security-opt=no-new-privileges:true \
  --cap-drop=ALL \
  --cap-add=NET_BIND_SERVICE \
  mcp-edge-gateway
```

## 📊 Monitoring & Observability

### 1. Health Check Endpoints
```bash
# Health status
curl http://localhost:8080/health

# Detailed metrics
curl http://localhost:8080/metrics

# Performance stats
curl http://localhost:8080/v1/stats
```

### 2. Prometheus Integration
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'mcp-gateway'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### 3. Grafana Dashboard
```json
{
  "dashboard": {
    "title": "MCP Edge Gateway",
    "panels": [
      {
        "title": "Response Time",
        "targets": [{"expr": "mcp_response_time_seconds"}]
      },
      {
        "title": "Request Rate", 
        "targets": [{"expr": "rate(mcp_requests_total[5m])"}]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [{"expr": "mcp_cache_hit_ratio"}]
      }
    ]
  }
}
```

### 4. Log Aggregation
```bash
# Fluentd configuration for log collection
docker run -d --log-driver=fluentd \
  --log-opt fluentd-address=fluentd:24224 \
  --log-opt tag=mcp.gateway \
  mcp-edge-gateway
```

## 🔧 Performance Tuning

### 1. CPU Optimization
```toml
[performance]
worker_threads = 8          # Match CPU cores
thread_stack_size = "2MB"
cpu_affinity = [0,1,2,3]
enable_simd = true
```

### 2. Memory Optimization  
```toml
[memory]
heap_size_mb = 2048
stack_size_mb = 8
enable_jemalloc = true
gc_threshold = 0.8
```

### 3. Network Optimization
```toml
[network]
tcp_nodelay = true
tcp_keepalive = true
send_buffer_size = 65536
recv_buffer_size = 65536
max_concurrent_streams = 1000
```

### 4. Disk I/O Optimization
```toml
[storage]
cache_write_buffer = "64MB"
enable_compression = true
sync_mode = "async"
max_open_files = 1000
```

## 🚀 Deployment Automation

### 1. CI/CD Pipeline (GitHub Actions)
```yaml
# .github/workflows/deploy.yml
name: Deploy MCP Gateway
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Build Multi-Platform
      run: |
        cargo build --release --target x86_64-unknown-linux-gnu
        cargo build --release --target aarch64-unknown-linux-gnu
        cargo build --release --target wasm32-unknown-unknown
    - name: Deploy to Edge Devices
      run: ./scripts/deploy-edge.sh
```

### 2. Terraform Infrastructure
```hcl
# infrastructure/main.tf
resource "aws_ecs_cluster" "mcp_cluster" {
  name = "mcp-edge-gateway"
  
  capacity_providers = ["FARGATE", "FARGATE_SPOT"]
  default_capacity_provider_strategy {
    capacity_provider = "FARGATE_SPOT"
    weight = 100
  }
}

resource "aws_ecs_service" "mcp_service" {
  name            = "mcp-gateway"
  cluster         = aws_ecs_cluster.mcp_cluster.id
  task_definition = aws_ecs_task_definition.mcp_task.arn
  desired_count   = 3

  deployment_configuration {
    maximum_percent         = 200
    minimum_healthy_percent = 50
  }
}
```

### 3. Ansible Playbook
```yaml
# playbooks/deploy.yml
- name: Deploy MCP Gateway
  hosts: edge_devices
  become: true
  tasks:
    - name: Upload binary
      copy:
        src: ../target/release/mcp-gateway
        dest: /opt/mcp/mcp-gateway
        mode: '0755'
    
    - name: Update configuration
      template:
        src: production.toml.j2
        dest: /opt/mcp/config.toml
    
    - name: Restart service
      systemd:
        name: mcp-gateway
        state: restarted
        enabled: true
```

## 🔄 Maintenance & Updates

### 1. Rolling Updates
```bash
# Zero-downtime rolling update
kubectl set image deployment/mcp-gateway \
  mcp-gateway=mcp-edge-gateway:v2.0.0
kubectl rollout status deployment/mcp-gateway
```

### 2. Backup & Recovery
```bash
# Backup configuration and models
tar -czf mcp-backup-$(date +%Y%m%d).tar.gz \
  /opt/mcp/config.toml \
  /opt/mcp/models/ \
  /opt/mcp/cache/

# Restore from backup
tar -xzf mcp-backup-20241220.tar.gz -C /opt/mcp/
```

### 3. Performance Monitoring
```bash
# Check performance metrics
curl -s http://localhost:8080/metrics | grep mcp_

# System resource monitoring
top -p $(pgrep mcp-gateway)
iostat -x 1
```

## 📞 Troubleshooting

### Common Issues
1. **High Memory Usage**: Reduce cache_size_mb in configuration
2. **Connection Timeouts**: Increase keep_alive_timeout
3. **SSL Certificate Errors**: Verify certificate validity and paths
4. **Permission Denied**: Check file permissions and user context

### Debug Mode
```bash
# Enable debug logging
export MCP_LOG_LEVEL=debug
export RUST_BACKTRACE=1

# Run with debug symbols
cargo build --release --features debug-symbols
```

---

**🎉 The MCP WASM Edge Gateway is now ready for production deployment across all supported platforms!**

For support, visit: https://github.com/terragon-labs/mcp-gateway/issues