#!/bin/bash
set -euo pipefail

echo "🚀 Setting up MCP WASM Edge Gateway development environment..."

# Update Rust toolchain
echo "📦 Updating Rust toolchain..."
rustup update

# Install additional Rust components
echo "🔧 Installing Rust components..."
rustup component add \
    rustfmt \
    clippy \
    rust-src \
    llvm-tools-preview

# Verify WASM targets
echo "🎯 Verifying WASM targets..."
rustup target list --installed | grep wasm

# Install pre-commit hooks if .pre-commit-config.yaml exists
if [ -f ".pre-commit-config.yaml" ]; then
    echo "🪝 Installing pre-commit hooks..."
    pip install pre-commit
    pre-commit install
fi

# Create initial Cargo project if it doesn't exist
if [ ! -f "Cargo.toml" ]; then
    echo "📝 Creating initial Cargo workspace..."
    cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "crates/mcp-wasm-edge-gateway",
    "crates/mcp-core",
    "crates/mcp-security",
    "crates/mcp-models",
    "crates/mcp-telemetry",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Terragon Labs <dev@terragon.ai>"]
license = "Apache-2.0"
repository = "https://github.com/terragon-labs/mcp-wasm-edge-gateway"
homepage = "https://terragon.ai/mcp-edge"
documentation = "https://docs.terragon.ai/mcp-edge"

[workspace.dependencies]
# Core dependencies
tokio = { version = "1.35", features = ["rt", "rt-multi-thread", "net", "time", "sync", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# WASM dependencies
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = "0.3"
wasm-bindgen-futures = "0.4"

# Async and networking
reqwest = { version = "0.11", features = ["json", "stream"] }
tungstenite = "0.20"
tokio-tungstenite = "0.20"

# Cryptography
ring = "0.17"
rustls = "0.22"
webpki-roots = "0.25"

# Compression
zstd = "0.13"
lz4_flex = "0.11"

# Serialization
rmp-serde = "1.1"  # MessagePack
bincode = "1.3"

# Metrics and telemetry
prometheus = "0.13"
opentelemetry = "0.21"

# Testing
criterion = "0.5"
proptest = "1.4"
mockall = "0.12"
EOF

    # Create workspace structure
    mkdir -p crates/{mcp-wasm-edge-gateway,mcp-core,mcp-security,mcp-models,mcp-telemetry}/src
    
    # Create placeholder Cargo.toml files for each crate
    for crate in mcp-wasm-edge-gateway mcp-core mcp-security mcp-models mcp-telemetry; do
        cat > "crates/$crate/Cargo.toml" << EOF
[package]
name = "$crate"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
documentation.workspace = true

[dependencies]
serde.workspace = true
anyhow.workspace = true
thiserror.workspace = true
EOF
        echo 'fn main() { println!("Hello from '"$crate"'!"); }' > "crates/$crate/src/main.rs"
    done
fi

# Install Node.js dependencies for web tooling
if [ -f "package.json" ]; then
    echo "📦 Installing Node.js dependencies..."
    npm install
fi

# Verify installation
echo "✅ Verifying installation..."
rustc --version
cargo --version
wasm-pack --version

echo "🎉 Development environment setup complete!"
echo ""
echo "Available commands:"
echo "  cargo build                    - Build the project"
echo "  cargo test                     - Run tests"
echo "  cargo clippy                   - Run linter"
echo "  cargo fmt                      - Format code"
echo "  wasm-pack build                - Build WASM package"
echo "  just --list                    - Show available recipes"
echo ""
echo "Happy coding! 🦀"