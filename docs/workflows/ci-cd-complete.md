# Complete CI/CD Workflow Documentation

## Overview

This document provides comprehensive GitHub Actions workflows for the MCP WASM Edge Gateway project. Due to GitHub App permission limitations, these workflows must be added manually to the `.github/workflows/` directory.

## Required Workflows

### 1. Main CI Pipeline (`.github/workflows/ci.yml`)

```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets --all-features

  test:
    name: Test Suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features

  integration-test:
    name: Integration Tests
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --package integration-tests

  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  wasm-build:
    name: WASM Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown, wasm32-wasi
      - uses: jetli/wasm-pack-action@v0.4.0
      - uses: Swatinem/rust-cache@v2
      - run: wasm-pack build --target web --out-dir pkg
      - run: wasm-pack build --target nodejs --out-dir pkg-node
      - run: wasm-pack test --node

  cross-compile:
    name: Cross Compile
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - aarch64-unknown-linux-gnu
          - x86_64-pc-windows-gnu
          - x86_64-apple-darwin
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}
      - uses: taiki-e/install-action@cross
      - run: cross build --target ${{ matrix.target }} --release

  performance:
    name: Performance Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo bench --workspace
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/reports/benchmark.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true

  mutation-test:
    name: Mutation Testing
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-mutants
      - run: cargo mutants --test-tool cargo --timeout 300 --no-shuffle
        continue-on-error: true
      - name: Upload mutation test results
        uses: actions/upload-artifact@v4
        with:
          name: mutation-test-results
          path: mutants.out/
```

### 2. Security Scanning (`.github/workflows/security.yml`)

```yaml
name: Security

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
  schedule:
    - cron: '0 6 * * 1'

jobs:
  security-audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-audit
      - run: cargo audit

  cargo-deny:
    name: Cargo Deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1

  dependency-review:
    name: Dependency Review
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - uses: actions/dependency-review-action@v4

  trivy-scan:
    name: Trivy Container Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Docker image
        run: docker build -t mcp-edge-gateway:${{ github.sha }} .
      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: 'mcp-edge-gateway:${{ github.sha }}'
          format: 'sarif'
          output: 'trivy-results.sarif'
      - name: Upload Trivy scan results
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: 'trivy-results.sarif'

  codeql:
    name: CodeQL Analysis
    runs-on: ubuntu-latest
    permissions:
      actions: read
      contents: read
      security-events: write
    steps:
      - uses: actions/checkout@v4
      - uses: github/codeql-action/init@v3
        with:
          languages: rust
      - uses: github/codeql-action/autobuild@v3
      - uses: github/codeql-action/analyze@v3

  secrets-scan:
    name: Secrets Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: trufflesecurity/trufflehog@main
        with:
          path: ./
          base: main
          head: HEAD
          extra_args: --debug --only-verified

  sbom-generation:
    name: Generate SBOM
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-cyclonedx
      - run: ./scripts/sbom-generate.sh
      - name: Upload SBOM artifacts
        uses: actions/upload-artifact@v4
        with:
          name: sbom-artifacts
          path: artifacts/sbom/
```

