#!/bin/bash

# MCP WASM Edge Gateway Production Deployment Script
# This script handles the complete deployment process with safety checks

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
DEPLOYMENT_DIR="$PROJECT_ROOT/deployment/production"
VERSION="${VERSION:-$(git rev-parse --short HEAD)}"
ENVIRONMENT="${ENVIRONMENT:-production}"
NAMESPACE="${NAMESPACE:-mcp-production}"

# Colors for output
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

# Error handling
trap 'log_error "Deployment failed at line $LINENO. Exit code: $?"' ERR

# Help function
show_help() {
    cat << EOF
MCP WASM Edge Gateway Deployment Script

Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -e, --environment       Environment (development|staging|production) [default: production]
    -v, --version           Version tag [default: git short hash]
    -n, --namespace         Kubernetes namespace [default: mcp-production]
    -d, --dry-run           Perform a dry run without actual deployment
    -f, --force             Force deployment without confirmation
    -r, --rollback          Rollback to previous version
    -m, --monitoring-only   Deploy only monitoring stack
    -c, --check-health      Check health of deployed services
    --skip-tests            Skip pre-deployment tests
    --skip-build            Skip building Docker images
    --skip-security-scan    Skip security scanning

EXAMPLES:
    $0                                  # Deploy to production
    $0 -e staging -v v1.2.3            # Deploy to staging with specific version
    $0 --dry-run                       # Perform dry run
    $0 --rollback                      # Rollback to previous version
    $0 --check-health                  # Check service health

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -e|--environment)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            -n|--namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -f|--force)
                FORCE=true
                shift
                ;;
            -r|--rollback)
                ROLLBACK=true
                shift
                ;;
            -m|--monitoring-only)
                MONITORING_ONLY=true
                shift
                ;;
            -c|--check-health)
                CHECK_HEALTH=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --skip-security-scan)
                SKIP_SECURITY_SCAN=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Validate environment
validate_environment() {
    log_info "Validating environment and dependencies..."
    
    # Check required tools
    local required_tools=("docker" "kubectl" "helm")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "$tool is required but not installed"
            exit 1
        fi
    done
    
    # Check Kubernetes connectivity
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    
    # Check Docker daemon
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    # Validate environment name
    if [[ ! "$ENVIRONMENT" =~ ^(development|staging|production)$ ]]; then
        log_error "Invalid environment: $ENVIRONMENT"
        exit 1
    fi
    
    log_success "Environment validation passed"
}

# Pre-deployment health checks
pre_deployment_checks() {
    log_info "Running pre-deployment checks..."
    
    # Check cluster resources
    local cpu_requests=$(kubectl describe nodes | grep -A3 "Allocated resources" | grep cpu | awk '{print $2}' | sed 's/[()]//g' | awk -F'%' '{sum += $1} END {print sum/NR}')
    local memory_requests=$(kubectl describe nodes | grep -A3 "Allocated resources" | grep memory | awk '{print $2}' | sed 's/[()]//g' | awk -F'%' '{sum += $1} END {print sum/NR}')
    
    if (( $(echo "$cpu_requests > 80" | bc -l) )); then
        log_warning "Cluster CPU utilization is high: ${cpu_requests}%"
    fi
    
    if (( $(echo "$memory_requests > 85" | bc -l) )); then
        log_warning "Cluster memory utilization is high: ${memory_requests}%"
    fi
    
    # Check namespace exists
    if ! kubectl get namespace "$NAMESPACE" &> /dev/null; then
        log_info "Creating namespace: $NAMESPACE"
        kubectl create namespace "$NAMESPACE"
        kubectl label namespace "$NAMESPACE" name="$NAMESPACE" --overwrite
    fi
    
    log_success "Pre-deployment checks completed"
}

# Run tests
run_tests() {
    if [[ "${SKIP_TESTS:-}" == "true" ]]; then
        log_info "Skipping tests as requested"
        return
    fi
    
    log_info "Running test suite..."
    
    cd "$PROJECT_ROOT"
    
    # Unit tests
    log_info "Running unit tests..."
    cargo test --workspace --exclude integration-tests
    
    # Integration tests
    log_info "Running integration tests..."
    cargo test --package integration-tests
    
    # Security tests
    log_info "Running security tests..."
    cargo test --package security-tests
    
    # Performance benchmarks (quick version)
    log_info "Running performance benchmarks..."
    cargo bench --no-run
    
    log_success "All tests passed"
}

# Security scanning
security_scan() {
    if [[ "${SKIP_SECURITY_SCAN:-}" == "true" ]]; then
        log_info "Skipping security scan as requested"
        return
    fi
    
    log_info "Running security scans..."
    
    # Cargo audit for known vulnerabilities
    if command -v cargo-audit &> /dev/null; then
        log_info "Running cargo audit..."
        cargo audit
    fi
    
    # Check for secrets in code
    if command -v git-secrets &> /dev/null; then
        log_info "Scanning for secrets..."
        git secrets --scan
    fi
    
    log_success "Security scans completed"
}

