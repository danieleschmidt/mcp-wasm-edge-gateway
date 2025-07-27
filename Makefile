# Makefile for MCP WASM Edge Gateway
# Provides standardized build, test, and deployment commands

.PHONY: help build test clean install dev docker wasm cross lint format audit docs release

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
DOCKER := docker
DOCKER_COMPOSE := docker-compose
PROJECT_NAME := mcp-wasm-edge-gateway
VERSION := $(shell grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
REGISTRY ?= ghcr.io/your-org
IMAGE_NAME := $(REGISTRY)/$(PROJECT_NAME)

# Colors for output
YELLOW := \033[33m
GREEN := \033[32m
RED := \033[31m
RESET := \033[0m

## Help: Show this help message
help:
	@echo "$(YELLOW)MCP WASM Edge Gateway - Build System$(RESET)"
	@echo ""
	@echo "$(GREEN)Available targets:$(RESET)"
	@awk 'BEGIN {FS = ":.*##"; printf ""} /^[a-zA-Z_-]+:.*##/ { printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(GREEN)Available profiles:$(RESET)"
	@echo "  $(GREEN)development$(RESET)     Full development environment"
	@echo "  $(GREEN)testing$(RESET)         Testing environment with test database"
	@echo "  $(GREEN)production$(RESET)      Production-ready deployment"

## Build: Build the project in release mode
build:
	@echo "$(YELLOW)Building MCP Gateway...$(RESET)"
	$(CARGO) build --release
	@echo "$(GREEN)Build completed!$(RESET)"

## Build Debug: Build the project in debug mode
build-debug:
	@echo "$(YELLOW)Building MCP Gateway (debug)...$(RESET)"
	$(CARGO) build
	@echo "$(GREEN)Debug build completed!$(RESET)"

## WASM: Build WebAssembly packages
wasm:
	@echo "$(YELLOW)Building WASM packages...$(RESET)"
	wasm-pack build --target web --out-dir pkg-web
	wasm-pack build --target nodejs --out-dir pkg-node
	@echo "$(GREEN)WASM packages built!$(RESET)"

## Cross: Cross-compile for multiple platforms
cross:
	@echo "$(YELLOW)Cross-compiling for multiple platforms...$(RESET)"
	$(CARGO) build --release --target aarch64-unknown-linux-gnu
	$(CARGO) build --release --target x86_64-pc-windows-gnu
	$(CARGO) build --release --target x86_64-apple-darwin
	@echo "$(GREEN)Cross-compilation completed!$(RESET)"

## Test: Run all tests
test:
	@echo "$(YELLOW)Running tests...$(RESET)"
	$(CARGO) test --all-features
	@echo "$(GREEN)Tests completed!$(RESET)"

## Test Integration: Run integration tests
test-integration:
	@echo "$(YELLOW)Running integration tests...$(RESET)"
	$(CARGO) test --test integration --features integration-tests
	@echo "$(GREEN)Integration tests completed!$(RESET)"

## Test WASM: Run WASM tests
test-wasm:
	@echo "$(YELLOW)Running WASM tests...$(RESET)"
	wasm-pack test --node
	@echo "$(GREEN)WASM tests completed!$(RESET)"

## Bench: Run benchmarks
bench:
	@echo "$(YELLOW)Running benchmarks...$(RESET)"
	$(CARGO) bench
	@echo "$(GREEN)Benchmarks completed!$(RESET)"

## Lint: Run clippy lints
lint:
	@echo "$(YELLOW)Running clippy lints...$(RESET)"
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo "$(GREEN)Linting completed!$(RESET)"

## Format: Format code
format:
	@echo "$(YELLOW)Formatting code...$(RESET)"
	$(CARGO) fmt --all
	@echo "$(GREEN)Code formatted!$(RESET)"

## Format Check: Check code formatting
format-check:
	@echo "$(YELLOW)Checking code formatting...$(RESET)"
	$(CARGO) fmt --all -- --check
	@echo "$(GREEN)Format check completed!$(RESET)"

## Audit: Run security audit
audit:
	@echo "$(YELLOW)Running security audit...$(RESET)"
	$(CARGO) audit
	@echo "$(GREEN)Security audit completed!$(RESET)"

## Clean: Clean build artifacts
clean:
	@echo "$(YELLOW)Cleaning build artifacts...$(RESET)"
	$(CARGO) clean
	rm -rf pkg-*
	rm -rf target/
	rm -rf coverage/
	@echo "$(GREEN)Clean completed!$(RESET)"

## Install: Install the binary locally
install:
	@echo "$(YELLOW)Installing MCP Gateway...$(RESET)"
	$(CARGO) install --path .
	@echo "$(GREEN)Installation completed!$(RESET)"

## Dev: Start development environment
dev:
	@echo "$(YELLOW)Starting development environment...$(RESET)"
	$(DOCKER_COMPOSE) --profile development up -d
	@echo "$(GREEN)Development environment started!$(RESET)"
	@echo "Gateway: http://localhost:8080"
	@echo "Grafana: http://localhost:3000 (admin/admin)"
	@echo "Prometheus: http://localhost:9091"

## Dev Stop: Stop development environment
dev-stop:
	@echo "$(YELLOW)Stopping development environment...$(RESET)"
	$(DOCKER_COMPOSE) --profile development down
	@echo "$(GREEN)Development environment stopped!$(RESET)"

## Docker: Build Docker image
docker:
	@echo "$(YELLOW)Building Docker image...$(RESET)"
	$(DOCKER) build -t $(IMAGE_NAME):$(VERSION) -t $(IMAGE_NAME):latest .
	@echo "$(GREEN)Docker image built: $(IMAGE_NAME):$(VERSION)$(RESET)"

## Docker Run: Run Docker container
docker-run:
	@echo "$(YELLOW)Running Docker container...$(RESET)"
	$(DOCKER) run -d --name $(PROJECT_NAME) -p 8080:8080 $(IMAGE_NAME):latest
	@echo "$(GREEN)Container started: http://localhost:8080$(RESET)"

## Docker Push: Push Docker image to registry
docker-push:
	@echo "$(YELLOW)Pushing Docker image to registry...$(RESET)"
	$(DOCKER) push $(IMAGE_NAME):$(VERSION)
	$(DOCKER) push $(IMAGE_NAME):latest
	@echo "$(GREEN)Docker image pushed!$(RESET)"

## Docs: Generate documentation
docs:
	@echo "$(YELLOW)Generating documentation...$(RESET)"
	$(CARGO) doc --no-deps --open
	@echo "$(GREEN)Documentation generated!$(RESET)"

## Docs Build: Build documentation without opening
docs-build:
	@echo "$(YELLOW)Building documentation...$(RESET)"
	$(CARGO) doc --no-deps
	@echo "$(GREEN)Documentation built!$(RESET)"

## Coverage: Generate test coverage report
coverage:
	@echo "$(YELLOW)Generating coverage report...$(RESET)"
	$(CARGO) tarpaulin --out Html --output-dir coverage
	@echo "$(GREEN)Coverage report generated: coverage/tarpaulin-report.html$(RESET)"

## Size: Analyze binary size
size:
	@echo "$(YELLOW)Analyzing binary size...$(RESET)"
	$(CARGO) bloat --release
	@echo "$(GREEN)Size analysis completed!$(RESET)"

## Profile: Profile the application
profile:
	@echo "$(YELLOW)Profiling application...$(RESET)"
	$(CARGO) build --release
	perf record --call-graph dwarf target/release/gateway
	@echo "$(GREEN)Profiling completed!$(RESET)"

## Setup: Setup development environment
setup:
	@echo "$(YELLOW)Setting up development environment...$(RESET)"
	rustup target add wasm32-unknown-unknown wasm32-wasi aarch64-unknown-linux-gnu
	cargo install wasm-pack cargo-audit cargo-tarpaulin cargo-bloat
	@echo "$(GREEN)Development environment setup completed!$(RESET)"

## CI: Run full CI pipeline locally
ci: format-check lint audit test test-integration build wasm docker
	@echo "$(GREEN)CI pipeline completed successfully!$(RESET)"

## Release: Create a release build with all platforms
release: clean ci cross
	@echo "$(YELLOW)Creating release artifacts...$(RESET)"
	mkdir -p releases/$(VERSION)
	cp target/release/gateway releases/$(VERSION)/gateway-linux-x86_64
	cp target/aarch64-unknown-linux-gnu/release/gateway releases/$(VERSION)/gateway-linux-aarch64
	cp target/x86_64-pc-windows-gnu/release/gateway.exe releases/$(VERSION)/gateway-windows-x86_64.exe
	tar -czf releases/$(VERSION)/gateway-wasm-web.tar.gz pkg-web/
	tar -czf releases/$(VERSION)/gateway-wasm-node.tar.gz pkg-node/
	@echo "$(GREEN)Release artifacts created in releases/$(VERSION)/$(RESET)"

## Monitor: Show real-time logs
monitor:
	@echo "$(YELLOW)Showing real-time logs...$(RESET)"
	$(DOCKER_COMPOSE) logs -f gateway

## Health: Check service health
health:
	@echo "$(YELLOW)Checking service health...$(RESET)"
	curl -f http://localhost:8080/health || echo "$(RED)Service unhealthy$(RESET)"
	@echo "$(GREEN)Health check completed!$(RESET)"

## Metrics: Show current metrics
metrics:
	@echo "$(YELLOW)Fetching current metrics...$(RESET)"
	curl -s http://localhost:9090/metrics | grep mcp_
	@echo "$(GREEN)Metrics retrieved!$(RESET)"

# Platform-specific targets
## Pi: Build for Raspberry Pi
pi:
	@echo "$(YELLOW)Building for Raspberry Pi...$(RESET)"
	$(CARGO) build --release --target aarch64-unknown-linux-gnu --features raspberry-pi
	@echo "$(GREEN)Raspberry Pi build completed!$(RESET)"

## ESP32: Build for ESP32
esp32:
	@echo "$(YELLOW)Building for ESP32...$(RESET)"
	$(CARGO) build --release --target xtensa-esp32-none-elf --features esp32
	@echo "$(GREEN)ESP32 build completed!$(RESET)"

## Jetson: Build for NVIDIA Jetson
jetson:
	@echo "$(YELLOW)Building for NVIDIA Jetson...$(RESET)"
	$(CARGO) build --release --target aarch64-unknown-linux-gnu --features jetson
	@echo "$(GREEN)Jetson build completed!$(RESET)"

# Environment info
## Info: Show environment information
info:
	@echo "$(YELLOW)Environment Information:$(RESET)"
	@echo "Project: $(PROJECT_NAME)"
	@echo "Version: $(VERSION)"
	@echo "Registry: $(REGISTRY)"
	@echo "Rust version: $(shell rustc --version)"
	@echo "Cargo version: $(shell cargo --version)"
	@echo "Docker version: $(shell docker --version 2>/dev/null || echo 'Not installed')"
	@echo "Available targets: $(shell rustup target list --installed | tr '\n' ' ')"