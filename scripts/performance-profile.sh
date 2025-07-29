#!/bin/bash
set -euo pipefail

# Performance Profiling Script for MCP WASM Edge Gateway
# Provides comprehensive performance analysis and optimization recommendations

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROFILE_DIR="$PROJECT_ROOT/artifacts/profiles"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[PROFILE]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Create output directory
mkdir -p "$PROFILE_DIR"/{cpu,memory,io,network,binary}

# Default configuration
BINARY_PATH="$PROJECT_ROOT/target/release/mcp-gateway"
CONFIG_FILE="$PROJECT_ROOT/config/development.toml"
DURATION=60
WORKLOAD_TYPE="mixed"
TARGET_PLATFORM="native"

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --binary)
                BINARY_PATH="$2"
                shift 2
                ;;
            --config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --duration)
                DURATION="$2"
                shift 2
                ;;
            --workload)
                WORKLOAD_TYPE="$2"
                shift 2
                ;;
            --platform)
                TARGET_PLATFORM="$2"
                shift 2
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done
}

show_help() {
    cat << EOF
Performance Profiling Script for MCP WASM Edge Gateway

Usage: $0 [OPTIONS]

Options:
    --binary PATH       Path to binary to profile (default: target/release/mcp-gateway)
    --config FILE       Configuration file to use (default: config/development.toml)
    --duration SECONDS  Profiling duration in seconds (default: 60)
    --workload TYPE     Workload type: light|mixed|heavy|edge (default: mixed)
    --platform TARGET   Target platform: native|wasm|embedded (default: native)
    --help             Show this help message

Workload Types:
    light    - Low request rate, minimal resource usage
    mixed    - Balanced workload with various request types
    heavy    - High throughput, stress testing
    edge     - Edge device simulation with resource constraints

Platform Targets:
    native   - Full performance profiling on native platform
    wasm     - WebAssembly-specific profiling
    embedded - Embedded/IoT device simulation

Examples:
    $0 --workload heavy --duration 300
    $0 --platform wasm --workload edge
    $0 --binary target/release/mcp-gateway --config config/production.toml
EOF
}

# Check required tools
check_tools() {
    log "Checking profiling tools..."
    
    local tools=()
    local optional_tools=("perf" "valgrind" "hyperfine" "flamegraph" "heaptrack")
    
    for tool in "${optional_tools[@]}"; do
        if command -v "$tool" >/dev/null 2>&1; then
            log "Found: $tool"
        else
            warn "Optional tool not found: $tool"
        fi
    done
    
    # Check for cargo tools
    if ! cargo flamegraph --version >/dev/null 2>&1; then
        warn "cargo-flamegraph not installed. Install with: cargo install flamegraph"
    fi
    
    if ! cargo bloat --version >/dev/null 2>&1; then
        warn "cargo-bloat not installed. Install with: cargo install cargo-bloat"
    fi
}

# Build optimized binary
build_binary() {
    log "Building optimized binary..."
    
    cd "$PROJECT_ROOT"
    
    case "$TARGET_PLATFORM" in
        "native")
            cargo build --release --target-dir target
            ;;
        "wasm")
            wasm-pack build --target web --out-dir pkg --release
            BINARY_PATH="$PROJECT_ROOT/pkg/mcp_wasm_edge_gateway_bg.wasm"
            ;;
        "embedded")
            # Cross-compile for ARM if cross is available
            if command -v cross >/dev/null 2>&1; then
                cross build --target aarch64-unknown-linux-gnu --release
                BINARY_PATH="$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/mcp-gateway"
            else
                warn "Cross-compilation not available, using native build"
                cargo build --release
            fi
            ;;
    esac
    
    if [ ! -f "$BINARY_PATH" ]; then
        error "Binary not found after build: $BINARY_PATH"
    fi
    
    log "Binary built: $BINARY_PATH"
}

