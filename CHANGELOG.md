# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure and SDLC automation
- Comprehensive development environment setup
- Multi-platform build system (Linux, Windows, ARM64, WASM)
- Docker containerization with multi-stage builds
- Testing framework with unit, integration, and benchmark tests
- Security scanning and compliance tools
- Monitoring and observability with Prometheus and Grafana
- Documentation and operational runbooks
- CI/CD pipeline configuration templates

### Changed
- Migrated from Python-based .gitignore to Rust/WASM-specific patterns

### Security
- Added comprehensive security policy and vulnerability reporting process
- Implemented dependency scanning with cargo-audit and cargo-deny
- Added security-focused linting and code analysis

## [0.1.0] - 2025-01-27

### Added
- Initial release with core SDLC automation
- Project foundation with requirements and architecture documentation
- Development environment with devcontainer support
- Build pipeline for cross-platform compilation
- Security baseline with scanning and compliance tools
- Monitoring foundation with metrics collection
- Comprehensive documentation structure

### Notes
- This is the initial release focusing on SDLC automation and project foundation
- Full gateway implementation to follow in subsequent releases

---

## Release Types

This project uses [Conventional Commits](https://www.conventionalcommits.org/) to automatically determine version numbers:

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality in a backwards compatible manner  
- **PATCH** version for backwards compatible bug fixes

### Commit Types

- `feat`: A new feature (triggers MINOR release)
- `fix`: A bug fix (triggers PATCH release)
- `perf`: A performance improvement (triggers PATCH release)
- `refactor`: Code refactoring (triggers PATCH release)
- `docs`: Documentation changes (may trigger PATCH release)
- `style`: Code style changes (no release)
- `test`: Test changes (no release)
- `chore`: Build process or auxiliary tool changes (no release)
- `BREAKING CHANGE`: Any commit with this footer triggers MAJOR release

### Examples

```
feat: add TPM 2.0 hardware security support
fix: resolve memory leak in request queue
perf: optimize WASM binary size by 15%
docs: add Raspberry Pi deployment guide
refactor: improve error handling consistency

feat!: redesign configuration API
BREAKING CHANGE: configuration format has changed
```