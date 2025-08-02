#!/bin/bash
# Comprehensive build script for MCP WASM Edge Gateway
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
BUILD_TYPE="debug"
TARGET_PLATFORM=""
ENABLE_WASM=false
ENABLE_CROSS=false
ENABLE_DOCKER=false
ENABLE_TESTS=false
ENABLE_BENCHMARKS=false
VERBOSE=false

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build script for MCP WASM Edge Gateway

OPTIONS:
    -t, --type TYPE          Build type: debug, release (default: debug)
    -p, --platform PLATFORM Target platform: native, raspberry-pi, esp32, windows, macos
    -w, --wasm              Build WASM targets
    -c, --cross             Enable cross-compilation
    -d, --docker            Build Docker images
    --tests                 Run tests after build
    --benchmarks            Run benchmarks after build
    -v, --verbose           Verbose output
    -h, --help              Show this help message

EXAMPLES:
    $0 --type release --platform raspberry-pi
    $0 --wasm --type release
    $0 --docker --tests
    $0 --cross --platform esp32 --type release

SUPPORTED PLATFORMS:
    native          Build for current platform
    raspberry-pi    ARM64 Linux (aarch64-unknown-linux-gnu)
    esp32           Xtensa ESP32 (xtensa-esp32s3-none-elf)
    windows         Windows x64 (x86_64-pc-windows-gnu)
    macos           macOS ARM64 (aarch64-apple-darwin)
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--type)
            BUILD_TYPE="$2"
            shift 2
            ;;
        -p|--platform)
            TARGET_PLATFORM="$2"
            shift 2
            ;;
        -w|--wasm)
            ENABLE_WASM=true
            shift
            ;;
        -c|--cross)
            ENABLE_CROSS=true
            shift
            ;;
        -d|--docker)
            ENABLE_DOCKER=true
            shift
            ;;
        --tests)
            ENABLE_TESTS=true
            shift
            ;;
        --benchmarks)
            ENABLE_BENCHMARKS=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Validate build type
if [[ "$BUILD_TYPE" != "debug" && "$BUILD_TYPE" != "release" ]]; then
    print_error "Invalid build type: $BUILD_TYPE. Must be 'debug' or 'release'"
    exit 1
fi

# Set verbose flag for cargo
CARGO_VERBOSE=""
if [[ "$VERBOSE" == true ]]; then
    CARGO_VERBOSE="--verbose"
fi

# Set build flags based on type
CARGO_BUILD_FLAGS=""
if [[ "$BUILD_TYPE" == "release" ]]; then
    CARGO_BUILD_FLAGS="--release"
fi

print_info "Starting MCP WASM Edge Gateway build..."
print_info "Build type: $BUILD_TYPE"
print_info "Target platform: ${TARGET_PLATFORM:-native}"
print_info "WASM build: $ENABLE_WASM"
print_info "Cross-compilation: $ENABLE_CROSS"
print_info "Docker build: $ENABLE_DOCKER"

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Function to install cross-compilation tools
install_cross_tools() {
    print_info "Installing cross-compilation tools..."
    
    if ! command -v cross &> /dev/null; then
        cargo install cross
    fi
    
    case "$TARGET_PLATFORM" in
        raspberry-pi)
            rustup target add aarch64-unknown-linux-gnu
            ;;
        esp32)
            # ESP32 toolchain installation is complex, provide instructions
            print_warning "ESP32 cross-compilation requires special setup."
            print_warning "Please follow: https://esp-rs.github.io/book/installation/index.html"
            ;;
        windows)
            rustup target add x86_64-pc-windows-gnu
            ;;
        macos)
            rustup target add aarch64-apple-darwin
            ;;
    esac
}

# Function to build native targets
build_native() {
    print_info "Building native targets..."
    
    cargo build $CARGO_BUILD_FLAGS $CARGO_VERBOSE --workspace --all-features
    
    if [[ "$BUILD_TYPE" == "release" ]]; then
        # Strip binaries to reduce size
        if command -v strip &> /dev/null; then
            print_info "Stripping release binaries..."
            find target/release -maxdepth 1 -type f -executable -exec strip {} \;
        fi
    fi
    
    print_success "Native build completed"
}

# Function to build WASM targets
build_wasm() {
    print_info "Building WASM targets..."
    
    # Check if wasm-pack is installed
    if ! command -v wasm-pack &> /dev/null; then
        print_info "Installing wasm-pack..."
        curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    fi
    
    # Add WASM targets
    rustup target add wasm32-unknown-unknown wasm32-wasi
    
    # Build for different WASM targets
    print_info "Building WASM for web..."
    wasm-pack build --target web --out-dir pkg-web $CARGO_BUILD_FLAGS -- --features wasm-web
    
    print_info "Building WASM for Node.js..."
    wasm-pack build --target nodejs --out-dir pkg-node $CARGO_BUILD_FLAGS -- --features wasm-node
    
    print_info "Building WASM for bundlers..."
    wasm-pack build --target bundler --out-dir pkg-bundler $CARGO_BUILD_FLAGS
    
    print_info "Building WASI binary..."
    cargo build --target wasm32-wasi $CARGO_BUILD_FLAGS --bin mcp-gateway-wasm
    
    # Optimize WASM if wasm-opt is available
    if command -v wasm-opt &> /dev/null; then
        print_info "Optimizing WASM binaries..."
        wasm-opt -O3 pkg-web/mcp_wasm_edge_gateway_bg.wasm -o pkg-web/optimized.wasm
        wasm-opt -O3 pkg-node/mcp_wasm_edge_gateway_bg.wasm -o pkg-node/optimized.wasm
    fi
    
    # Check WASM size requirements
    check_wasm_size
    
    print_success "WASM build completed"
}

