# Multi-stage Dockerfile for MCP WASM Edge Gateway
# Optimized for size and security

# Build stage
FROM rust:1.75-bullseye as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install cross-compilation targets
RUN rustup target add wasm32-unknown-unknown \
    && rustup target add wasm32-wasi \
    && rustup target add aarch64-unknown-linux-gnu

# Install WASM tools
RUN cargo install wasm-pack

# Create app user
RUN groupadd -r mcp && useradd -r -g mcp mcp

# Set working directory
WORKDIR /usr/src/app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ .cargo/

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && \
    rm -rf src/

# Copy actual source code
COPY src/ src/
COPY tests/ tests/
COPY benches/ benches/
COPY examples/ examples/

# Build the application
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --bin gateway

# Build WASM version
RUN wasm-pack build --target web --out-dir pkg-web && \
    wasm-pack build --target nodejs --out-dir pkg-node

# Runtime stage - Use distroless for security
FROM gcr.io/distroless/cc-debian11:latest

# Copy user from builder
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

# Copy binary
COPY --from=builder /usr/src/app/target/release/gateway /usr/local/bin/gateway

# Copy WASM packages
COPY --from=builder /usr/src/app/pkg-web /opt/mcp/wasm/web/
COPY --from=builder /usr/src/app/pkg-node /opt/mcp/wasm/node/

# Create directories
RUN mkdir -p /var/lib/mcp/queue /etc/mcp /opt/models

# Set ownership
RUN chown -R mcp:mcp /var/lib/mcp /etc/mcp /opt/models

# Switch to non-root user
USER mcp

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/gateway", "health", "--endpoint", "http://localhost:8080/health"]

# Default command
CMD ["/usr/local/bin/gateway", "serve"]

# Metadata
LABEL org.opencontainers.image.title="MCP WASM Edge Gateway"
LABEL org.opencontainers.image.description="Ultra-lightweight Model Context Protocol gateway for edge devices"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.vendor="Terragon Labs"
LABEL org.opencontainers.image.source="https://github.com/your-org/mcp-wasm-edge-gateway"
LABEL org.opencontainers.image.documentation="https://github.com/your-org/mcp-wasm-edge-gateway/blob/main/README.md"
LABEL org.opencontainers.image.licenses="MIT"