# Build Docker images
build_images() {
    if [[ "${SKIP_BUILD:-}" == "true" ]]; then
        log_info "Skipping build as requested"
        return
    fi
    
    log_info "Building Docker images..."
    
    cd "$PROJECT_ROOT"
    
    # Build main application image
    log_info "Building MCP Gateway image..."
    docker build \
        --target production \
        --build-arg VERSION="$VERSION" \
        --build-arg BUILD_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --build-arg GIT_COMMIT="$(git rev-parse HEAD)" \
        -t "mcp-gateway:$VERSION" \
        -t "mcp-gateway:latest" \
        .
    
    # Build backup service image
    log_info "Building backup service image..."
    docker build \
        -t "mcp-backup:$VERSION" \
        -t "mcp-backup:latest" \
        "$DEPLOYMENT_DIR/backup/"
    
    # Tag images for registry if specified
    if [[ -n "${DOCKER_REGISTRY:-}" ]]; then
        docker tag "mcp-gateway:$VERSION" "$DOCKER_REGISTRY/mcp-gateway:$VERSION"
        docker tag "mcp-backup:$VERSION" "$DOCKER_REGISTRY/mcp-backup:$VERSION"
        
        log_info "Pushing images to registry..."
        docker push "$DOCKER_REGISTRY/mcp-gateway:$VERSION"
        docker push "$DOCKER_REGISTRY/mcp-backup:$VERSION"
    fi
    
    log_success "Docker images built successfully"
}

# Deploy monitoring stack
deploy_monitoring() {
    log_info "Deploying monitoring stack..."
    
    # Create monitoring namespace
    if ! kubectl get namespace monitoring &> /dev/null; then
        kubectl create namespace monitoring
        kubectl label namespace monitoring name=monitoring --overwrite
    fi
    
    # Deploy Prometheus
    log_info "Deploying Prometheus..."
    kubectl apply -f "$DEPLOYMENT_DIR/monitoring/prometheus/" -n monitoring
    
    # Deploy Grafana
    log_info "Deploying Grafana..."
    kubectl apply -f "$DEPLOYMENT_DIR/monitoring/grafana/" -n monitoring
    
    # Deploy AlertManager
    log_info "Deploying AlertManager..."
    kubectl apply -f "$DEPLOYMENT_DIR/monitoring/alertmanager/" -n monitoring
    
    # Deploy Loki and Promtail
    log_info "Deploying Loki stack..."
    kubectl apply -f "$DEPLOYMENT_DIR/monitoring/loki/" -n monitoring
    
    # Deploy Jaeger
    log_info "Deploying Jaeger..."
    kubectl apply -f "$DEPLOYMENT_DIR/monitoring/jaeger/" -n monitoring
    
    log_success "Monitoring stack deployed"
}

# Deploy database and cache
deploy_infrastructure() {
    log_info "Deploying infrastructure components..."
    
    # Deploy PostgreSQL
    log_info "Deploying PostgreSQL..."
    kubectl apply -f "$DEPLOYMENT_DIR/infrastructure/postgres/" -n "$NAMESPACE"
    
    # Deploy Redis
    log_info "Deploying Redis..."
    kubectl apply -f "$DEPLOYMENT_DIR/infrastructure/redis/" -n "$NAMESPACE"
    
    # Wait for infrastructure to be ready
    log_info "Waiting for infrastructure to be ready..."
    kubectl wait --for=condition=Ready pod -l app=postgres -n "$NAMESPACE" --timeout=300s
    kubectl wait --for=condition=Ready pod -l app=redis -n "$NAMESPACE" --timeout=300s
    
    log_success "Infrastructure deployed successfully"
}

# Deploy main application
deploy_application() {
    log_info "Deploying MCP Gateway application..."
    
    # Apply Kubernetes manifests
    log_info "Applying Kubernetes manifests..."
    
    # Update image tags in deployment
    sed -i.bak "s|image: mcp-gateway:.*|image: mcp-gateway:$VERSION|g" \
        "$DEPLOYMENT_DIR/kubernetes/mcp-gateway-deployment.yaml"
    
    if [[ "${DRY_RUN:-}" == "true" ]]; then
        log_info "DRY RUN: Would apply the following resources:"
        kubectl apply --dry-run=client -f "$DEPLOYMENT_DIR/kubernetes/" -n "$NAMESPACE"
        return
    fi
    
    # Apply resources
    kubectl apply -f "$DEPLOYMENT_DIR/kubernetes/" -n "$NAMESPACE"
    
    # Wait for rollout to complete
    log_info "Waiting for deployment rollout..."
    kubectl rollout status deployment/mcp-gateway -n "$NAMESPACE" --timeout=600s
    
    # Restore original file
    mv "$DEPLOYMENT_DIR/kubernetes/mcp-gateway-deployment.yaml.bak" \
       "$DEPLOYMENT_DIR/kubernetes/mcp-gateway-deployment.yaml"
    
    log_success "Application deployed successfully"
}