# Analyze binary size and composition
analyze_binary() {
    log "Analyzing binary composition..."
    
    if [ "$TARGET_PLATFORM" = "wasm" ]; then
        analyze_wasm_binary
        return
    fi
    
    # Binary size analysis
    local size_info="$PROFILE_DIR/binary/size-analysis.txt"
    
    {
        echo "Binary Size Analysis - $(date)"
        echo "======================================="
        echo
        
        ls -lh "$BINARY_PATH"
        echo
        
        if command -v file >/dev/null 2>&1; then
            echo "File type:"
            file "$BINARY_PATH"
            echo
        fi
        
        if command -v strip >/dev/null 2>&1; then
            local stripped_binary="$PROFILE_DIR/binary/$(basename "$BINARY_PATH")-stripped"
            cp "$BINARY_PATH" "$stripped_binary"
            strip "$stripped_binary"
            
            echo "Size comparison:"
            echo "Original: $(stat -f%z "$BINARY_PATH" 2>/dev/null || stat -c%s "$BINARY_PATH") bytes"
            echo "Stripped: $(stat -f%z "$stripped_binary" 2>/dev/null || stat -c%s "$stripped_binary") bytes"
            echo
        fi
    } > "$size_info"
    
    # Cargo bloat analysis
    if cargo bloat --version >/dev/null 2>&1; then
        log "Running cargo bloat analysis..."
        cd "$PROJECT_ROOT"
        cargo bloat --release --crates > "$PROFILE_DIR/binary/bloat-crates.txt" 2>/dev/null || true
        cargo bloat --release > "$PROFILE_DIR/binary/bloat-functions.txt" 2>/dev/null || true
    fi
    
    log "Binary analysis complete: $PROFILE_DIR/binary/"
}

# Analyze WASM binary
analyze_wasm_binary() {
    log "Analyzing WASM binary..."
    
    if command -v twiggy >/dev/null 2>&1; then
        local wasm_analysis="$PROFILE_DIR/binary/wasm-analysis.txt"
        
        {
            echo "WASM Binary Analysis - $(date)"
            echo "==============================="
            echo
            
            echo "Binary size:"
            ls -lh "$BINARY_PATH"
            echo
            
            echo "Top functions by size:"
            twiggy top "$BINARY_PATH" --max-items 20
            echo
            
            echo "Dominators tree:"
            twiggy dominators "$BINARY_PATH" --max-items 10
            echo
            
            echo "Call graph paths:"
            twiggy paths "$BINARY_PATH" --max-items 10
            
        } > "$wasm_analysis" 2>/dev/null || warn "WASM analysis failed"
    fi
}

# CPU profiling
profile_cpu() {
    log "Starting CPU profiling for ${DURATION}s..."
    
    if [ "$TARGET_PLATFORM" = "wasm" ]; then
        log "CPU profiling not available for WASM target"
        return
    fi
    
    # Start the service
    local service_pid
    start_service service_pid
    
    sleep 2  # Let service start
    
    # Generate workload
    generate_workload &
    local workload_pid=$!
    
    # CPU profiling with perf
    if command -v perf >/dev/null 2>&1; then
        log "Running perf profiling..."
        timeout "${DURATION}s" perf record -g -p "$service_pid" -o "$PROFILE_DIR/cpu/perf.data" 2>/dev/null || true
        
        if [ -f "$PROFILE_DIR/cpu/perf.data" ]; then
            perf report -i "$PROFILE_DIR/cpu/perf.data" --stdio > "$PROFILE_DIR/cpu/perf-report.txt" 2>/dev/null || true
        fi
    fi
    
    # Flamegraph generation
    if cargo flamegraph --version >/dev/null 2>&1; then
        log "Generating flamegraph..."
        timeout "${DURATION}s" cargo flamegraph \
            --bin mcp-gateway \
            --output "$PROFILE_DIR/cpu/flamegraph.svg" \
            -- --config "$CONFIG_FILE" 2>/dev/null || true
    fi
    
    # Clean up
    kill $workload_pid 2>/dev/null || true
    kill $service_pid 2>/dev/null || true
    wait $service_pid 2>/dev/null || true
    
    log "CPU profiling complete: $PROFILE_DIR/cpu/"
}

