//! Common types and utilities for MCP WASM Edge Gateway
//!
//! This crate provides shared types, traits, and utilities used across
//! all components of the MCP Edge Gateway system.

pub mod circuit_breaker;
pub mod config;
pub mod error;
pub mod metrics;
pub mod retry;
pub mod types;
pub mod utils;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState, with_circuit_breaker};
pub use config::Config;
pub use error::{Error, Result};
pub use retry::{RetryStrategy, RetryExecutor, retry_operation, retry_for_error};
pub use types::*;
pub use metrics::{HealthLevel, ComponentHealth, HealthStatus};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
