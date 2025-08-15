//! Self-healing pipeline guard for MCP Edge Gateway
//!
//! This crate provides automatic failure detection, health monitoring,
//! and self-healing capabilities for critical pipeline operations.

pub mod guard;
pub mod health_monitor;
pub mod recovery_engine;
pub mod pipeline_state;
pub mod alerts;

pub use guard::{PipelineGuard, GuardConfig};
pub use health_monitor::{HealthMonitor, HealthThresholds};
pub use recovery_engine::{RecoveryEngine, RecoveryStrategy};
pub use pipeline_state::{PipelineState, PipelineStatus, ComponentStatus};
pub use alerts::{AlertManager, AlertSeverity, AlertChannel};

use mcp_common::{Error, Result};

/// Create a new pipeline guard with default configuration
pub async fn create_pipeline_guard(config: mcp_common::Config) -> Result<PipelineGuard> {
    let guard_config = GuardConfig::from_mcp_config(&config)?;
    PipelineGuard::new(guard_config).await
}

/// Trait for pipeline-aware components
#[async_trait::async_trait]
pub trait PipelineAware {
    /// Check if component is healthy and operational
    async fn is_healthy(&self) -> bool;
    
    /// Get component-specific metrics
    async fn get_metrics(&self) -> std::collections::HashMap<String, f64>;
    
    /// Attempt to recover from failure state
    async fn recover(&self) -> Result<()>;
    
    /// Get component identifier
    fn component_id(&self) -> &str;
}