# Memory profiling
profile_memory() {
    log "Starting memory profiling..."
    
    if [ "$TARGET_PLATFORM" = "wasm" ]; then
        log "Memory profiling limited for WASM target"
        return
    fi
    
    local service_pid
    start_service service_pid
    
    sleep 2
    
    generate_workload &
    local workload_pid=$!
    
    # Memory profiling with Valgrind
    if command -v valgrind >/dev/null 2>&1; then
        log "Running Valgrind memory analysis..."
        
        # Start service under Valgrind
        kill $service_pid 2>/dev/null || true
        
        timeout "${DURATION}s" valgrind \
            --tool=memcheck \
            --leak-check=full \
            --show-leak-kinds=all \
            --track-origins=yes \
            --log-file="$PROFILE_DIR/memory/valgrind.log" \
            "$BINARY_PATH" --config "$CONFIG_FILE" 2>/dev/null &
        
        local valgrind_pid=$!
        sleep 5  # Let Valgrind start
        
        # Run workload
        sleep $((DURATION - 10))
        
        kill $valgrind_pid 2>/dev/null || true
        wait $valgrind_pid 2>/dev/null || true
    fi
    
    # Heap profiling with heaptrack (if available)
    if command -v heaptrack >/dev/null 2>&1; then
        log "Running heaptrack analysis..."
        
        timeout "${DURATION}s" heaptrack \
            --output "$PROFILE_DIR/memory/heaptrack.out" \
            "$BINARY_PATH" --config "$CONFIG_FILE" 2>/dev/null || true
    fi
    
    # RSS memory monitoring
    monitor_memory_usage "$service_pid" &
    local monitor_pid=$!
    
    sleep "$DURATION"
    
    # Clean up
    kill $workload_pid 2>/dev/null || true
    kill $monitor_pid 2>/dev/null || true
    kill $service_pid 2>/dev/null || true
    
    log "Memory profiling complete: $PROFILE_DIR/memory/"
}

# Monitor memory usage over time
monitor_memory_usage() {
    local pid=$1
    local memory_log="$PROFILE_DIR/memory/memory-usage.csv"
    
    echo "timestamp,rss_mb,vms_mb,cpu_percent" > "$memory_log"
    
    while kill -0 "$pid" 2>/dev/null; do
        if command -v ps >/dev/null 2>&1; then
            local stats=$(ps -o pid,rss,vsz,pcpu -p "$pid" --no-headers 2>/dev/null || echo "")
            if [ -n "$stats" ]; then
                local rss=$(echo "$stats" | awk '{print $2}')
                local vsz=$(echo "$stats" | awk '{print $3}')
                local cpu=$(echo "$stats" | awk '{print $4}')
                
                # Convert KB to MB
                rss=$(echo "scale=2; $rss / 1024" | bc 2>/dev/null || echo "0")
                vsz=$(echo "scale=2; $vsz / 1024" | bc 2>/dev/null || echo "0")
                
                echo "$(date -u +%Y-%m-%dT%H:%M:%SZ),$rss,$vsz,$cpu" >> "$memory_log"
            fi
        fi
        sleep 1
    done
}

# I/O profiling
profile_io() {
    log "Starting I/O profiling..."
    
    if [ "$TARGET_PLATFORM" = "wasm" ]; then
        log "I/O profiling not available for WASM target"
        return
    fi
    
    local service_pid
    start_service service_pid
    
    sleep 2
    
    # I/O monitoring with iostat (if available)
    if command -v iostat >/dev/null 2>&1; then
        log "Monitoring I/O with iostat..."
        iostat -x 1 "$DURATION" > "$PROFILE_DIR/io/iostat.log" 2>/dev/null &
        local iostat_pid=$!
    fi
    
    # Process-specific I/O monitoring
    monitor_process_io "$service_pid" &
    local io_monitor_pid=$!
    
    generate_workload &
    local workload_pid=$!
    
    sleep "$DURATION"
    
    # Clean up
    kill $workload_pid 2>/dev/null || true
    kill $io_monitor_pid 2>/dev/null || true
    kill ${iostat_pid:-0} 2>/dev/null || true
    kill $service_pid 2>/dev/null || true
    
    log "I/O profiling complete: $PROFILE_DIR/io/"
}

# Monitor process-specific I/O
monitor_process_io() {
    local pid=$1
    local io_log="$PROFILE_DIR/io/process-io.csv"
    
    echo "timestamp,read_bytes,write_bytes,read_calls,write_calls" > "$io_log"
    
    while kill -0 "$pid" 2>/dev/null; do
        if [ -f "/proc/$pid/io" ]; then
            local read_bytes=$(grep "read_bytes" "/proc/$pid/io" | awk '{print $2}' 2>/dev/null || echo "0")
            local write_bytes=$(grep "write_bytes" "/proc/$pid/io" | awk '{print $2}' 2>/dev/null || echo "0")
            local read_calls=$(grep "syscr" "/proc/$pid/io" | awk '{print $2}' 2>/dev/null || echo "0")
            local write_calls=$(grep "syscw" "/proc/$pid/io" | awk '{print $2}' 2>/dev/null || echo "0")
            
            echo "$(date -u +%Y-%m-%dT%H:%M:%SZ),$read_bytes,$write_bytes,$read_calls,$write_calls" >> "$io_log"
        fi
        sleep 1
    done
}

