#!/bin/bash
# Build script for MCP WASM Edge Gateway
# Supports multiple platforms and targets

set -e

# Configuration
PROJECT_NAME="mcp-wasm-edge-gateway"
BUILD_DIR="target"
RELEASE_DIR="releases"
PLATFORMS=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-pc-windows-gnu")
WASM_TARGETS=("web" "nodejs")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
MCP WASM Edge Gateway Build Script

Usage: $0 [OPTIONS] [COMMAND]

Commands:
    all             Build all targets (default)
    native          Build native target only
    wasm            Build WASM targets only
    cross           Build cross-compilation targets
    release         Build release packages
    clean           Clean build artifacts
    
Options:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -r, --release   Build in release mode (default)
    -d, --debug     Build in debug mode
    --profile PROF  Build with specific profile
    --target TARGET Build specific target only
    --features FEAT Enable specific features

Examples:
    $0                          # Build all targets
    $0 native                   # Build native target only
    $0 --target wasm32-unknown-unknown
    $0 --features "hardware-security,compression"
    $0 release                  # Create release packages
EOF
}

# Parse command line arguments
COMMAND="all"
BUILD_MODE="release"
VERBOSE=false
TARGET=""
FEATURES=""
PROFILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -r|--release)
            BUILD_MODE="release"
            shift
            ;;
        -d|--debug)
            BUILD_MODE="debug"
            shift
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        all|native|wasm|cross|release|clean)
            COMMAND="$1"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Set build flags
BUILD_FLAGS=""
if [[ "$BUILD_MODE" == "release" ]]; then
    BUILD_FLAGS="--release"
fi

if [[ -n "$FEATURES" ]]; then
    BUILD_FLAGS="$BUILD_FLAGS --features $FEATURES"
fi

if [[ -n "$PROFILE" ]]; then
    BUILD_FLAGS="$BUILD_FLAGS --profile $PROFILE"
fi

if [[ "$VERBOSE" == true ]]; then
    BUILD_FLAGS="$BUILD_FLAGS --verbose"
fi

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust."
        exit 1
    fi
    
    # Check wasm-pack for WASM builds
    if [[ "$COMMAND" == "all" || "$COMMAND" == "wasm" ]] && ! command -v wasm-pack &> /dev/null; then
        log_warning "wasm-pack not found. Installing..."
        cargo install wasm-pack
    fi
    
    log_success "Prerequisites check completed"
}

# Build native target
build_native() {
    log_info "Building native target..."
    
    cargo build $BUILD_FLAGS
    
    if [[ "$BUILD_MODE" == "release" ]]; then
        local binary_path="target/release/gateway"
        if [[ -f "$binary_path" ]]; then
            local size=$(du -h "$binary_path" | cut -f1)
            log_success "Native build completed. Binary size: $size"
        fi
    else
        log_success "Native debug build completed"
    fi
}

# Build WASM targets
build_wasm() {
    log_info "Building WASM targets..."
    
    for target in "${WASM_TARGETS[@]}"; do
        log_info "Building WASM for $target..."
        
        local out_dir="pkg-$target"
        local wasm_flags=""
        
        if [[ "$BUILD_MODE" == "release" ]]; then
            wasm_flags="--release"
        fi
        
        if [[ "$target" == "web" ]]; then
            wasm-pack build --target web --out-dir "$out_dir" $wasm_flags
        elif [[ "$target" == "nodejs" ]]; then
            wasm-pack build --target nodejs --out-dir "$out_dir" $wasm_flags
        fi
        
        if [[ -f "$out_dir/mcp_gateway_bg.wasm" ]]; then
            local size=$(du -h "$out_dir/mcp_gateway_bg.wasm" | cut -f1)
            log_success "WASM build for $target completed. WASM size: $size"
        fi
    done
}