### 3. Release Automation (`.github/workflows/release.yml`)

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Generate changelog
        run: |
          cargo install git-cliff
          git-cliff --tag ${{ github.ref_name }} > CHANGELOG.md
      - name: Create Release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          body_path: CHANGELOG.md
          draft: false
          prerelease: false

  build-binaries:
    name: Build Release Binaries
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            asset_name: mcp-gateway-linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            asset_name: mcp-gateway-linux-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            asset_name: mcp-gateway-windows-x64.exe
          - os: macos-latest
            target: x86_64-apple-darwin
            asset_name: mcp-gateway-macos-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            asset_name: mcp-gateway-macos-arm64
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}
      - uses: taiki-e/install-action@cross
      - name: Build binary
        run: |
          if [[ "${{ matrix.target }}" == *"linux"* ]]; then
            cross build --target ${{ matrix.target }} --release
          else
            cargo build --target ${{ matrix.target }} --release
          fi
      - name: Strip binary (Unix)
        if: matrix.os != 'windows-latest'
        run: strip target/${{ matrix.target }}/release/mcp-gateway
      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: target/${{ matrix.target }}/release/mcp-gateway${{ matrix.os == 'windows-latest' && '.exe' || '' }}
          asset_name: ${{ matrix.asset_name }}
          asset_content_type: application/octet-stream

  build-wasm:
    name: Build WASM Package
    needs: create-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: jetli/wasm-pack-action@v0.4.0
      - uses: Swatinem/rust-cache@v2
      - name: Build WASM packages
        run: |
          wasm-pack build --target web --out-dir pkg
          wasm-pack build --target nodejs --out-dir pkg-node
      - name: Package WASM artifacts
        run: |
          tar czf wasm-web.tar.gz pkg/
          tar czf wasm-node.tar.gz pkg-node/
      - name: Upload WASM Web Package
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: wasm-web.tar.gz
          asset_name: mcp-gateway-wasm-web.tar.gz
          asset_content_type: application/gzip
      - name: Upload WASM Node Package
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: wasm-node.tar.gz
          asset_name: mcp-gateway-wasm-node.tar.gz
          asset_content_type: application/gzip

  build-containers:
    name: Build Container Images
    needs: create-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      - name: Login to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  publish-crates:
    name: Publish to crates.io
    needs: create-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Publish crates
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: |
          # Publish crates in dependency order
          cargo publish -p mcp-common
          sleep 30
          cargo publish -p mcp-security
          sleep 30
          cargo publish -p mcp-telemetry
          sleep 30
          cargo publish -p mcp-models
          sleep 30
          cargo publish -p mcp-queue
          sleep 30
          cargo publish -p mcp-router
          sleep 30
          cargo publish -p mcp-gateway
```

### 4. Documentation Deployment (`.github/workflows/docs.yml`)

```yaml
name: Documentation

on:
  push:
    branches: [ main ]
    paths:
      - 'docs/**'
      - 'src/**'
      - 'crates/**/src/**'
      - 'README.md'
      - 'Cargo.toml'
  pull_request:
    branches: [ main ]
    paths:
      - 'docs/**'
      - 'src/**'
      - 'crates/**/src/**'

jobs:
  build-docs:
    name: Build Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build API documentation
        run: cargo doc --workspace --no-deps --all-features
      - name: Build mdBook
        uses: peaceiris/actions-mdbook@v1
        with:
          mdbook-version: 'latest'
      - name: Build book
        run: |
          cd docs
          mdbook build
      - name: Upload documentation artifacts
        uses: actions/upload-artifact@v4
        with:
          name: documentation
          path: |
            target/doc/
            docs/book/

  deploy-docs:
    name: Deploy Documentation
    needs: build-docs
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    permissions:
      contents: read
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Download documentation artifacts
        uses: actions/download-artifact@v4
        with:
          name: documentation
          path: .
      - name: Setup Pages
        uses: actions/configure-pages@v4
      - name: Upload to GitHub Pages
        uses: actions/upload-pages-artifact@v3
        with:
          path: .
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4

  link-check:
    name: Link Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Link Checker
        uses: lycheeverse/lychee-action@v1.8.0
        with:
          args: --verbose --no-progress --exclude-mail '**/*.md' '**/*.rs'
          fail: true
```

### 5. Monitoring and Alerting (`.github/workflows/monitoring.yml`)

```yaml
name: Monitoring

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours
  workflow_dispatch:

