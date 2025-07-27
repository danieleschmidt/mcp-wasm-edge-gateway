# Justfile for MCP WASM Edge Gateway
# https://github.com/casey/just

# Default recipe
default:
    @just --list

# Development commands
dev:
    cargo watch -x 'run --bin mcp-gateway'

# Build commands
build:
    cargo build --workspace

build-release:
    cargo build --workspace --release

build-wasm:
    wasm-pack build --target web --out-dir pkg

build-wasm-node:
    wasm-pack build --target nodejs --out-dir pkg-node

build-all: build build-wasm build-wasm-node

# Cross-compilation
build-rpi:
    cross build --target aarch64-unknown-linux-gnu --release

build-esp32:
    cargo build --target xtensa-esp32s3-none-elf --release

build-windows:
    cross build --target x86_64-pc-windows-gnu --release

build-macos:
    cross build --target aarch64-apple-darwin --release

# Testing
test:
    cargo test --workspace

test-wasm:
    wasm-pack test --node

test-integration:
    cargo test --test integration

test-all: test test-wasm test-integration

# Quality assurance
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

lint-fix:
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

audit:
    cargo audit

check: fmt-check lint test audit

# Documentation
docs:
    cargo doc --workspace --no-deps --open

docs-build:
    cargo doc --workspace --no-deps

# Benchmarking and profiling
bench:
    cargo bench --workspace

profile:
    cargo build --release --bin mcp-gateway
    perf record -g target/release/mcp-gateway
    perf report

flamegraph:
    cargo flamegraph --bin mcp-gateway

# Size analysis
bloat:
    cargo bloat --release --crates

wasm-size:
    @echo "WASM file sizes:"
    @ls -la pkg/*.wasm 2>/dev/null || echo "No WASM files found"
    @if [ -f pkg/mcp_wasm_edge_gateway_bg.wasm ]; then \
        echo "Detailed WASM analysis:"; \
        twiggy top pkg/mcp_wasm_edge_gateway_bg.wasm; \
    fi

# Docker commands
docker-build:
    docker build -t mcp-edge-gateway .

docker-run:
    docker run -p 8080:8080 mcp-edge-gateway

docker-compose-up:
    docker-compose up -d

docker-compose-down:
    docker-compose down

# Development environment
setup:
    rustup target add wasm32-wasi wasm32-unknown-unknown
    cargo install wasm-pack wasm-bindgen-cli cargo-watch cargo-audit cargo-outdated cargo-bloat twiggy flamegraph just

setup-cross:
    cargo install cross

install-tools:
    npm run install:tools

# Release management
release-dry:
    cargo release --dry-run

release-patch:
    cargo release patch --execute

release-minor:
    cargo release minor --execute

release-major:
    cargo release major --execute

changelog:
    git-cliff -o CHANGELOG.md

# Security
security-scan:
    cargo audit
    cargo deny check

secrets-scan:
    @echo "Scanning for secrets..."
    @if command -v trufflehog >/dev/null 2>&1; then \
        trufflehog git file://. --only-verified; \
    else \
        echo "trufflehog not installed, skipping secrets scan"; \
    fi

# Maintenance
update-deps:
    cargo update

outdated:
    cargo outdated

clean:
    cargo clean
    rm -rf pkg pkg-node node_modules
    docker system prune -f

clean-all: clean
    rm -rf target

# Configuration validation
validate-config:
    @echo "Validating configuration files..."
    @for file in config/*.toml; do \
        echo "Checking $$file..."; \
        cargo run --bin config-validator -- "$$file" || exit 1; \
    done

# Health checks
health-check:
    @echo "Running health checks..."
    @cargo check --workspace
    @cargo test --workspace --lib
    @echo "All health checks passed!"

# Performance monitoring
perf-test:
    @echo "Running performance tests..."
    @cargo bench | tee benchmark-results.txt
    @echo "Performance results saved to benchmark-results.txt"

# License compliance
license-check:
    cargo license --json > licenses.json
    @echo "License report generated: licenses.json"

# Project metrics
metrics:
    @echo "Project metrics:"
    @echo "Lines of code:"
    @tokei --output json . | jq '.Total.code'
    @echo "Dependencies:"
    @cargo tree --workspace | wc -l
    @echo "Test coverage:"
    @cargo llvm-cov --workspace --summary-only 2>/dev/null || echo "Install cargo-llvm-cov for coverage"

# CI simulation
ci: fmt-check lint test audit wasm-size

# Local development
serve-dev:
    cargo run --bin mcp-gateway -- --config ./config/development.toml

serve-prod:
    cargo run --release --bin mcp-gateway -- --config ./config/production.toml