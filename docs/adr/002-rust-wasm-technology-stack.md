# ADR-002: Rust and WASM Technology Stack

## Status
Accepted

## Context
We need to select the core technology stack for the MCP WASM Edge Gateway. The solution must be:
- Extremely lightweight (<3MB binary)
- Cross-platform compatible
- Secure and memory-safe
- High-performance for edge devices
- Suitable for embedded and resource-constrained environments

## Decision
We will use Rust as the primary programming language with WebAssembly (WASM) as the target compilation format for maximum portability.

## Rationale

### Rust Language Choice
- **Memory Safety**: Zero-cost abstractions with compile-time memory safety
- **Performance**: Near C/C++ performance with modern optimizations
- **Concurrency**: Safe concurrency primitives and async/await support
- **Cross-compilation**: Excellent support for multiple target architectures
- **Ecosystem**: Rich crate ecosystem for networking, crypto, and ML
- **WASM Support**: First-class WebAssembly compilation support

### WASM Compilation Target
- **Portability**: Run anywhere with WASM runtime (browsers, servers, edge)
- **Security**: Sandboxed execution environment
- **Size**: Compact binary format suitable for edge deployment
- **Performance**: Near-native performance with SIMD support
- **Interoperability**: Easy integration with existing systems

### Alternative Considerations
1. **C/C++**: Rejected due to memory safety concerns and development complexity
2. **Go**: Rejected due to larger binary size and garbage collector overhead
3. **Python**: Rejected due to performance and deployment complexity
4. **JavaScript/Node.js**: Rejected due to security and resource constraints
5. **Zig**: Considered but ecosystem too immature

## Consequences

### Positive
- Memory-safe code reduces security vulnerabilities
- Excellent performance characteristics for edge devices
- Strong type system prevents many runtime errors
- Cross-platform compatibility through WASM
- Active community and growing ecosystem
- Modern development tools and practices

### Negative
- Learning curve for developers unfamiliar with Rust
- Compile times can be slower than interpreted languages
- WASM limitations for certain system-level operations
- Smaller talent pool compared to mainstream languages
- Some crates may not be compatible with no_std environments

## Implementation

### Core Dependencies
```toml
[dependencies]
tokio = { version = "1.0", features = ["rt", "net", "time"] }
serde = { version = "1.0", features = ["derive"] }
reqwest = { version = "0.11", features = ["json"] }
tonic = "0.10"  # gRPC support
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }
```

### WASM-Specific Dependencies
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = "0.3"
```

### Build Configuration
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
opt-level = "s"  # Optimize for size

[profile.release.package."*"]
opt-level = "s"
```

### Target Architectures
- `wasm32-unknown-unknown` - Browser and WASM runtimes
- `wasm32-wasi` - WASI-compatible environments
- `aarch64-unknown-linux-gnu` - ARM64 (Raspberry Pi, Jetson)
- `x86_64-unknown-linux-gnu` - x86_64 Linux
- `x86_64-pc-windows-gnu` - Windows
- `x86_64-apple-darwin` - macOS

### Development Tools
- **rustfmt**: Code formatting
- **clippy**: Linting and suggestions
- **cargo-audit**: Security vulnerability scanning
- **cargo-deny**: License and dependency management
- **wasm-pack**: WASM packaging and optimization

## Platform-Specific Adaptations

### Embedded Targets (ESP32, etc.)
```rust
#![no_std]
#![no_main]

use esp32_hal as hal;
use mcp_gateway_core::Gateway;

#[entry]
fn main() -> ! {
    let peripherals = hal::Peripherals::take().unwrap();
    let config = GatewayConfig::embedded_default();
    let gateway = Gateway::new(config);
    
    gateway.run_embedded(peripherals)
}
```

### WASM Browser Target
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebGateway {
    inner: Gateway,
}

#[wasm_bindgen]
impl WebGateway {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let config = GatewayConfig::web_default();
        Self {
            inner: Gateway::new(config),
        }
    }
    
    #[wasm_bindgen]
    pub async fn process_request(&self, request: &str) -> String {
        self.inner.process_json_request(request).await
    }
}
```

## Migration Strategy
1. **Phase 1**: Core Rust implementation with native targets
2. **Phase 2**: Add WASM compilation and optimization
3. **Phase 3**: Platform-specific optimizations and adaptations
4. **Phase 4**: Performance tuning and size optimization

## Success Metrics
- Binary size <3MB for WASM target
- Memory usage <512MB on target platforms
- Compilation time <5 minutes for full build
- Test coverage >90% across all targets
- Zero unsafe code blocks in core modules

## Review and Updates
This decision will be reviewed quarterly or when:
- Performance requirements cannot be met
- Security vulnerabilities are discovered
- New platforms require different approaches
- Ecosystem changes significantly

---
*Author: Terry (Terragon Labs)*
*Date: 2025-01-27*
*Reviewers: Architecture Team*