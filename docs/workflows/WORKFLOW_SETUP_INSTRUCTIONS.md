# GitHub Actions Workflow Setup Instructions

## Overview

This document provides step-by-step instructions for manually adding the GitHub Actions workflows to complete the SDLC automation. Due to GitHub App permission limitations, these workflow files must be added manually.

## Quick Setup

### 1. Create Workflow Directory
```bash
mkdir -p .github/workflows
```

### 2. Add Core Workflows

Copy the following workflow content from `docs/workflows/ci-cd-complete.md` to create these files:

#### Required Workflows
- `.github/workflows/ci.yml` - Core CI pipeline
- `.github/workflows/security.yml` - Security scanning
- `.github/workflows/release.yml` - Release automation
- `.github/workflows/docs.yml` - Documentation deployment

#### Recommended Workflows  
- `.github/workflows/monitoring.yml` - Production monitoring
- `.github/workflows/performance.yml` - Performance testing
- `.github/workflows/modernization.yml` - Technical debt analysis

### 3. Configure Repository Secrets

Go to Repository Settings → Secrets and Variables → Actions:

```bash
# Required secrets
CARGO_REGISTRY_TOKEN=<your_crates_io_token>

# Optional for production monitoring
PRODUCTION_API_KEY=<production_api_key>
MONITORING_WEBHOOK=<slack_discord_webhook>
```

### 4. Enable GitHub Pages

1. Go to Repository Settings → Pages
2. Set Source to "GitHub Actions"
3. Documentation will auto-deploy on main branch changes

### 5. Configure Branch Protection

Repository Settings → Branches → Add rule for `main`:

```yaml
# Required status checks
- "Check"
- "Test Suite" 
- "Clippy"
- "Security Audit"

# Pull request requirements
- Require pull request reviews: 1
- Dismiss stale reviews: true
- Require review from CODEOWNERS: true
```

## Detailed Workflow Descriptions

### ci.yml - Comprehensive CI Pipeline
**Purpose**: Complete testing and validation pipeline
**Features**:
- Unit, integration, and WASM tests
- Cross-platform compilation (Linux, Windows, macOS, ARM64)
- Code formatting and linting
- Performance benchmarking
- Mutation testing for PRs

**Triggers**: Push/PR to main/develop
**Duration**: ~15-20 minutes
**Dependencies**: None

### security.yml - Multi-Layer Security
**Purpose**: Comprehensive security scanning and monitoring
**Features**:
- Cargo audit for Rust vulnerabilities
- CodeQL static analysis
- Container security with Trivy
- Secrets detection with TruffleHog
- SBOM generation
- Dependency review

**Triggers**: Push/PR to main, weekly schedule
**Duration**: ~10-15 minutes
**Dependencies**: Dockerfile

### release.yml - Automated Releases
**Purpose**: Multi-platform release automation
**Features**:
- Cross-platform binary builds (Linux, Windows, macOS, ARM64)
- WASM package generation (web + Node.js)
- Container image publishing to GHCR
- Crates.io publishing
- Automated changelog generation

**Triggers**: Version tags (v*)
**Duration**: ~30-45 minutes
**Dependencies**: git-cliff.toml configuration

### docs.yml - Documentation Deployment
**Purpose**: Automated documentation building and deployment
**Features**:
- Cargo doc API documentation
- mdBook integration
- GitHub Pages deployment
- Link checking

**Triggers**: Push to main (docs changes), PR documentation review
**Duration**: ~5-10 minutes
**Dependencies**: docs/book.toml for mdBook

### monitoring.yml - Production Health
**Purpose**: Production monitoring and health checks
**Features**:
- Health endpoint monitoring
- Load testing with Artillery
- Security advisory monitoring
- Repository metrics collection

**Triggers**: 6-hour schedule, manual dispatch
**Duration**: ~5-10 minutes
**Dependencies**: config/loadtest.yml

### performance.yml - Performance Testing
**Purpose**: Automated performance testing and regression detection
**Features**:
- Comprehensive benchmarking
- Memory profiling with Valgrind
- WASM performance testing
- Cross-platform performance validation
- Binary size monitoring

**Triggers**: Push/PR to main, weekly schedule
**Duration**: ~20-30 minutes
**Dependencies**: benchmark configurations

### modernization.yml - Technical Debt Analysis
**Purpose**: Monthly modernization and technical debt assessment
**Features**:
- Dependency modernization analysis
- Security best practices review
- Performance optimization suggestions
- Rust edition upgrade evaluation

**Triggers**: Monthly schedule, manual dispatch
**Duration**: ~10-15 minutes
**Dependencies**: Various analysis tools

## Configuration Files

### Repository Secrets Setup

```bash
# Generate crates.io token
cargo login
# Copy token from ~/.cargo/credentials

# Add to GitHub repository secrets
CARGO_REGISTRY_TOKEN=<token>
```

### Branch Protection Configuration

```bash
# Using GitHub CLI
gh api repos/:owner/:repo/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["Check","Test Suite","Clippy","Security Audit"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}'
```

## Troubleshooting

### Common Issues

#### 1. Workflow Permission Errors
```
Error: Resource not accessible by integration
```
**Solution**: Check repository Settings → Actions → General → Workflow permissions

#### 2. Missing Secrets
```
Error: Secret CARGO_REGISTRY_TOKEN not found
```
**Solution**: Add required secrets in repository settings

#### 3. Cross-compilation Failures
```
Error: cross build failed
```
**Solution**: Ensure cross-compilation dependencies are properly configured

#### 4. WASM Build Issues
```
Error: wasm-pack not found
```
**Solution**: Verify wasm-pack action version compatibility

### Performance Optimization

#### 1. Rust Cache Configuration
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    # Cache key customization
    key: ${{ matrix.target }}
    # Shared cache across jobs
    shared-key: "stable"
```

#### 2. Parallel Job Execution
- CI jobs run in parallel for faster feedback
- Use `needs:` to create dependencies only when required
- Matrix builds optimize resource usage

#### 3. Artifact Management
```yaml
# Optimize artifact retention
- uses: actions/upload-artifact@v4
  with:
    retention-days: 7  # Reduce for non-critical artifacts
```

## Verification Steps

### 1. Test Workflow Execution
```bash
# Push a small change to trigger CI
git commit --allow-empty -m "test: trigger CI pipeline"
git push
```

### 2. Verify Security Scanning
```bash
# Check security workflow results
gh run list --workflow=security.yml
```

### 3. Test Release Process
```bash
# Create test tag (use pre-release)
git tag v0.1.0-test
git push origin v0.1.0-test
```

### 4. Validate Documentation
```bash
# Check if docs are deployed to GitHub Pages
curl -I https://your-username.github.io/repo-name/
```

## Maintenance

### Weekly Tasks
- Review failed workflow runs
- Update dependency bot PRs
- Monitor security alerts

### Monthly Tasks
- Review modernization reports
- Update workflow versions
- Assess performance trends

### Quarterly Tasks
- Audit security configurations
- Review and update branch protection rules
- Evaluate new GitHub Actions features

## Support

For issues with workflow setup:

1. **Check Documentation**: Review `docs/workflows/ci-cd-complete.md`
2. **GitHub Actions Logs**: Examine workflow run logs for specific errors
3. **Community Support**: GitHub Actions community and Rust forums
4. **Repository Issues**: Create issue with `ci/cd` label

---

This setup provides enterprise-grade automation while maintaining flexibility for customization based on specific project needs.