# Multi-stage Dockerfile for MCP WASM Edge Gateway
# Optimized for size and security

# Build stage
FROM rust:1.88-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 appuser

# Set working directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build dependencies first (for better caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --bin mcp-gateway
RUN rm -rf src target/release/deps/mcp_gateway*

# Copy source code
COPY src/ ./src/
COPY tests/ ./tests/

# Build the application
RUN cargo build --release --bin mcp-gateway

# Strip the binary to reduce size
RUN strip target/release/mcp-gateway

# Runtime stage - distroless for security
FROM gcr.io/distroless/cc-debian12:nonroot

# Copy the binary from builder stage
COPY --from=builder /app/target/release/mcp-gateway /usr/local/bin/mcp-gateway

# Copy default configuration
COPY --from=builder /app/config/production.toml /etc/mcp/config.toml

# Create directories for data
USER nonroot:nonroot
WORKDIR /app

# Create data directories
RUN mkdir -p /app/data /app/logs /app/cache

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/mcp-gateway", "--health-check"]

# Expose ports
EXPOSE 8080 9090

# Set environment variables
ENV RUST_LOG=info
ENV MCP_CONFIG_PATH=/etc/mcp/config.toml
ENV MCP_DATA_PATH=/app/data
ENV MCP_LOG_PATH=/app/logs

# Runtime command
ENTRYPOINT ["/usr/local/bin/mcp-gateway"]
CMD ["--config", "/etc/mcp/config.toml"]

# Metadata
LABEL maintainer="Terragon Labs <dev@terragon.ai>"
LABEL description="Ultra-lightweight MCP gateway for edge devices"
LABEL version="0.1.0"
LABEL org.opencontainers.image.source="https://github.com/terragon-labs/mcp-wasm-edge-gateway"
LABEL org.opencontainers.image.documentation="https://docs.terragon.ai/mcp-edge"
LABEL org.opencontainers.image.licenses="Apache-2.0"