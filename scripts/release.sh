#!/bin/bash
# Release script for MCP WASM Edge Gateway
# Automates the complete release process

set -e

# Configuration
PROJECT_NAME="mcp-wasm-edge-gateway"
REGISTRY="ghcr.io/your-org"
RELEASE_DIR="releases"

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
MCP WASM Edge Gateway Release Script

Usage: $0 [OPTIONS] [VERSION]

Options:
    -h, --help          Show this help message
    -d, --dry-run       Dry run mode (no actual release)
    -p, --push          Push Docker images to registry
    -g, --github        Create GitHub release
    --skip-tests        Skip running tests
    --skip-build        Skip building artifacts
    --pre-release       Mark as pre-release

Arguments:
    VERSION             Version to release (e.g., 1.0.0)
                       If not provided, will be auto-determined

Examples:
    $0                      # Auto-release based on commits
    $0 1.0.0               # Release specific version
    $0 --dry-run           # Test release process
    $0 --pre-release       # Create pre-release
EOF
}

# Parse command line arguments
VERSION=""
DRY_RUN=false
PUSH_DOCKER=false
CREATE_GITHUB_RELEASE=false
SKIP_TESTS=false
SKIP_BUILD=false
PRE_RELEASE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -p|--push)
            PUSH_DOCKER=true
            shift
            ;;
        -g|--github)
            CREATE_GITHUB_RELEASE=true
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
        --pre-release)
            PRE_RELEASE=true
            shift
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                log_error "Unknown option: $1"
                show_help
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate environment
validate_environment() {
    log_info "Validating environment..."
    
    # Check required tools
    local required_tools=("cargo" "git" "docker")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "$tool is required but not installed"
            exit 1
        fi
    done
    
    # Check git status
    if [[ -n $(git status --porcelain) ]]; then
        log_error "Working directory is not clean"
        git status --porcelain
        exit 1
    fi
    
    # Check current branch
    local current_branch=$(git branch --show-current)
    if [[ "$current_branch" != "main" && "$current_branch" != "develop" ]]; then
        log_warning "Not on main or develop branch (currently on: $current_branch)"
    fi
    
    log_success "Environment validation completed"
}

# Determine version
determine_version() {
    if [[ -z "$VERSION" ]]; then
        log_info "Auto-determining version from commits..."
        
        # Use semantic-release in dry-run mode to determine next version
        if command -v semantic-release &> /dev/null; then
            VERSION=$(npx semantic-release --dry-run 2>/dev/null | grep "The next release version is" | sed 's/.*is \(.*\)/\1/')
        fi
        
        # Fallback: extract from Cargo.toml and increment patch
        if [[ -z "$VERSION" ]]; then
            local current_version=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
            log_warning "Could not auto-determine version, using current: $current_version"
            VERSION="$current_version"
        fi
    fi
    
    log_info "Release version: $VERSION"
}

# Update version in files
update_version() {
    log_info "Updating version to $VERSION..."
    
    if [[ "$DRY_RUN" == false ]]; then
        # Update Cargo.toml
        sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
        
        # Update package.json
        if [[ -f "package.json" ]]; then
            sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json
        fi
        
        # Update lock file
        cargo update --workspace
        
        log_success "Version updated in project files"
    else
        log_info "[DRY RUN] Would update version to $VERSION"
    fi
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        log_warning "Skipping tests as requested"
        return
    fi
    
    log_info "Running test suite..."
    
    # Run linting
    cargo clippy --all-targets --all-features -- -D warnings
    
    # Run tests
    cargo test --all-features
    
    # Run security audit
    cargo audit
    
    log_success "All tests passed"
}

# Build artifacts
build_artifacts() {
    if [[ "$SKIP_BUILD" == true ]]; then
        log_warning "Skipping build as requested"
        return
    fi
    
    log_info "Building release artifacts..."
    
    # Clean previous builds
    cargo clean
    rm -rf "$RELEASE_DIR/$VERSION"
    
    # Build all platforms
    ./scripts/build.sh release
    
    log_success "Release artifacts built"
}

# Build Docker images
build_docker() {
    log_info "Building Docker images..."
    
    local image_name="$REGISTRY/$PROJECT_NAME"
    
    # Build production image
    docker build -t "$image_name:$VERSION" -t "$image_name:latest" .
    
    # Build platform-specific images
    docker buildx build --platform linux/amd64,linux/arm64 \
        -t "$image_name:$VERSION-multiarch" \
        --push="$PUSH_DOCKER" .
    
    log_success "Docker images built"
}

