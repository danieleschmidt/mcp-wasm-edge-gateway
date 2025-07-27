# ADR-001: Rust + WASM for Edge AI Gateway

## Status
Accepted

## Date
2025-01-27

## Context
We need to choose a technology stack for building an ultra-lightweight AI gateway that can run on edge devices including Raspberry Pi, ESP32, mobile platforms, and browsers. The solution must be:

- Extremely small binary size (<3MB)
- High performance on resource-constrained devices
- Cross-platform compatibility
- Memory safe and secure
- Suitable for embedded systems

## Decision
We will use **Rust** as the primary programming language and compile to **WebAssembly (WASM)** for deployment across platforms.

## Rationale

### Why Rust?
1. **Memory Safety**: Zero-cost abstractions with compile-time memory safety guarantees
2. **Performance**: Comparable to C/C++ with better safety guarantees
3. **Cross-compilation**: Excellent support for embedded and edge targets
4. **No Runtime**: No garbage collector or runtime overhead
5. **Ecosystem**: Rich crate ecosystem with embedded/no-std support
6. **Concurrency**: Safe, efficient async/await and threading primitives

### Why WASM?
1. **Size Optimization**: Advanced dead-code elimination and compression
2. **Sandboxing**: Built-in security isolation for untrusted environments
3. **Portability**: Single binary runs across platforms (browsers, servers, edge)
4. **Performance**: Near-native execution speed with predictable performance
5. **Ecosystem**: Growing support in embedded runtimes (WasmEdge, Wasmtime)

### Technology Stack Details
- **Core Language**: Rust 1.75+ with edition 2021
- **WASM Target**: wasm32-wasi for WASI compatibility
- **Async Runtime**: tokio (native) / wasm-bindgen-futures (WASM)
- **Serialization**: MessagePack for efficient binary protocols
- **Crypto**: RustCrypto crates with hardware acceleration
- **ML Runtime**: candle-core for tensor operations

## Alternatives Considered

### C/C++
**Pros**: Maximum performance, wide platform support
**Cons**: Memory safety issues, complex build system, security vulnerabilities

### Go
**Pros**: Simple deployment, good concurrency, fast compilation
**Cons**: Garbage collector overhead, larger binary size, limited embedded support

### JavaScript/Node.js
**Pros**: Rapid development, large ecosystem, JSON-native
**Cons**: V8 runtime overhead, inconsistent performance, security concerns

### Python
**Pros**: Rich ML ecosystem, rapid prototyping
**Cons**: Interpreter overhead, packaging complexity, poor embedded support

## Implementation Strategy

### Phase 1: Core Rust Implementation
```rust
// Core gateway structure
pub struct Gateway {
    config: Arc<Config>,
    router: Arc<dyn RequestRouter>,
    model_engine: Arc<ModelEngine>,
    security: Arc<SecurityManager>,
}

// WASM-compatible async runtime
#[cfg(target_arch = "wasm32")]
type Runtime = wasm_bindgen_futures::spawn_local;

#[cfg(not(target_arch = "wasm32"))]
type Runtime = tokio::runtime::Runtime;
```

### Phase 2: Platform Adaptations
```rust
// Platform-specific optimizations
#[cfg(target_arch = "wasm32")]
impl PlatformAdapter for WASMAdapter {
    fn optimize_for_platform(&self, config: &mut Config) {
        config.enable_simd = true;
        config.threading_model = ThreadingModel::SingleThreaded;
    }
}

#[cfg(target_os = "espidf")]
impl PlatformAdapter for ESP32Adapter {
    fn optimize_for_platform(&self, config: &mut Config) {
        config.max_memory_mb = 64;
        config.power_profile = PowerProfile::UltraLowPower;
    }
}
```

### Phase 3: Build Optimization
```toml
# Cargo.toml optimizations
[profile.release]
opt-level = 'z'  # Optimize for size
lto = true       # Link-time optimization
codegen-units = 1
panic = 'abort'

[profile.release.package."*"]
opt-level = 'z'

# WASM-specific optimizations
[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
wee_alloc = "0.4"  # Tiny allocator
```

## Consequences

### Positive
- **Binary Size**: Expected 2-3MB WASM binary vs 50MB+ for alternatives
- **Performance**: Near-native performance with memory safety
- **Security**: Compile-time safety + WASM sandboxing
- **Portability**: Single codebase for all target platforms
- **Future-proof**: Growing WASM ecosystem and tooling

### Negative
- **Learning Curve**: Rust ownership model complexity
- **Compile Times**: Slower compilation vs interpreted languages
- **Debugging**: Limited debugging tools for WASM targets
- **Ecosystem**: Some crates not no-std compatible

### Risks & Mitigations
- **Risk**: WASM performance limitations for ML workloads
  **Mitigation**: Benchmark early, use SIMD optimizations, profile extensively
- **Risk**: Platform-specific debugging challenges
  **Mitigation**: Comprehensive testing on target hardware, logging framework
- **Risk**: Dependency compatibility issues
  **Mitigation**: Vendor critical dependencies, maintain compatibility matrix

## Success Criteria
- [ ] WASM binary size < 3MB
- [ ] Rust compilation to all target platforms
- [ ] Performance within 20% of native C implementation
- [ ] Memory usage < 512MB on Raspberry Pi
- [ ] Successful deployment on 5+ platform targets

## Follow-up Actions
- Create Rust project structure with workspace organization
- Set up cross-compilation toolchain for all targets
- Implement basic WASM build pipeline with size optimization
- Create platform-specific adapter framework
- Establish performance benchmarking suite