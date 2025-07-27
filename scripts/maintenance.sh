#!/bin/bash
# Maintenance script for MCP WASM Edge Gateway
# Performs routine maintenance tasks

set -e

# Configuration
LOG_RETENTION_DAYS=30
BACKUP_RETENTION_DAYS=90
METRICS_RETENTION_DAYS=7

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
MCP WASM Edge Gateway Maintenance Script

Usage: $0 [OPTIONS] [TASK]

Tasks:
    all                 Run all maintenance tasks (default)
    cleanup             Clean up temporary files and old logs
    backup              Create backups of important data
    update              Update dependencies and check for updates
    security            Run security scans and audits
    health              Perform health checks
    metrics             Collect and archive metrics
    optimize            Optimize performance and clean caches

Options:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -d, --dry-run       Dry run mode (show what would be done)
    --force             Force operations that normally require confirmation
    --config FILE       Use specific configuration file

Examples:
    $0                      # Run all maintenance tasks
    $0 cleanup              # Only clean up files
    $0 --dry-run           # Show what would be done
    $0 security --verbose   # Run security tasks with verbose output
EOF
}

# Parse command line arguments
TASK="all"
VERBOSE=false
DRY_RUN=false
FORCE=false
CONFIG_FILE=""

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
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        all|cleanup|backup|update|security|health|metrics|optimize)
            TASK="$1"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Load configuration
load_config() {
    if [[ -n "$CONFIG_FILE" && -f "$CONFIG_FILE" ]]; then
        source "$CONFIG_FILE"
        log_info "Loaded configuration from $CONFIG_FILE"
    elif [[ -f ".env" ]]; then
        source ".env"
        log_info "Loaded configuration from .env"
    fi
}

# Cleanup task
cleanup_files() {
    log_info "Running cleanup tasks..."
    
    local cleaned_items=0
    
    # Clean temporary files
    if [[ -d "tmp" ]]; then
        if [[ "$DRY_RUN" == false ]]; then
            find tmp -type f -mtime +1 -delete 2>/dev/null || true
            cleaned_items=$((cleaned_items + $(find tmp -type f -mtime +1 2>/dev/null | wc -l)))
        else
            log_info "[DRY RUN] Would clean $(find tmp -type f -mtime +1 2>/dev/null | wc -l) temporary files"
        fi
    fi
    
    # Clean old log files
    if [[ -d "logs" ]]; then
        if [[ "$DRY_RUN" == false ]]; then
            find logs -name "*.log" -mtime +$LOG_RETENTION_DAYS -delete 2>/dev/null || true
            cleaned_items=$((cleaned_items + $(find logs -name "*.log" -mtime +$LOG_RETENTION_DAYS 2>/dev/null | wc -l)))
        else
            log_info "[DRY RUN] Would clean $(find logs -name "*.log" -mtime +$LOG_RETENTION_DAYS 2>/dev/null | wc -l) old log files"
        fi
    fi
    
    # Clean build artifacts
    if [[ -d "target" ]]; then
        local target_size=$(du -sh target 2>/dev/null | cut -f1 || echo "0")
        if [[ "$DRY_RUN" == false ]]; then
            cargo clean
            log_info "Cleaned build artifacts (freed: $target_size)"
        else
            log_info "[DRY RUN] Would clean build artifacts ($target_size)"
        fi
    fi
    
    # Clean Docker artifacts
    if command -v docker &> /dev/null; then
        if [[ "$DRY_RUN" == false ]]; then
            docker system prune -f > /dev/null 2>&1 || true
            log_info "Cleaned Docker artifacts"
        else
            log_info "[DRY RUN] Would clean Docker artifacts"
        fi
    fi
    
    log_success "Cleanup completed"
}

# Backup task
backup_data() {
    log_info "Creating backups..."
    
    local backup_dir="backups/$(date +%Y%m%d_%H%M%S)"
    
    if [[ "$DRY_RUN" == false ]]; then
        mkdir -p "$backup_dir"
        
        # Backup configuration files
        cp -r .env* "$backup_dir/" 2>/dev/null || true
        cp -r *.toml "$backup_dir/" 2>/dev/null || true
        cp -r docs/ "$backup_dir/" 2>/dev/null || true
        
        # Backup database if exists
        if [[ -f "gateway.db" ]]; then
            cp gateway.db "$backup_dir/"
        fi
        
        # Create archive
        tar -czf "$backup_dir.tar.gz" -C backups "$(basename "$backup_dir")"
        rm -rf "$backup_dir"
        
        log_success "Backup created: $backup_dir.tar.gz"
    else
        log_info "[DRY RUN] Would create backup: $backup_dir.tar.gz"
    fi
    
    # Clean old backups
    if [[ -d "backups" ]]; then
        if [[ "$DRY_RUN" == false ]]; then
            find backups -name "*.tar.gz" -mtime +$BACKUP_RETENTION_DAYS -delete 2>/dev/null || true
        else
            local old_backups=$(find backups -name "*.tar.gz" -mtime +$BACKUP_RETENTION_DAYS 2>/dev/null | wc -l)
            log_info "[DRY RUN] Would clean $old_backups old backup files"
        fi
    fi
}

