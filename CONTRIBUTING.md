# Contributing to MCP WASM Edge Gateway

We welcome contributions to the MCP WASM Edge Gateway project! This document provides guidelines for contributing to ensure a smooth collaboration process.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please read it before contributing.

## Getting Started

### Prerequisites

- **Rust** 1.75+ with Cargo
- **Node.js** 18+ and npm (for tooling)
- **Docker** and Docker Compose (for testing)
- **Git** with LFS support

### Development Setup

1. **Fork and clone the repository**:
   ```bash
   git clone https://github.com/your-username/mcp-wasm-edge-gateway
   cd mcp-wasm-edge-gateway
   ```

2. **Set up development environment**:
   ```bash
   make setup
   npm install
   ```

3. **Start development environment**:
   ```bash
   make dev
   ```

4. **Run tests to verify setup**:
   ```bash
   make test
   ```

## How to Contribute

### Reporting Issues

Before creating an issue, please:

1. **Search existing issues** to avoid duplicates
2. **Use the issue template** provided
3. **Provide clear reproduction steps** for bugs
4. **Include system information** (OS, Rust version, etc.)

#### Bug Reports

Use the bug report template and include:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- Environment details
- Relevant logs or error messages

#### Feature Requests

Use the feature request template and include:
- Clear description of the feature
- Use case and motivation
- Proposed implementation (if any)
- Alternatives considered

### Contributing Code

#### 1. Create an Issue

For significant changes, create an issue first to discuss:
- The problem you're solving
- Your proposed approach
- Any breaking changes

#### 2. Fork and Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

#### 3. Make Changes

Follow our [coding standards](#coding-standards) and:

- Write clear, self-documenting code
- Add tests for new functionality
- Update documentation as needed
- Ensure all tests pass
- Follow conventional commit format

#### 4. Test Your Changes

```bash
# Run all tests
make test-all

# Run linting
make lint

# Check formatting
make format-check

# Security audit
make audit

# Full CI pipeline
make ci
```

#### 5. Submit Pull Request

- Use the pull request template
- Reference related issues
- Provide clear description of changes
- Include testing notes
- Ensure CI passes

## Coding Standards

### Rust Code Style

We follow standard Rust conventions:

- **Use `rustfmt`** for formatting (configured in `rustfmt.toml`)
- **Address all `clippy` warnings** (configured in `clippy.toml`)
- **Write comprehensive tests** for new functionality
- **Document public APIs** with doc comments
- **Use meaningful variable names** and avoid abbreviations

### Code Organization

```rust
// Module structure
pub mod config;
pub mod gateway;
pub mod router;
// ... other modules

// Error handling
use anyhow::Result;
use thiserror::Error;

// Async code
use tokio::prelude::*;

// Logging
use tracing::{debug, info, warn, error};
```

### Documentation

- **API documentation**: Use `///` doc comments for public items
- **Module documentation**: Use `//!` for module-level docs
- **Examples**: Include usage examples in doc comments
- **README updates**: Keep README.md current with changes

### Testing

- **Unit tests**: Test individual functions and methods
- **Integration tests**: Test complete workflows
- **Property tests**: Use `proptest` for property-based testing
- **Benchmarks**: Add benchmarks for performance-critical code

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_functionality() {
        // Test implementation
    }
    
    #[tokio::test]
    async fn test_async_functionality() {
        // Async test implementation
    }
}
```

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

#### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Build process or auxiliary tool changes
- `perf`: Performance improvements
- `ci`: CI configuration changes

#### Examples

```
feat(router): add complexity-based routing algorithm

Implements a new routing algorithm that analyzes request complexity
to determine whether to use local or cloud processing.

Closes #123

fix(queue): resolve memory leak in offline queue manager

The queue manager was not properly cleaning up completed requests,
causing memory usage to grow over time.

docs: update deployment guide for Raspberry Pi

Add section on GPIO configuration and power management for
Raspberry Pi deployments.

BREAKING CHANGE: configuration format has changed

