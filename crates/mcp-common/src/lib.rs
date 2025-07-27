//! Common types and utilities for MCP WASM Edge Gateway
//!
//! This crate provides shared types, traits, and utilities used across
//! all components of the MCP Edge Gateway system.

pub mod error;
pub mod types;
pub mod config;
pub mod metrics;
pub mod utils;

pub use error::{Result, Error};
pub use types::*;
pub use config::Config;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;