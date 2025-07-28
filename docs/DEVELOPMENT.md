# Development Guide

## Quick Start

```bash
# Clone repository
git clone <repo-url>
cd mcp-wasm-edge-gateway

# Install Rust toolchain
rustup target add wasm32-wasi wasm32-unknown-unknown

# Setup development environment
npm run install:tools
pre-commit install

# Build and test
cargo check --workspace
cargo test --workspace
npm run build:wasm
```

## Development Environment

- Rust 1.75+ with rustfmt/clippy
- Node.js 18+ for web tooling
- Docker for containerized development
- Git with pre-commit hooks

## Reference Documentation

- [Contributing Guide](../CONTRIBUTING.md) - Full development process
- [Architecture Guide](../ARCHITECTURE.md) - System design
- [Security Policy](../SECURITY.md) - Security guidelines
- [Rust Book](https://doc.rust-lang.org/book/) - Rust language reference
- [WASM Guide](https://rustwasm.github.io/book/) - WebAssembly with Rust

## Common Commands

```bash
npm run ci          # Full CI pipeline
cargo fmt           # Format code
cargo clippy        # Lint code  
cargo test          # Run tests
cargo bench         # Performance benchmarks
```