jobs:
  health-check:
    name: Health Check Production
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Check production endpoints
        run: |
          endpoints=(
            "https://mcp-gateway.production.com/health"
            "https://mcp-gateway.production.com/metrics"
          )
          
          for endpoint in "${endpoints[@]}"; do
            echo "Checking $endpoint"
            if ! curl -f -s "$endpoint" > /dev/null; then
              echo "❌ $endpoint is down"
              exit 1
            else
              echo "✅ $endpoint is healthy"
            fi
          done

  load-test:
    name: Production Load Test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'
      - name: Install Artillery
        run: npm install -g artillery@latest
      - name: Run load test
        run: |
          # Copy and modify load test config for production
          sed 's/http:\/\/localhost:8080/https:\/\/mcp-gateway.production.com/g' \
            config/loadtest.yml > config/loadtest-prod.yml
          
          # Run shortened load test
          timeout 300s artillery run config/loadtest-prod.yml || true
      - name: Upload load test results
        uses: actions/upload-artifact@v4
        with:
          name: load-test-results
          path: artillery-report.json

  security-monitoring:
    name: Security Monitoring
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check for new vulnerabilities
        run: |
          # Check RustSec advisory database
          cargo install cargo-audit
          cargo audit
          
          # Check if any new high-severity CVEs
          curl -s "https://api.github.com/repos/RustSec/advisory-db/commits" \
            | jq -r '.[0].commit.message' > latest_advisory.txt
          
          if grep -i "high\|critical" latest_advisory.txt; then
            echo "⚠️ New high-severity security advisory detected"
            # In a real setup, this would trigger alerts
          fi

  metrics-collection:
    name: Collect Metrics
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Collect build metrics
        run: |
          # Collect various metrics
          echo "Repository size: $(du -sh . | cut -f1)"
          echo "Last commit: $(git log -1 --format='%h %s')"
          echo "Contributors: $(git shortlog -sn | wc -l)"
          
          # Store in metrics file
          cat > metrics.json << EOF
          {
            "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
            "repository_size": "$(du -sb . | cut -f1)",
            "commit_count": $(git rev-list --count HEAD),
            "contributor_count": $(git shortlog -sn | wc -l),
            "open_issues": $(curl -s "https://api.github.com/repos/${{ github.repository }}/issues?state=open" | jq length),
            "closed_issues": $(curl -s "https://api.github.com/repos/${{ github.repository }}/issues?state=closed" | jq length)
          }
          EOF
      - name: Upload metrics
        uses: actions/upload-artifact@v4
        with:
          name: repository-metrics
          path: metrics.json
```

## Deployment Instructions

### 1. Create Workflow Files

Create the `.github/workflows/` directory and add each workflow file:

```bash
mkdir -p .github/workflows
# Copy each workflow YAML content above into respective files
```

### 2. Configure Repository Secrets

Add the following secrets in your GitHub repository settings:

- `CARGO_REGISTRY_TOKEN` - For publishing to crates.io
- `PRODUCTION_API_KEY` - For production deployments
- `MONITORING_WEBHOOK` - For alerting integrations

### 3. Enable GitHub Pages

1. Go to repository Settings → Pages
2. Set source to "GitHub Actions"
3. The documentation will be automatically deployed

### 4. Configure Branch Protection

Set up branch protection rules for `main`:

```yaml
# Example branch protection settings
required_status_checks:
  - "Check"
  - "Test Suite"
  - "Clippy"
  - "Security Audit"
enforce_admins: true
required_pull_request_reviews:
  required_approving_review_count: 1
dismiss_stale_reviews: true
```

### 5. Set Up Notifications

Configure Slack/Discord webhooks for:
- Failed CI runs
- Security alerts
- Release notifications
- Performance regression alerts

## Workflow Features

### Comprehensive Testing
- Unit tests, integration tests, WASM tests
- Mutation testing for code quality
- Cross-platform compilation
- Performance benchmarking

### Security First
- Multiple security scanning tools
- SBOM generation
- Container vulnerability scanning
- Secrets detection
- Dependency review

### Release Automation
- Automated changelog generation
- Multi-platform binary builds
- Container image publishing
- Package publishing to registries

### Monitoring & Observability
- Production health checks
- Load testing
- Metrics collection
- Security monitoring

### Documentation
- API documentation generation
- mdBook integration
- Link checking
- Automated deployment

## Customization

### Environment-Specific Configurations

Modify the workflows for your specific deployment environments:

```yaml
# Add environment-specific jobs
deploy-staging:
  environment: staging
  # ... staging deployment steps

deploy-production:
  environment: production
  needs: [staging-tests]
  # ... production deployment steps
```

### Custom Notifications

Add notification steps to workflows:

```yaml
- name: Notify on failure
  if: failure()
  uses: 8398a7/action-slack@v3
  with:
    status: failure
    webhook_url: ${{ secrets.SLACK_WEBHOOK }}
```

### Performance Thresholds

Set up performance regression detection:

```yaml
- name: Check performance regression
  run: |
    # Compare benchmark results with baseline
    # Fail if performance decreased by >10%
```

## Best Practices

1. **Incremental Rollout**: Deploy workflows gradually
2. **Test in Fork**: Test workflow changes in a fork first
3. **Monitor Costs**: GitHub Actions minutes have limits
4. **Cache Optimization**: Use Rust cache effectively
5. **Parallel Execution**: Optimize job dependencies for speed
6. **Security**: Use OIDC tokens instead of long-lived secrets
7. **Observability**: Add comprehensive logging and metrics

This complete CI/CD setup provides enterprise-grade automation while maintaining security, performance, and reliability standards.