The gateway configuration now uses TOML format instead of JSON.
See migration guide in docs/migration.md
```

## Pull Request Process

### 1. Before Submitting

- [ ] All tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] No merge conflicts with main branch

### 2. Review Process

1. **Automated checks**: CI must pass
2. **Code review**: At least one reviewer approval
3. **Security review**: Required for security-related changes
4. **Documentation review**: Required for public API changes

### 3. Merge Requirements

- All CI checks passing
- At least one approval from a maintainer
- No unresolved review comments
- Up-to-date with main branch

## Development Workflow

### Branch Strategy

- **`main`**: Production-ready code
- **`develop`**: Integration branch for features
- **`feature/*`**: Feature development branches
- **`fix/*`**: Bug fix branches
- **`hotfix/*`**: Critical production fixes

### Release Process

1. Features merged to `develop`
2. Release branch created from `develop`
3. Testing and bug fixes on release branch
4. Release branch merged to `main` and tagged
5. `main` merged back to `develop`

## Platform-Specific Contributions

### Raspberry Pi

- Test on actual hardware when possible
- Consider power consumption implications
- Verify GPIO functionality if applicable
- Document hardware-specific requirements

### ESP32/Arduino

- Use appropriate memory constraints
- Test on actual ESP32 devices
- Follow embedded best practices
- Document flash/RAM usage

### WASM

- Test in multiple browsers
- Verify Node.js compatibility
- Consider bundle size implications
- Test offline functionality

## Performance Considerations

- **Benchmark changes**: Use `cargo bench` for performance testing
- **Memory usage**: Consider memory-constrained environments
- **Binary size**: Keep WASM binaries under 3MB
- **Startup time**: Optimize for fast initialization
- **Power efficiency**: Important for battery-powered devices

## Security Guidelines

- **No secrets in code**: Use environment variables or secure storage
- **Input validation**: Validate all external inputs
- **Error handling**: Don't leak sensitive information in errors
- **Dependencies**: Keep dependencies updated and audited
- **Code review**: Security-sensitive changes require extra review

## Documentation Standards

### API Documentation

```rust
/// Processes an MCP request through the gateway.
///
/// This function routes the request based on complexity analysis and
/// available resources. Simple requests are processed locally while
/// complex requests may be forwarded to cloud services.
///
/// # Arguments
///
/// * `request` - The MCP request to process
///
/// # Returns
///
/// Returns a `Result` containing the processed response or an error.
///
/// # Errors
///
/// This function will return an error if:
/// - The request format is invalid
/// - Network connectivity issues occur
/// - Resource limits are exceeded
///
/// # Examples
///
/// ```rust
/// use mcp_gateway::{Gateway, MCPRequest};
///
/// let gateway = Gateway::new(config).await?;
/// let request = MCPRequest::new("Hello, world!");
/// let response = gateway.process_request(request).await?;
/// ```
pub async fn process_request(&self, request: MCPRequest) -> Result<MCPResponse> {
    // Implementation
}
```

### User Guides

- **Clear structure**: Introduction, prerequisites, steps, troubleshooting
- **Real examples**: Include working code and configuration samples
- **Screenshots**: Use screenshots for UI-related guides
- **Testing steps**: Include verification steps
- **Next steps**: Guide users to related documentation

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and community discussions
- **Discord**: Real-time chat and community support
- **Email**: security@terragon.ai for security issues

### Getting Help

1. **Check documentation**: Start with the guides and API docs
2. **Search issues**: Look for existing solutions
3. **Ask in discussions**: For general questions
4. **Create an issue**: For bugs or feature requests

### Recognition

Contributors are recognized in:
- **CONTRIBUTORS.md**: List of all contributors
- **Release notes**: Major contributions highlighted
- **Documentation**: Author attribution where appropriate

## License

By contributing to this project, you agree that your contributions will be licensed under the [MIT License](LICENSE).

## Questions?

If you have questions about contributing, please:
1. Check this document and other documentation
2. Search existing issues and discussions
3. Ask in GitHub Discussions
4. Contact the maintainers

Thank you for contributing to MCP WASM Edge Gateway! 🚀