# Push Docker images
push_docker() {
    if [[ "$PUSH_DOCKER" == false ]]; then
        log_info "Skipping Docker push"
        return
    fi
    
    log_info "Pushing Docker images..."
    
    local image_name="$REGISTRY/$PROJECT_NAME"
    
    if [[ "$DRY_RUN" == false ]]; then
        docker push "$image_name:$VERSION"
        docker push "$image_name:latest"
        log_success "Docker images pushed"
    else
        log_info "[DRY RUN] Would push Docker images"
    fi
}

# Create GitHub release
create_github_release() {
    if [[ "$CREATE_GITHUB_RELEASE" == false ]]; then
        log_info "Skipping GitHub release creation"
        return
    fi
    
    log_info "Creating GitHub release..."
    
    if ! command -v gh &> /dev/null; then
        log_error "GitHub CLI (gh) is required for creating releases"
        return 1
    fi
    
    local release_notes_file="release-notes-$VERSION.md"
    local release_flags=""
    
    if [[ "$PRE_RELEASE" == true ]]; then
        release_flags="--prerelease"
    fi
    
    # Generate release notes
    generate_release_notes > "$release_notes_file"
    
    if [[ "$DRY_RUN" == false ]]; then
        # Create release with artifacts
        gh release create "v$VERSION" \
            --title "Release v$VERSION" \
            --notes-file "$release_notes_file" \
            $release_flags \
            "$RELEASE_DIR/$VERSION"/*
            
        # Clean up
        rm "$release_notes_file"
        
        log_success "GitHub release created"
    else
        log_info "[DRY RUN] Would create GitHub release"
        log_info "Release notes:"
        cat "$release_notes_file"
        rm "$release_notes_file"
    fi
}

# Generate release notes
generate_release_notes() {
    cat << EOF
## What's Changed

$(git log --pretty=format:"- %s" $(git describe --tags --abbrev=0)..HEAD)

## Artifacts

This release includes the following artifacts:

- **Linux x86_64**: \`gateway-linux-x86_64\`
- **Linux ARM64**: \`gateway-linux-aarch64\` (Raspberry Pi, Jetson)
- **Windows x86_64**: \`gateway-windows-x86_64.exe\`
- **WASM Web**: \`gateway-wasm-web.tar.gz\`
- **WASM Node.js**: \`gateway-wasm-node.tar.gz\`
- **Checksums**: \`checksums.txt\`

## Docker Images

\`\`\`bash
docker pull $REGISTRY/$PROJECT_NAME:$VERSION
docker pull $REGISTRY/$PROJECT_NAME:latest
\`\`\`

## Installation

See the [Installation Guide](https://github.com/your-org/$PROJECT_NAME/blob/main/docs/guides/installation.md) for detailed instructions.

**Full Changelog**: https://github.com/your-org/$PROJECT_NAME/compare/v$(git describe --tags --abbrev=0)...v$VERSION
EOF
}

# Commit and tag
commit_and_tag() {
    log_info "Creating git commit and tag..."
    
    if [[ "$DRY_RUN" == false ]]; then
        # Add updated files
        git add Cargo.toml Cargo.lock package.json CHANGELOG.md
        
        # Create commit
        git commit -m "chore(release): $VERSION

Release artifacts:
- Linux x86_64 and ARM64 binaries
- Windows x86_64 binary
- WASM packages for web and Node.js
- Docker images with multi-platform support

🤖 Generated with Claude Code
"
        
        # Create tag
        git tag -a "v$VERSION" -m "Release v$VERSION"
        
        log_success "Git commit and tag created"
    else
        log_info "[DRY RUN] Would create commit and tag for v$VERSION"
    fi
}

# Push changes
push_changes() {
    log_info "Pushing changes to repository..."
    
    if [[ "$DRY_RUN" == false ]]; then
        git push origin main
        git push origin "v$VERSION"
        log_success "Changes pushed to repository"
    else
        log_info "[DRY RUN] Would push changes to repository"
    fi
}

# Cleanup
cleanup() {
    log_info "Cleaning up temporary files..."
    # Add any cleanup tasks here
    log_success "Cleanup completed"
}

# Main execution
main() {
    log_info "Starting release process for $PROJECT_NAME"
    
    # Trap cleanup on exit
    trap cleanup EXIT
    
    validate_environment
    determine_version
    update_version
    run_tests
    build_artifacts
    build_docker
    push_docker
    commit_and_tag
    push_changes
    create_github_release
    
    log_success "Release process completed successfully!"
    log_info "Version $VERSION has been released"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_warning "This was a dry run - no actual changes were made"
    fi
}

# Execute main function
main "$@"