# Update task
update_dependencies() {
    log_info "Checking for updates..."
    
    # Update Rust toolchain
    if command -v rustup &> /dev/null; then
        if [[ "$DRY_RUN" == false ]]; then
            rustup update
            log_info "Rust toolchain updated"
        else
            log_info "[DRY RUN] Would update Rust toolchain"
        fi
    fi
    
    # Check for Cargo updates
    if command -v cargo-outdated &> /dev/null; then
        log_info "Checking for outdated Cargo dependencies:"
        cargo outdated --root-deps-only
    else
        log_warning "cargo-outdated not installed, skipping dependency check"
    fi
    
    # Check for npm updates
    if [[ -f "package.json" ]] && command -v npm &> /dev/null; then
        log_info "Checking for npm updates:"
        npm outdated || true
    fi
    
    # Update lock files
    if [[ "$DRY_RUN" == false ]]; then
        cargo update
        if [[ -f "package.json" ]]; then
            npm update
        fi
        log_success "Lock files updated"
    else
        log_info "[DRY RUN] Would update lock files"
    fi
}

# Security task
security_audit() {
    log_info "Running security audits..."
    
    # Cargo security audit
    if command -v cargo-audit &> /dev/null; then
        cargo audit
        log_success "Cargo security audit completed"
    else
        log_warning "cargo-audit not installed, skipping Rust security check"
    fi
    
    # Dependency license check
    if command -v cargo-deny &> /dev/null; then
        cargo deny check
        log_success "License and ban check completed"
    else
        log_warning "cargo-deny not installed, skipping license check"
    fi
    
    # npm security audit
    if [[ -f "package.json" ]] && command -v npm &> /dev/null; then
        npm audit --audit-level moderate
        log_success "npm security audit completed"
    fi
    
    # Docker image scanning
    if command -v docker &> /dev/null && command -v trivy &> /dev/null; then
        if docker images | grep -q "mcp-gateway"; then
            trivy image mcp-gateway:latest
            log_success "Docker image security scan completed"
        fi
    fi
}

# Health check task
health_check() {
    log_info "Performing health checks..."
    
    # Check disk space
    local disk_usage=$(df -h . | awk 'NR==2 {print $5}' | sed 's/%//')
    if [[ $disk_usage -gt 80 ]]; then
        log_warning "Disk usage is at ${disk_usage}%"
    else
        log_info "Disk usage: ${disk_usage}%"
    fi
    
    # Check memory usage
    if command -v free &> /dev/null; then
        local mem_usage=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}')
        if [[ $mem_usage -gt 80 ]]; then
            log_warning "Memory usage is at ${mem_usage}%"
        else
            log_info "Memory usage: ${mem_usage}%"
        fi
    fi
    
    # Check if gateway is running (if applicable)
    if systemctl is-active --quiet mcp-gateway 2>/dev/null; then
        log_success "Gateway service is running"
    elif pgrep -f "gateway" > /dev/null; then
        log_success "Gateway process is running"
    else
        log_info "Gateway is not currently running"
    fi
    
    # Test compilation
    if [[ "$DRY_RUN" == false ]]; then
        if cargo check --quiet; then
            log_success "Code compilation check passed"
        else
            log_error "Code compilation check failed"
        fi
    else
        log_info "[DRY RUN] Would check code compilation"
    fi
}

# Metrics task
collect_metrics() {
    log_info "Collecting and archiving metrics..."
    
    local metrics_dir="metrics/$(date +%Y%m%d)"
    
    if [[ "$DRY_RUN" == false ]]; then
        mkdir -p "$metrics_dir"
        
        # Collect system metrics
        {
            echo "# System metrics collected on $(date)"
            echo "disk_usage_percent $(df -h . | awk 'NR==2 {print $5}' | sed 's/%//')"
            echo "memory_usage_percent $(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}')"
            echo "load_average $(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')"
        } > "$metrics_dir/system.metrics"
        
        # Collect binary sizes
        if [[ -f "target/release/gateway" ]]; then
            ls -lh target/release/gateway | awk '{print "binary_size_bytes", $5}' > "$metrics_dir/binary.metrics"
        fi
        
        log_success "Metrics collected in $metrics_dir"
    else
        log_info "[DRY RUN] Would collect metrics in $metrics_dir"
    fi
    
    # Clean old metrics
    if [[ -d "metrics" ]]; then
        if [[ "$DRY_RUN" == false ]]; then
            find metrics -type d -mtime +$METRICS_RETENTION_DAYS -exec rm -rf {} + 2>/dev/null || true
        else
            local old_metrics=$(find metrics -type d -mtime +$METRICS_RETENTION_DAYS 2>/dev/null | wc -l)
            log_info "[DRY RUN] Would clean $old_metrics old metric directories"
        fi
    fi
}

