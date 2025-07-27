#!/bin/bash
set -e

echo "🚀 Setting up MCP WASM Edge Gateway development environment..."

# Update system packages
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    lldb \
    cmake \
    protobuf-compiler \
    sqlite3 \
    libsqlite3-dev

# Install Rust toolchains and targets
echo "📦 Installing Rust toolchains and targets..."
rustup default stable
rustup update

# WASM targets
rustup target add wasm32-unknown-unknown
rustup target add wasm32-wasi

# Cross-compilation targets for edge devices
rustup target add aarch64-unknown-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf
rustup target add x86_64-pc-windows-gnu

# Embedded targets
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imc-unknown-none-elf

# Install essential Rust tools
echo "🔧 Installing Rust development tools..."
cargo install --locked \
    cargo-watch \
    cargo-edit \
    cargo-audit \
    cargo-deny \
    cargo-outdated \
    cargo-tree \
    cargo-expand \
    cargo-bloat \
    wasm-pack \
    basic-http-server

# Install WASM tools
echo "🌐 Installing WebAssembly tools..."
curl https://wasmtime.dev/install.sh -sSf | bash
source ~/.bashrc

# Install Node.js tools for testing WASM
echo "📦 Installing Node.js tools..."
npm install -g \
    @wasmer/cli \
    wasm-opt \
    terser

# Install cross-compilation toolchains
echo "🔗 Installing cross-compilation tools..."
sudo apt-get install -y \
    gcc-aarch64-linux-gnu \
    gcc-arm-linux-gnueabihf \
    gcc-mingw-w64

# Create .cargo/config.toml for cross-compilation
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml << 'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

[build]
rustc-wrapper = "sccache"

[net]
git-fetch-with-cli = true
EOF

# Install sccache for faster compilation
echo "⚡ Installing compilation cache..."
cargo install sccache
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc

# Set up git configuration
echo "🔐 Setting up git configuration..."
git config --global init.defaultBranch main
git config --global pull.rebase false
git config --global core.autocrlf input

echo "✅ Development environment setup complete!"
echo "🔄 Please reload your shell or restart the container to use all tools."