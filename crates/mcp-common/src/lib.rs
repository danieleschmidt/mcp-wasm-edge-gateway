//! Common types and utilities for MCP WASM Edge Gateway
//!
//! This crate provides shared types, traits, and utilities used across
//! all components of the MCP Edge Gateway system.

pub mod config;
pub mod error;
pub mod metrics;
pub mod types;
pub mod utils;

pub use config::Config;
pub use error::{Error, Result};
pub use types::*;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
