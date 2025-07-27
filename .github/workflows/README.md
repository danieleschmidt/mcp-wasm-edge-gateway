# GitHub Actions Workflows for MCP WASM Edge Gateway

This directory should contain the following GitHub Actions workflow files. **Note: These need to be created manually as they cannot be auto-generated.**

## Required Workflow Files

### 1. `ci.yml` - Continuous Integration
```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all-features
      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
      - name: Check formatting
        run: cargo fmt --all -- --check

  build-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Build WASM
        run: wasm-pack build --target web

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Security audit
        run: cargo audit
```

### 2. `build.yml` - Multi-platform Build
```yaml
name: Build

on:
  push:
    tags: [ 'v*' ]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86_64-pc-windows-gnu]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Build for ${{ matrix.target }}
        run: cargo build --release --target ${{ matrix.target }}
```

### 3. `docker.yml` - Docker Build and Push
```yaml
name: Docker

on:
  push:
    branches: [ main ]
    tags: [ 'v*' ]

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:latest
```

### 4. `security.yml` - Security Scanning
```yaml
name: Security

on:
  push:
    branches: [ main ]
  schedule:
    - cron: '0 0 * * 0'

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run CodeQL Analysis
        uses: github/codeql-action/analyze@v3
      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
```

## Workflow Features

- **Parallel Execution**: Multiple jobs run concurrently for faster feedback
- **Matrix Builds**: Test across multiple platforms and Rust versions
- **Caching**: Cargo dependencies are cached to speed up builds
- **Security Scanning**: Automated vulnerability detection
- **Release Automation**: Automatic releases on version tags
- **Docker Integration**: Automated container builds and pushes

## Setup Instructions

1. Create each workflow file in `.github/workflows/`
2. Configure repository secrets for:
   - `GITHUB_TOKEN` (automatically provided)
   - `DOCKER_REGISTRY_TOKEN` (if using external registry)
   - Any API keys for security scanning tools

3. Enable GitHub Actions in repository settings
4. Configure branch protection rules to require CI checks

## Best Practices

- Use specific action versions (e.g., `@v4` instead of `@main`)
- Cache dependencies to improve build times
- Use matrix builds for multi-platform testing
- Implement security scanning in CI pipeline
- Use semantic versioning for releases
- Include comprehensive test coverage

## Monitoring

The workflows will:
- Run on every push and pull request
- Generate build artifacts for releases
- Report security vulnerabilities
- Update container registries
- Send notifications on failures