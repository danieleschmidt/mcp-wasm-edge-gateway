# Development Environment Setup

This document provides comprehensive instructions for setting up the MCP WASM Edge Gateway development environment.

## 🚀 Quick Setup (Recommended)

### Option 1: VS Code Devcontainer (Fastest)

```bash
# 1. Clone the repository
git clone https://github.com/your-org/mcp-wasm-edge-gateway
cd mcp-wasm-edge-gateway

# 2. Open in VS Code
code .

# 3. When prompted, click "Reopen in Container"
# All dependencies will be automatically installed
```

### Option 2: Local Development

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Install required targets and tools
rustup target add wasm32-wasi wasm32-unknown-unknown
cargo install wasm-pack just cargo-watch

# 3. Install Node.js (for web tooling)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# 4. Install project dependencies
npm run setup
```

## 📋 Prerequisites

### System Requirements

- **Operating System**: Linux, macOS, or Windows with WSL2
- **Memory**: 8GB RAM minimum, 16GB recommended
- **Storage**: 10GB free space for toolchain and dependencies
- **Network**: Internet connection for initial setup

### Required Software

#### Core Dependencies
- **Rust**: 1.70+ with `rustfmt`, `clippy`, and WASM targets
- **Node.js**: 18+ (for web tooling and package management)
- **Git**: Latest version for source control

#### Development Tools
- **wasm-pack**: WASM package builder
- **just**: Command runner (alternative to make)
- **cargo-watch**: File watcher for development

#### Optional but Recommended
- **Docker**: For containerized development and deployment
- **VS Code**: With Rust analyzer and recommended extensions
- **LLDB**: For debugging (included in VS Code extension)

## 🔧 Detailed Installation

### 1. Rust Toolchain Setup

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Configure the current shell
source ~/.cargo/env

# Update to latest version
rustup update

# Install required components
rustup component add rustfmt clippy rust-src llvm-tools-preview

# Add WASM compilation targets
rustup target add wasm32-wasi wasm32-unknown-unknown

# Verify installation
rustc --version
cargo --version
```

### 2. WASM Tools Installation

```bash
# Install wasm-pack for building WASM packages
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Alternatively, use cargo
cargo install wasm-pack

# Install additional WASM tools
cargo install wasm-bindgen-cli

# Verify installation
wasm-pack --version
wasm-bindgen --version
```

### 3. Development Tools

```bash
# Install just (command runner)
cargo install just

# Install cargo-watch for file watching
cargo install cargo-watch

# Install additional development tools
cargo install \
  cargo-audit \
  cargo-outdated \
  cargo-bloat \
  cargo-expand \
  cargo-tree \
  cargo-udeps

# Install WASM analysis tools
cargo install twiggy

# Optional: Install flamegraph for profiling
cargo install flamegraph
```

### 4. Node.js and Web Tools

```bash
# Install Node.js 20+ (Ubuntu/Debian)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# For macOS with Homebrew
brew install node

# For Windows, download from nodejs.org

# Verify installation
node --version
npm --version

# Install global tools
npm install -g wasm-pack
```

### 5. Cross-Compilation Setup (Optional)

For building binaries for different platforms:

```bash
# Install cross for cross-compilation
cargo install cross

# Add additional targets
rustup target add \
  aarch64-unknown-linux-gnu \
  armv7-unknown-linux-gnueabihf \
  x86_64-pc-windows-gnu \
  aarch64-apple-darwin

# For ESP32 development (advanced)
# Follow the ESP-IDF setup guide
```

## 🏗️ Project Setup

### 1. Clone and Initialize

```bash
# Clone the repository
git clone https://github.com/your-org/mcp-wasm-edge-gateway
cd mcp-wasm-edge-gateway

# Install Node.js dependencies
npm install

# Run the setup script (installs Rust tools and targets)
npm run setup

# Verify everything is working
cargo check --workspace
```

### 2. Environment Configuration

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your configuration
# Set up paths, API keys, and development settings
```

### 3. Build and Test

```bash
# Build all targets
npm run build:all

# Run tests
npm run test:all

# Start development server with file watching
npm run dev
```

## 🔍 IDE Configuration

### VS Code (Recommended)

#### Automatic Setup
Open the workspace file for automatic configuration:
```bash
code mcp-wasm-edge-gateway.code-workspace
```

#### Manual Setup
1. Install recommended extensions:
   - Rust Analyzer
   - CodeLLDB (for debugging)
   - Even Better TOML
   - WASM DWARF Debugging

2. Copy recommended settings:
```bash
mkdir -p .vscode
cp .vscode.recommended/settings.json .vscode/settings.json
```

### Other IDEs

#### IntelliJ IDEA / CLion
- Install Rust plugin
- Configure Rust toolchain path
- Set up run configurations for debugging

#### Vim/Neovim
- Install rust.vim or rust-tools.nvim
- Configure LSP with rust-analyzer
- Set up debugging with nvim-dap

## 🧪 Development Workflow

### Daily Development

```bash
# Start development server with auto-reload
just dev