# Optimization task
optimize_performance() {
    log_info "Running optimization tasks..."
    
    # Optimize Git repository
    if [[ "$DRY_RUN" == false ]]; then
        git gc --aggressive --prune=now > /dev/null 2>&1 || true
        log_info "Git repository optimized"
    else
        log_info "[DRY RUN] Would optimize Git repository"
    fi
    
    # Optimize Docker
    if command -v docker &> /dev/null; then
        if [[ "$DRY_RUN" == false ]]; then
            docker system prune -f > /dev/null 2>&1 || true
            log_info "Docker system optimized"
        else
            log_info "[DRY RUN] Would optimize Docker system"
        fi
    fi
    
    # Clear Cargo cache if too large
    if [[ -d ~/.cargo ]]; then
        local cargo_size=$(du -sm ~/.cargo 2>/dev/null | cut -f1 || echo "0")
        if [[ $cargo_size -gt 5000 ]]; then  # 5GB threshold
            if [[ "$DRY_RUN" == false ]]; then
                cargo clean
                log_info "Cargo cache cleaned (was ${cargo_size}MB)"
            else
                log_info "[DRY RUN] Would clean Cargo cache (${cargo_size}MB)"
            fi
        fi
    fi
    
    log_success "Optimization completed"
}

# Generate maintenance report
generate_report() {
    local report_file="maintenance_report_$(date +%Y%m%d_%H%M%S).md"
    
    cat << EOF > "$report_file"
# Maintenance Report

**Date**: $(date)
**Tasks**: $TASK
**Mode**: $(if [[ "$DRY_RUN" == true ]]; then echo "Dry Run"; else echo "Execution"; fi)

## Summary

- **Disk Usage**: $(df -h . | awk 'NR==2 {print $5}')
- **Memory Usage**: $(free | grep Mem | awk '{printf "%.0f%%", $3/$2 * 100.0}')
- **Git Status**: $(git status --porcelain | wc -l) uncommitted changes
- **Rust Version**: $(rustc --version)

## Tasks Completed

$(if [[ "$TASK" == "all" || "$TASK" == "cleanup" ]]; then echo "- [x] Cleanup"; else echo "- [ ] Cleanup"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "backup" ]]; then echo "- [x] Backup"; else echo "- [ ] Backup"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "update" ]]; then echo "- [x] Update"; else echo "- [ ] Update"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "security" ]]; then echo "- [x] Security"; else echo "- [ ] Security"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "health" ]]; then echo "- [x] Health"; else echo "- [ ] Health"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "metrics" ]]; then echo "- [x] Metrics"; else echo "- [ ] Metrics"; fi)
$(if [[ "$TASK" == "all" || "$TASK" == "optimize" ]]; then echo "- [x] Optimize"; else echo "- [ ] Optimize"; fi)

## Next Maintenance

Recommended next maintenance: $(date -d "+1 week" +%Y-%m-%d)

---
*Generated by maintenance script*
EOF

    log_info "Maintenance report generated: $report_file"
}

# Main execution
main() {
    log_info "Starting maintenance tasks for MCP WASM Edge Gateway"
    log_info "Task: $TASK, Mode: $(if [[ "$DRY_RUN" == true ]]; then echo "Dry Run"; else echo "Execution"; fi)"
    
    load_config
    
    case "$TASK" in
        all)
            cleanup_files
            backup_data
            update_dependencies
            security_audit
            health_check
            collect_metrics
            optimize_performance
            ;;
        cleanup)
            cleanup_files
            ;;
        backup)
            backup_data
            ;;
        update)
            update_dependencies
            ;;
        security)
            security_audit
            ;;
        health)
            health_check
            ;;
        metrics)
            collect_metrics
            ;;
        optimize)
            optimize_performance
            ;;
        *)
            log_error "Unknown task: $TASK"
            show_help
            exit 1
            ;;
    esac
    
    generate_report
    
    log_success "Maintenance tasks completed successfully!"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_warning "This was a dry run - no actual changes were made"
    fi
}

# Execute main function
main "$@"