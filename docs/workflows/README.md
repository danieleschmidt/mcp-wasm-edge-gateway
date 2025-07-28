# Workflow Requirements

## Required GitHub Actions Workflows

The following workflows need manual setup due to permission limitations:

### Core CI/CD Workflows
- **ci.yml** - Continuous integration with Rust/WASM builds
- **security.yml** - Security scanning and dependency audits  
- **release.yml** - Automated releases with cargo-release
- **cross-platform.yml** - Multi-architecture builds (ARM, x86_64)

### Quality Assurance
- **pre-commit.yml** - Pre-commit hook enforcement
- **dependabot.yml** - Automated dependency updates
- **codeql.yml** - Static security analysis

### Deployment
- **docker.yml** - Container builds and registry pushes
- **embedded.yml** - Embedded device testing (ESP32, Raspberry Pi)

## Manual Setup Required

1. Create `.github/workflows/` directory
2. Reference [GitHub Actions for Rust](https://github.com/actions-rs/meta)
3. Configure branch protection rules
4. Set up required status checks
5. Configure deployment environments

## Documentation Links

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [WASM Toolchain Actions](https://rustwasm.github.io/book/reference/add-wasm-support-to-cargo-project.html)

## Security Considerations

- Use GitHub secrets for sensitive data
- Implement OIDC authentication for cloud deployments
- Enable security scanning on all workflows
- Configure artifact retention policies

## Integration Requirements

See `docs/SETUP_REQUIRED.md` for complete manual setup checklist.