# Function to check WASM binary sizes
check_wasm_size() {
    print_info "Checking WASM binary sizes..."
    
    local size_limit=$((3 * 1024 * 1024)) # 3MB in bytes
    
    for wasm_file in pkg-web/mcp_wasm_edge_gateway_bg.wasm pkg-node/mcp_wasm_edge_gateway_bg.wasm; do
        if [[ -f "$wasm_file" ]]; then
            local size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file" 2>/dev/null)
            local size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc -l 2>/dev/null || echo "N/A")
            
            if [[ "$size" -gt "$size_limit" ]]; then
                print_warning "$wasm_file is ${size_mb}MB (exceeds 3MB limit)"
            else
                print_success "$wasm_file is ${size_mb}MB (within 3MB limit)"
            fi
        fi
    done
}

# Function to build cross-compilation targets
build_cross() {
    print_info "Building cross-compilation targets..."
    
    if [[ "$ENABLE_CROSS" == true ]]; then
        install_cross_tools
    fi
    
    case "$TARGET_PLATFORM" in
        raspberry-pi)
            print_info "Building for Raspberry Pi (ARM64)..."
            cross build --target aarch64-unknown-linux-gnu $CARGO_BUILD_FLAGS
            ;;
        esp32)
            print_info "Building for ESP32..."
            cargo build --target xtensa-esp32s3-none-elf $CARGO_BUILD_FLAGS
            ;;
        windows)
            print_info "Building for Windows..."
            cross build --target x86_64-pc-windows-gnu $CARGO_BUILD_FLAGS
            ;;
        macos)
            print_info "Building for macOS..."
            cross build --target aarch64-apple-darwin $CARGO_BUILD_FLAGS
            ;;
        *)
            print_error "Unsupported platform: $TARGET_PLATFORM"
            exit 1
            ;;
    esac
    
    print_success "Cross-compilation completed"
}

# Function to build Docker images
build_docker() {
    print_info "Building Docker images..."
    
    # Check if Docker is available
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi
    
    # Build main application image
    print_info "Building main application image..."
    docker build -t mcp-edge-gateway:latest .
    
    # Build test image
    print_info "Building test image..."
    docker build -f Dockerfile.test -t mcp-edge-gateway:test .
    
    # Build WASM image if requested
    if [[ "$ENABLE_WASM" == true ]]; then
        print_info "Building WASM image..."
        docker build -f Dockerfile.wasm -t mcp-edge-gateway:wasm .
    fi
    
    print_success "Docker build completed"
}

# Function to run tests
run_tests() {
    print_info "Running test suite..."
    
    # Run unit tests
    cargo test $CARGO_VERBOSE --workspace --all-features
    
    # Run integration tests
    cargo test $CARGO_VERBOSE --test integration
    
    # Run doc tests
    cargo test $CARGO_VERBOSE --doc
    
    print_success "All tests passed"
}

# Function to run benchmarks
run_benchmarks() {
    print_info "Running benchmarks..."
    
    cargo bench $CARGO_VERBOSE --workspace
    
    print_success "Benchmarks completed"
}

# Main build process
main() {
    # Native build (always done)
    if [[ -z "$TARGET_PLATFORM" || "$TARGET_PLATFORM" == "native" ]]; then
        build_native
    fi
    
    # Cross-compilation build
    if [[ -n "$TARGET_PLATFORM" && "$TARGET_PLATFORM" != "native" ]]; then
        build_cross
    fi
    
    # WASM build
    if [[ "$ENABLE_WASM" == true ]]; then
        build_wasm
    fi
    
    # Docker build
    if [[ "$ENABLE_DOCKER" == true ]]; then
        build_docker
    fi
    
    # Tests
    if [[ "$ENABLE_TESTS" == true ]]; then
        run_tests
    fi
    
    # Benchmarks
    if [[ "$ENABLE_BENCHMARKS" == true ]]; then
        run_benchmarks
    fi
    
    print_success "Build process completed successfully!"
    
    # Show build artifacts
    print_info "Build artifacts:"
    if [[ "$BUILD_TYPE" == "release" ]]; then
        ls -lh target/release/ 2>/dev/null | grep -E '^-.*mcp' || true
    else
        ls -lh target/debug/ 2>/dev/null | grep -E '^-.*mcp' || true
    fi
    
    if [[ "$ENABLE_WASM" == true ]]; then
        print_info "WASM artifacts:"
        ls -lh pkg-*/*.wasm 2>/dev/null || true
    fi
}

# Run main function
main "$@"