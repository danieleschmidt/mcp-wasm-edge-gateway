# Security Policy

## Supported Versions

We currently support the following versions with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it to us responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via:

1. **Email**: Send details to [security@terragon.ai](mailto:security@terragon.ai)
2. **GitHub Security Advisories**: Use our [private vulnerability reporting](https://github.com/terragon-labs/mcp-wasm-edge-gateway/security/advisories/new)

### What to Include

Please include the following information in your report:

- Type of issue (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- Full paths of source file(s) related to the manifestation of the issue
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit the issue

### Response Timeline

- **Initial Response**: Within 24 hours of receiving your report
- **Confirmation**: Within 72 hours, we'll confirm receipt and provide an initial assessment
- **Updates**: Regular updates every 5 business days until resolution
- **Resolution**: We aim to resolve critical vulnerabilities within 7 days, high severity within 30 days

### Disclosure Policy

- We request that you give us adequate time to address the vulnerability before any public disclosure
- We will publicly acknowledge your responsible disclosure (unless you prefer to remain anonymous)
- We will coordinate the timing of any public disclosure with you

## Security Measures

### Development Security

- **Secure Coding**: All code follows secure coding practices and undergoes security review
- **Dependency Scanning**: Automated scanning for known vulnerabilities in dependencies
- **Static Analysis**: Code is analyzed for security issues using multiple tools
- **Fuzzing**: Critical components undergo fuzz testing
- **Penetration Testing**: Regular security assessments by third-party experts

### Runtime Security

- **Memory Safety**: Written in Rust for memory safety guarantees
- **Sandboxing**: WASM sandboxing provides additional isolation
- **Principle of Least Privilege**: Components run with minimal required permissions
- **Hardware Security**: Support for TPM 2.0 and hardware security modules
- **Encryption**: All communication encrypted with TLS 1.3+
- **Authentication**: Strong device authentication and attestation

### Infrastructure Security

- **Container Security**: Minimal container images with no unnecessary components
- **Network Security**: Network isolation and traffic encryption
- **Secret Management**: Secure handling of API keys and certificates
- **Monitoring**: Comprehensive security monitoring and alerting
- **Compliance**: Regular compliance audits and certifications

## Security Features

### Hardware Security Module (HSM) Support

- TPM 2.0 integration for device attestation
- Secure key generation and storage
- Hardware-backed device identity
- Secure boot verification

### Zero-Trust Architecture

- Device identity verification for every request
- Policy-based access control
- Risk-based authentication
- Comprehensive audit logging

### Encryption

- **At Rest**: AES-256 encryption for stored data
- **In Transit**: TLS 1.3 for all network communication
- **Key Management**: Hardware-backed key derivation and rotation

### Edge-Specific Security

- Offline security validation
- Secure model loading and verification
- Resource constraint handling
- Power-aware security features

## Security Configuration

### Recommended Settings

```toml
[security]
use_tpm = true
attestation_required = true
tls_min_version = "1.3"
require_client_cert = true
audit_logging = true

[encryption]
algorithm = "AES-256-GCM"
key_rotation_hours = 24
secure_key_storage = true
```

### Security Headers

The gateway automatically includes security headers:

- `Strict-Transport-Security`
- `X-Content-Type-Options`
- `X-Frame-Options`
- `X-XSS-Protection`
- `Content-Security-Policy`

## Compliance

This project aims to comply with:

- **NIST Cybersecurity Framework**
- **ISO 27001** security standards
- **GDPR** for data protection
- **FIPS 140-2** for cryptographic modules
- **Common Criteria** for secure software

## Security Tools

We use the following tools for security:

- **Static Analysis**: Clippy, cargo-audit, CodeQL
- **Dependency Scanning**: cargo-deny, Dependabot
- **Container Scanning**: Trivy, Snyk
- **Secrets Scanning**: TruffleHog, detect-secrets
- **Fuzzing**: cargo-fuzz, honggfuzz
- **SAST/DAST**: Various commercial and open-source tools

## Security Training

All contributors receive security training covering:

- Secure coding practices
- Threat modeling
- Vulnerability assessment
- Incident response
- Privacy protection

## Bug Bounty Program

We are considering establishing a bug bounty program. Stay tuned for updates.

## Contact

For general security questions or concerns:

- **Email**: [security@terragon.ai](mailto:security@terragon.ai)
- **Documentation**: [Security Guide](https://docs.terragon.ai/mcp-edge/security)
- **Community**: [Security Discussions](https://github.com/terragon-labs/mcp-wasm-edge-gateway/discussions)

## Acknowledgments

We thank the security researchers who have responsibly disclosed vulnerabilities to us.

## Legal

This security policy is subject to our [Terms of Service](https://terragon.ai/terms) and [Privacy Policy](https://terragon.ai/privacy).