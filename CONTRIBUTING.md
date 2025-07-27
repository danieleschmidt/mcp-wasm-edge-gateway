# Contributing to MCP WASM Edge Gateway

Thank you for your interest in contributing to the MCP WASM Edge Gateway! This document provides guidelines and information about contributing to this project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Style Guidelines](#style-guidelines)
- [Security](#security)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Rust 1.75+ with `rustfmt` and `clippy`
- Node.js 18+ (for web tooling)
- Docker (for containerized development)
- Git

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/mcp-wasm-edge-gateway.git
   cd mcp-wasm-edge-gateway
   ```

2. **Development Container (Recommended)**
   ```bash
   # Open in VS Code with Dev Containers extension
   code .
   # VS Code will prompt to reopen in container
   ```

3. **Local Setup**
   ```bash
   # Install Rust targets
   rustup target add wasm32-wasi wasm32-unknown-unknown

   # Install tools
   npm run install:tools

   # Install pre-commit hooks
   pre-commit install
   ```

4. **Verify Setup**
   ```bash
   cargo check --workspace
   cargo test --workspace
   npm run build:wasm
   ```

## Making Changes

### Branching Strategy

- `main` - Stable release branch
- `develop` - Integration branch for features
- `feature/name` - Feature development
- `bugfix/name` - Bug fixes
- `hotfix/name` - Critical fixes for production

### Workflow

1. **Create a Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clear, concise commit messages
   - Keep commits atomic and focused
   - Include tests for new functionality

3. **Test Locally**
   ```bash
   npm run ci  # Runs format, lint, test, audit
   ```

4. **Push and Create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

## Testing

### Test Categories

1. **Unit Tests**
   ```bash
   cargo test --lib
   ```

2. **Integration Tests**
   ```bash
   cargo test --test integration
   ```

3. **WASM Tests**
   ```bash
   wasm-pack test --node
   ```

4. **Performance Tests**
   ```bash
   cargo bench
   ```

### Writing Tests

- **Unit tests**: Place in `src/` files using `#[cfg(test)]` modules
- **Integration tests**: Place in `tests/` directory
- **Documentation tests**: Include examples in doc comments

Example test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_initialization() {
        let config = Config::default();
        let gateway = Gateway::new(config).await;
        assert!(gateway.is_ok());
    }
}
```

## Submitting Changes

### Pull Request Process

1. **Before Submitting**
   - Ensure all tests pass
   - Update documentation if needed
   - Add changelog entry
   - Verify WASM build succeeds

2. **PR Title Format**
   ```
   feat: add support for ESP32 platform
   fix: resolve memory leak in model cache
   docs: update installation guide
   test: add integration tests for offline mode
   ```

3. **PR Description Template**
   ```markdown
   ## Summary
   Brief description of changes

   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update

   ## Testing
   - [ ] Unit tests pass
   - [ ] Integration tests pass
   - [ ] WASM build succeeds
   - [ ] Manual testing completed

   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Self-review completed
   - [ ] Documentation updated
   - [ ] Changelog updated
   ```

### Review Process

1. **Automated Checks**: All CI checks must pass
2. **Code Review**: At least one maintainer approval required
3. **Testing**: Thorough testing on multiple platforms
4. **Documentation**: Documentation must be updated for user-facing changes

## Style Guidelines

### Rust Code Style

- **Formatting**: Use `cargo fmt` (configured in `rustfmt.toml`)
- **Linting**: Follow `cargo clippy` recommendations
- **Naming**: Use `snake_case` for functions/variables, `PascalCase` for types
- **Comments**: Document public APIs with doc comments

Example:
```rust
/// Processes an MCP request and returns a response.
///
/// # Arguments
/// * `request` - The MCP request to process
///
/// # Returns
/// * `Result<MCPResponse, Error>` - The response or error
///
/// # Examples
/// ```
/// let request = MCPRequest::new("completion", params);
/// let response = gateway.process_request(request).await?;
/// ```
pub async fn process_request(&self, request: MCPRequest) -> Result<MCPResponse, Error> {
    // Implementation
}
```

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes bug nor adds feature
- `test`: Adding missing tests
- `chore`: Changes to build process or auxiliary tools

### Documentation Style

- **API Documentation**: Use Rust doc comments with examples
- **User Documentation**: Clear, concise markdown with code examples
- **Architecture Documentation**: Include diagrams where helpful

## Security

### Security Guidelines

1. **Never commit secrets** - Use environment variables or secure vaults
2. **Validate inputs** - Always validate and sanitize user inputs
3. **Use safe APIs** - Prefer safe Rust APIs over unsafe blocks
4. **Review dependencies** - Check new dependencies for security issues

### Reporting Security Issues

Please see our [Security Policy](SECURITY.md) for reporting vulnerabilities.

## Performance Considerations

### Optimization Guidelines

1. **Memory Usage**: Monitor memory usage, especially on edge devices
2. **Binary Size**: Keep WASM binary under 3MB target
3. **Latency**: Optimize for < 200ms request latency
4. **Power**: Consider power consumption on battery devices

### Benchmarking

Always benchmark performance-critical changes:

```bash
cargo bench
# Check results in target/criterion/reports/
```

## Documentation

### Types of Documentation

1. **API Documentation**: Auto-generated from code comments
2. **User Guides**: Step-by-step tutorials
3. **Architecture Documentation**: System design and decisions
4. **Deployment Guides**: Platform-specific instructions

### Building Documentation

```bash
# API documentation
cargo doc --workspace --no-deps --open

# User documentation (if using mdBook)
mdbook build docs
mdbook serve docs  # For live preview
```

## Community

### Communication Channels

- **GitHub Discussions**: For questions and general discussion
- **GitHub Issues**: For bug reports and feature requests
- **Discord**: [Terragon Labs Community](https://discord.gg/terragon)
- **Email**: [dev@terragon.ai](mailto:dev@terragon.ai)

### Getting Help

1. **Check Documentation**: Start with README and docs
2. **Search Issues**: Look for existing issues or discussions
3. **Ask Questions**: Use GitHub Discussions for questions
4. **Join Community**: Join our Discord for real-time help

### Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Invited to contributor events
- Eligible for contributor rewards

## Project Structure

```
mcp-wasm-edge-gateway/
├── src/                    # Source code
├── crates/                 # Workspace crates
├── tests/                  # Integration tests
├── docs/                   # Documentation
├── examples/               # Example code
├── config/                 # Configuration files
├── scripts/                # Build and utility scripts
├── .github/                # GitHub workflows and templates
└── benchmarks/             # Performance benchmarks
```

## Release Process

### Version Numbers

We follow [Semantic Versioning](https://semver.org/):
- `MAJOR.MINOR.PATCH`
- Breaking changes increment MAJOR
- New features increment MINOR
- Bug fixes increment PATCH

### Release Checklist

1. Update version numbers
2. Update CHANGELOG.md
3. Create release PR
4. Tag release after merge
5. Publish packages
6. Update documentation

## Advanced Topics

### Cross-Platform Development

- Test on multiple platforms (Linux, Windows, macOS)
- Use platform-specific feature flags
- Consider resource constraints of target devices

### WASM Development

- Keep binary size minimal
- Use `wee_alloc` for small allocator
- Optimize for startup time
- Test in different WASM runtimes

### Embedded Development

- Use `no_std` where possible
- Minimize memory allocations
- Consider real-time constraints
- Test on actual hardware

## Questions?

If you have any questions not covered here:

1. Check existing documentation
2. Search GitHub issues and discussions
3. Ask in our community channels
4. Create a new discussion or issue

Thank you for contributing to MCP WASM Edge Gateway! 🦀