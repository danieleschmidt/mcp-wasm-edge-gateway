# Manual Setup Requirements

## GitHub Repository Configuration

### Branch Protection Rules
- Require status checks to pass before merging
- Require branches to be up to date before merging
- Include administrators in restrictions
- Require linear history
- Allow force pushes (disabled)
- Allow deletions (disabled)

### Required Status Checks
- `ci / test (ubuntu-latest)`
- `ci / test (windows-latest)` 
- `ci / test (macos-latest)`
- `security / cargo-audit`
- `pre-commit.ci`

### Repository Settings
- **Topics**: `mcp`, `edge-computing`, `wasm`, `rust`, `ai`, `iot`
- **Description**: Ultra-lightweight Model Context Protocol gateway for edge devices
- **Homepage**: https://docs.terragon.ai/mcp-edge
- **Issues**: Enabled with templates
- **Wiki**: Disabled
- **Discussions**: Enabled

## GitHub Actions Workflows

Create in `.github/workflows/`:

1. **ci.yml** - Core CI pipeline
2. **security.yml** - Security scanning  
3. **release.yml** - Automated releases
4. **docker.yml** - Container builds

Reference: [Rust CI Templates](https://github.com/actions-rs/meta)

## External Integrations

### Code Quality
- **Codecov** - Coverage reporting
- **CodeClimate** - Code quality metrics
- **Dependabot** - Security updates

### Security
- **CodeQL** - Static analysis
- **Snyk** - Vulnerability scanning
- **FOSSA** - License compliance

## Development Environment

### Pre-commit Hooks
```bash
pre-commit install
pre-commit run --all-files
```

### IDE Configuration
- VSCode: Install Rust Analyzer extension
- Configure rustfmt and clippy integration
- Set up debugging configurations

## Deployment Setup

### Container Registry
- Configure Docker Hub or GitHub Packages
- Set up automated builds on tag push
- Configure vulnerability scanning

### Edge Device Testing
- Set up hardware-in-the-loop testing
- Configure cross-compilation targets
- Establish device deployment pipeline