# Or use npm script
npm run dev

# In another terminal, run tests in watch mode
cargo watch -x test

# Run linter
npm run lint

# Format code
npm run format
```

### Building for Different Targets

```bash
# Native debug build
cargo build

# Native release build
cargo build --release

# WASM for web
npm run build:wasm

# WASM for Node.js
npm run build:wasm-node

# Cross-compile for Raspberry Pi
npm run cross:rpi

# Build all targets
npm run build:all
```

### Testing Strategy

```bash
# Unit tests
cargo test --workspace

# Integration tests
npm run test:integration

# WASM tests
npm run test:wasm

# Performance tests
npm run test:performance

# All tests
npm run test:all
```

## 🐛 Debugging Setup

### Native Debugging (VS Code)

1. Set breakpoints in your Rust code
2. Press `F5` or use "Debug MCP Gateway" configuration
3. Use the debug console for inspection

### WASM Debugging

For WASM debugging in the browser:
1. Build with debug symbols: `wasm-pack build --dev`
2. Use browser developer tools
3. Install WASM debugging extension for enhanced experience

### Command Line Debugging

```bash
# Debug with LLDB
cargo build
lldb target/debug/mcp-gateway

# Debug with GDB
cargo build
gdb target/debug/mcp-gateway

# Run with Valgrind (Linux)
cargo build
valgrind --tool=memcheck target/debug/mcp-gateway
```

## 📊 Performance Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile the application
npm run flamegraph

# Or manually
cargo flamegraph --bin mcp-gateway
```

### Memory Profiling

```bash
# Use Valgrind (Linux)
valgrind --tool=massif target/release/mcp-gateway

# Use heaptrack (Linux)
heaptrack target/release/mcp-gateway

# Use instruments (macOS)
instruments -t "Allocations" target/release/mcp-gateway
```

### WASM Size Analysis

```bash
# Analyze WASM binary size
npm run size

# Detailed size breakdown
npm run size:detailed

# Check for bloat
npm run bloat
```

## 🔒 Security Development

### Security Scanning

```bash
# Audit dependencies
cargo audit

# Check for outdated dependencies
cargo outdated

# Security scan
npm run security
```

### Secure Development Practices

1. **Dependencies**: Regularly update and audit dependencies
2. **Secrets**: Never commit secrets; use environment variables
3. **Input Validation**: Validate all external inputs
4. **Error Handling**: Don't leak sensitive information in errors

## 🚀 Deployment Preparation

### Production Build

```bash
# Build optimized release
cargo build --release --target wasm32-wasi

# Build with link-time optimization
RUSTFLAGS="-C lto=fat" cargo build --release

# Strip debug symbols
strip target/release/mcp-gateway
```

### Docker Development

```bash
# Build development Docker image
docker build -f Dockerfile.dev -t mcp-gateway-dev .

# Run in container
docker run -p 8080:8080 mcp-gateway-dev

# Use docker-compose for full stack
docker-compose up -d
```

## 🔧 Troubleshooting

### Common Issues

#### Rust compilation errors
```bash
# Update Rust toolchain
rustup update

# Clean build artifacts
cargo clean

# Rebuild dependencies
cargo build
```

#### WASM build failures
```bash
# Update wasm-pack
cargo install wasm-pack --force

# Clear WASM cache
rm -rf pkg pkg-node

# Rebuild
npm run build:wasm
```

#### Permission errors (Linux/macOS)
```bash
# Fix cargo permissions
sudo chown -R $USER:$USER ~/.cargo

# Fix target directory permissions
sudo chown -R $USER:$USER target
```

### Performance Issues

#### Slow compilation
```bash
# Use parallel compilation
export CARGO_BUILD_JOBS=8

# Use faster linker (Linux)
sudo apt install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
```

#### Large binary size
```bash
# Enable optimization for dependencies
[profile.release]
lto = true
codegen-units = 1
panic = "abort"

# Use wee_alloc for WASM
[dependencies]
wee_alloc = "0.4"
```

## 📞 Getting Help

### Resources
- **Documentation**: Check `docs/` directory
- **Examples**: See `examples/` directory
- **Issues**: [GitHub Issues](https://github.com/your-org/mcp-wasm-edge-gateway/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/mcp-wasm-edge-gateway/discussions)

### Community
- **Discord**: [Join our Discord](https://discord.gg/your-server)
- **Reddit**: r/rust, r/WebAssembly
- **Stack Overflow**: Use tags `rust`, `webassembly`, `mcp`

---

**Happy Coding! 🦀**

For more advanced topics, see our [Advanced Development Guide](./ADVANCED_DEVELOPMENT.md).