# Build cross-compilation targets
build_cross() {
    log_info "Building cross-compilation targets..."
    
    # Check if targets are installed
    for platform in "${PLATFORMS[@]}"; do
        if ! rustup target list --installed | grep -q "$platform"; then
            log_info "Installing target $platform..."
            rustup target add "$platform"
        fi
    done
    
    for platform in "${PLATFORMS[@]}"; do
        log_info "Building for $platform..."
        
        cargo build $BUILD_FLAGS --target "$platform"
        
        if [[ "$BUILD_MODE" == "release" ]]; then
            local binary_name="gateway"
            if [[ "$platform" == *"windows"* ]]; then
                binary_name="gateway.exe"
            fi
            
            local binary_path="target/$platform/release/$binary_name"
            if [[ -f "$binary_path" ]]; then
                local size=$(du -h "$binary_path" | cut -f1)
                log_success "Cross-compilation for $platform completed. Binary size: $size"
            fi
        else
            log_success "Cross-compilation for $platform completed (debug)"
        fi
    done
}

# Build specific target
build_target() {
    local target="$1"
    log_info "Building for target: $target"
    
    # Install target if not present
    if ! rustup target list --installed | grep -q "$target"; then
        log_info "Installing target $target..."
        rustup target add "$target"
    fi
    
    cargo build $BUILD_FLAGS --target "$target"
    log_success "Build for $target completed"
}

# Create release packages
create_release() {
    local version=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
    local release_path="$RELEASE_DIR/$version"
    
    log_info "Creating release packages for version $version..."
    
    # Ensure release mode
    BUILD_MODE="release"
    BUILD_FLAGS="--release"
    
    # Clean and build all targets
    cargo clean
    build_native
    build_wasm
    build_cross
    
    # Create release directory
    mkdir -p "$release_path"
    
    # Copy native binary
    if [[ -f "target/release/gateway" ]]; then
        cp "target/release/gateway" "$release_path/gateway-linux-x86_64"
    fi
    
    # Copy cross-compiled binaries
    for platform in "${PLATFORMS[@]}"; do
        local binary_name="gateway"
        if [[ "$platform" == *"windows"* ]]; then
            binary_name="gateway.exe"
            local output_name="gateway-windows-x86_64.exe"
        elif [[ "$platform" == *"aarch64"* ]]; then
            local output_name="gateway-linux-aarch64"
        else
            local output_name="gateway-linux-x86_64-alt"
        fi
        
        local binary_path="target/$platform/release/$binary_name"
        if [[ -f "$binary_path" ]]; then
            cp "$binary_path" "$release_path/$output_name"
        fi
    done
    
    # Package WASM builds
    for target in "${WASM_TARGETS[@]}"; do
        local pkg_dir="pkg-$target"
        if [[ -d "$pkg_dir" ]]; then
            tar -czf "$release_path/gateway-wasm-$target.tar.gz" -C "$pkg_dir" .
        fi
    done
    
    # Create checksums
    cd "$release_path"
    sha256sum * > checksums.txt
    cd - > /dev/null
    
    log_success "Release packages created in $release_path"
    log_info "Contents:"
    ls -la "$release_path"
}

# Clean build artifacts
clean_build() {
    log_info "Cleaning build artifacts..."
    
    cargo clean
    rm -rf pkg-*
    rm -rf "$RELEASE_DIR"
    
    log_success "Build artifacts cleaned"
}

# Run tests
run_tests() {
    log_info "Running tests..."
    
    cargo test --all-features
    
    # Run WASM tests if wasm-pack is available
    if command -v wasm-pack &> /dev/null; then
        wasm-pack test --node
    fi
    
    log_success "Tests completed"
}

# Main execution
main() {
    log_info "Starting build process for $PROJECT_NAME"
    log_info "Command: $COMMAND, Mode: $BUILD_MODE"
    
    check_prerequisites
    
    case "$COMMAND" in
        all)
            if [[ -n "$TARGET" ]]; then
                build_target "$TARGET"
            else
                build_native
                build_wasm
                build_cross
            fi
            ;;
        native)
            build_native
            ;;
        wasm)
            build_wasm
            ;;
        cross)
            build_cross
            ;;
        release)
            create_release
            ;;
        clean)
            clean_build
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            show_help
            exit 1
            ;;
    esac
    
    log_success "Build process completed successfully!"
}

# Error handling
trap 'log_error "Build failed!"; exit 1' ERR

# Execute main function
main