# Health check
health_check() {
    log_info "Checking application health..."
    
    # Check pod status
    local ready_pods=$(kubectl get pods -l app=mcp-gateway -n "$NAMESPACE" -o jsonpath='{.items[*].status.conditions[?(@.type=="Ready")].status}' | tr ' ' '\n' | grep -c "True" || true)
    local total_pods=$(kubectl get pods -l app=mcp-gateway -n "$NAMESPACE" --no-headers | wc -l)
    
    log_info "Ready pods: $ready_pods/$total_pods"
    
    if [[ "$ready_pods" -eq 0 ]]; then
        log_error "No pods are ready"
        return 1
    fi
    
    # Check service endpoints
    local service_ip=$(kubectl get service mcp-gateway-service -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
    if [[ -n "$service_ip" ]]; then
        log_info "Service IP: $service_ip"
        
        # Health check endpoint
        if curl -sf "http://$service_ip/health" > /dev/null; then
            log_success "Health check endpoint is responding"
        else
            log_error "Health check endpoint is not responding"
            return 1
        fi
    else
        log_warning "Service IP not yet assigned"
    fi
    
    # Check metrics endpoint
    local pod_name=$(kubectl get pods -l app=mcp-gateway -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    if kubectl exec -n "$NAMESPACE" "$pod_name" -- curl -sf localhost:9090/metrics > /dev/null; then
        log_success "Metrics endpoint is responding"
    else
        log_warning "Metrics endpoint is not responding"
    fi
    
    log_success "Health check completed"
}

# Rollback deployment
rollback_deployment() {
    log_info "Rolling back deployment..."
    
    # Get previous revision
    local previous_revision=$(kubectl rollout history deployment/mcp-gateway -n "$NAMESPACE" | tail -2 | head -1 | awk '{print $1}')
    
    if [[ -z "$previous_revision" ]]; then
        log_error "No previous revision found for rollback"
        exit 1
    fi
    
    log_info "Rolling back to revision $previous_revision"
    kubectl rollout undo deployment/mcp-gateway -n "$NAMESPACE" --to-revision="$previous_revision"
    
    # Wait for rollback to complete
    kubectl rollout status deployment/mcp-gateway -n "$NAMESPACE" --timeout=300s
    
    log_success "Rollback completed successfully"
}

# Cleanup resources
cleanup() {
    log_info "Cleaning up temporary resources..."
    
    # Remove any temporary files
    find "$DEPLOYMENT_DIR" -name "*.bak" -delete 2>/dev/null || true
    
    # Cleanup old images
    if [[ "${CLEANUP_IMAGES:-}" == "true" ]]; then
        docker image prune -f
    fi
}

# Main deployment function
main() {
    local start_time=$(date +%s)
    
    log_info "Starting MCP WASM Edge Gateway deployment"
    log_info "Environment: $ENVIRONMENT"
    log_info "Version: $VERSION"
    log_info "Namespace: $NAMESPACE"
    
    # Handle special operations
    if [[ "${CHECK_HEALTH:-}" == "true" ]]; then
        health_check
        exit 0
    fi
    
    if [[ "${ROLLBACK:-}" == "true" ]]; then
        rollback_deployment
        exit 0
    fi
    
    # Confirmation for production
    if [[ "$ENVIRONMENT" == "production" && "${FORCE:-}" != "true" ]]; then
        read -p "Are you sure you want to deploy to PRODUCTION? (y/N): " -r
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Deployment cancelled"
            exit 0
        fi
    fi
    
    # Execute deployment steps
    validate_environment
    pre_deployment_checks
    
    if [[ "${MONITORING_ONLY:-}" != "true" ]]; then
        run_tests
        security_scan
        build_images
        deploy_infrastructure
        deploy_application
    fi
    
    deploy_monitoring
    health_check
    cleanup
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    log_success "Deployment completed successfully in ${duration}s"
    
    # Display useful information
    echo ""
    echo "=== Deployment Information ==="
    echo "Environment: $ENVIRONMENT"
    echo "Version: $VERSION"
    echo "Namespace: $NAMESPACE"
    echo "Duration: ${duration}s"
    echo ""
    echo "=== Useful Commands ==="
    echo "View pods: kubectl get pods -n $NAMESPACE"
    echo "View services: kubectl get services -n $NAMESPACE"
    echo "View logs: kubectl logs -f deployment/mcp-gateway -n $NAMESPACE"
    echo "Check health: $0 --check-health"
    echo "Rollback: $0 --rollback"
    echo ""
}

# Parse arguments and run main function
parse_args "$@"
main