# Network profiling
profile_network() {
    log "Starting network profiling..."
    
    local service_pid
    start_service service_pid
    
    sleep 2
    
    # Network monitoring with netstat/ss
    monitor_network_connections &
    local network_monitor_pid=$!
    
    generate_workload &
    local workload_pid=$!
    
    sleep "$DURATION"
    
    # Clean up
    kill $workload_pid 2>/dev/null || true
    kill $network_monitor_pid 2>/dev/null || true
    kill $service_pid 2>/dev/null || true
    
    log "Network profiling complete: $PROFILE_DIR/network/"
}

# Monitor network connections
monitor_network_connections() {
    local network_log="$PROFILE_DIR/network/connections.csv"
    
    echo "timestamp,established,listen,time_wait,close_wait" > "$network_log"
    
    while true; do
        if command -v ss >/dev/null 2>&1; then
            local established=$(ss -t state established 2>/dev/null | wc -l)
            local listen=$(ss -t state listening 2>/dev/null | wc -l)
            local time_wait=$(ss -t state time-wait 2>/dev/null | wc -l)
            local close_wait=$(ss -t state close-wait 2>/dev/null | wc -l)
            
            echo "$(date -u +%Y-%m-%dT%H:%M:%SZ),$established,$listen,$time_wait,$close_wait" >> "$network_log"
        fi
        sleep 1
    done
}

# Start the service
start_service() {
    local pid_var=$1
    
    log "Starting service..."
    
    "$BINARY_PATH" --config "$CONFIG_FILE" >/dev/null 2>&1 &
    local service_pid=$!
    
    # Wait for service to be ready
    local retries=30
    while [ $retries -gt 0 ]; do
        if curl -s http://localhost:8080/health >/dev/null 2>&1; then
            log "Service ready (PID: $service_pid)"
            eval "$pid_var=$service_pid"
            return 0
        fi
        sleep 1
        retries=$((retries - 1))
    done
    
    error "Service failed to start"
}

# Generate workload based on type
generate_workload() {
    log "Generating $WORKLOAD_TYPE workload..."
    
    local base_url="http://localhost:8080"
    local requests_per_second
    local concurrent_requests
    
    case "$WORKLOAD_TYPE" in
        "light")
            requests_per_second=5
            concurrent_requests=2
            ;;
        "mixed")
            requests_per_second=20
            concurrent_requests=5
            ;;
        "heavy")
            requests_per_second=100
            concurrent_requests=20
            ;;
        "edge")
            requests_per_second=10
            concurrent_requests=3
            ;;
        *)
            requests_per_second=20
            concurrent_requests=5
            ;;
    esac
    
    # Use curl for simple load generation
    for ((i=1; i<=concurrent_requests; i++)); do
        generate_request_stream "$base_url" "$requests_per_second" &
    done
    
    wait
}

# Generate continuous request stream
generate_request_stream() {
    local base_url=$1
    local rps=$2
    local interval=$(echo "scale=3; 1 / $rps" | bc 2>/dev/null || echo "0.05")
    
    local endpoints=("/health" "/metrics" "/v1/queue/status")
    
    for ((i=0; i<DURATION*rps; i++)); do
        local endpoint=${endpoints[$((RANDOM % ${#endpoints[@]}))]}
        curl -s "$base_url$endpoint" >/dev/null 2>&1 || true
        sleep "$interval" 2>/dev/null || sleep 0.05
    done
}

# Generate performance report
generate_report() {
    log "Generating performance report..."
    
    local report_file="$PROFILE_DIR/performance-report.md"
    
    cat > "$report_file" << EOF
# Performance Profiling Report

## Configuration

- **Binary**: \`$(basename "$BINARY_PATH")\`
- **Platform**: $TARGET_PLATFORM
- **Workload**: $WORKLOAD_TYPE
- **Duration**: ${DURATION}s
- **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)

## Summary

### Binary Analysis

EOF
    
    if [ -f "$PROFILE_DIR/binary/size-analysis.txt" ]; then
        echo "#### Binary Size" >> "$report_file"
        echo '```' >> "$report_file"
        head -10 "$PROFILE_DIR/binary/size-analysis.txt" >> "$report_file"
        echo '```' >> "$report_file"
        echo >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
### CPU Performance

EOF
    
    if [ -f "$PROFILE_DIR/cpu/perf-report.txt" ]; then
        echo "#### Top CPU Consuming Functions" >> "$report_file"
        echo '```' >> "$report_file"
        head -20 "$PROFILE_DIR/cpu/perf-report.txt" | grep -v "^#" >> "$report_file"
        echo '```' >> "$report_file"
        echo >> "$report_file"
    fi
    
    if [ -f "$PROFILE_DIR/cpu/flamegraph.svg" ]; then
        echo "#### Flamegraph" >> "$report_file"
        echo "See: \`$(basename "$PROFILE_DIR")/cpu/flamegraph.svg\`" >> "$report_file"
        echo >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
### Memory Analysis

EOF
    
    if [ -f "$PROFILE_DIR/memory/memory-usage.csv" ]; then
        echo "#### Memory Usage Statistics" >> "$report_file"
        local max_rss=$(tail -n +2 "$PROFILE_DIR/memory/memory-usage.csv" | cut -d',' -f2 | sort -n | tail -1)
        local avg_rss=$(tail -n +2 "$PROFILE_DIR/memory/memory-usage.csv" | cut -d',' -f2 | awk '{sum+=$1} END {print sum/NR}')
        echo "- Peak RSS: ${max_rss}MB" >> "$report_file"
        echo "- Average RSS: ${avg_rss}MB" >> "$report_file"
        echo >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
### I/O Performance

EOF
    
    if [ -f "$PROFILE_DIR/io/process-io.csv" ]; then
        echo "#### I/O Statistics" >> "$report_file"
        local total_read=$(tail -1 "$PROFILE_DIR/io/process-io.csv" | cut -d',' -f2)
        local total_write=$(tail -1 "$PROFILE_DIR/io/process-io.csv" | cut -d',' -f3)
        echo "- Total bytes read: $total_read" >> "$report_file"
        echo "- Total bytes written: $total_write" >> "$report_file"
        echo >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
### Network Performance

EOF
    
    if [ -f "$PROFILE_DIR/network/connections.csv" ]; then
        echo "#### Connection Statistics" >> "$report_file"
        local max_connections=$(tail -n +2 "$PROFILE_DIR/network/connections.csv" | cut -d',' -f2 | sort -n | tail -1)
        echo "- Peak established connections: $max_connections" >> "$report_file"
        echo >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
## Recommendations

### Performance Optimizations

Based on the profiling results, consider the following optimizations:

EOF
    
    # Add automated recommendations based on findings
    if [ -f "$PROFILE_DIR/binary/bloat-crates.txt" ]; then
        local largest_crate=$(head -2 "$PROFILE_DIR/binary/bloat-crates.txt" | tail -1 | awk '{print $2}')
        if [ -n "$largest_crate" ]; then
            echo "- Consider optimizing the largest crate: $largest_crate" >> "$report_file"
        fi
    fi
    
    cat >> "$report_file" << EOF

### Platform-Specific Recommendations

EOF
    
    case "$TARGET_PLATFORM" in
        "wasm")
            cat >> "$report_file" << EOF
- Enable WASM SIMD optimizations
- Use \`wee_alloc\` for smaller binary size
- Consider code splitting for large applications
EOF
            ;;
        "embedded")
            cat >> "$report_file" << EOF
- Optimize for memory usage over performance
- Use \`panic = "abort"\` in release profile
- Consider no-std alternatives where possible
EOF
            ;;
        "native")
            cat >> "$report_file" << EOF
- Enable link-time optimization (LTO)
- Consider profile-guided optimization (PGO)
- Use system allocator optimizations
EOF
            ;;
    esac
    
    cat >> "$report_file" << EOF

## Files Generated

EOF
    
    find "$PROFILE_DIR" -type f -exec basename {} \; | sort | sed 's/^/- /' >> "$report_file"
    
    log "Performance report generated: $report_file"
}

# Main execution
main() {
    parse_args "$@"
    
    log "Starting performance profiling..."
    log "Target: $TARGET_PLATFORM"
    log "Workload: $WORKLOAD_TYPE"
    log "Duration: ${DURATION}s"
    
    check_tools
    build_binary
    analyze_binary
    
    case "$TARGET_PLATFORM" in
        "wasm")
            # Limited profiling for WASM
            log "WASM profiling completed"
            ;;
        *)
            profile_cpu
            profile_memory
            profile_io
            profile_network
            ;;
    esac
    
    generate_report
    
    log "Performance profiling completed!"
    log "Results: $PROFILE_DIR"
}

# Run main function
main "$@"