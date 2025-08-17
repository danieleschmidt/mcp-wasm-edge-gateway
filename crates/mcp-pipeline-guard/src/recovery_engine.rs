//! Recovery engine for automatic component recovery

use crate::PipelineAware;
use mcp_common::{Error, Result};
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error, debug};
use std::collections::HashMap;
use parking_lot::Mutex;

/// Recovery strategy for different types of failures
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Simple restart attempt
    Restart,
    /// Graceful restart with state preservation
    GracefulRestart,
    /// Circuit breaker pattern
    CircuitBreaker { timeout_seconds: u64 },
    /// Fallback to backup component
    Fallback { backup_component_id: String },
    /// Custom recovery procedure
    Custom { procedure_name: String },
}

/// Recovery attempt result
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub success: bool,
    pub strategy_used: RecoveryStrategy,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

/// Component recovery engine
pub struct RecoveryEngine {
    recovery_strategies: Arc<Mutex<HashMap<String, Vec<RecoveryStrategy>>>>,
    recovery_history: Arc<Mutex<HashMap<String, Vec<RecoveryResult>>>>,
}

impl RecoveryEngine {
    /// Create a new recovery engine
    pub fn new() -> Self {
        RecoveryEngine {
            recovery_strategies: Arc::new(Mutex::new(HashMap::new())),
            recovery_history: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register recovery strategies for a component
    pub fn register_strategies(&self, component_id: &str, strategies: Vec<RecoveryStrategy>) {
        debug!("Registering recovery strategies for component: {}", component_id);
        self.recovery_strategies
            .lock()
            .insert(component_id.to_string(), strategies);
    }

    /// Attempt to recover a component
    pub async fn recover_component(&self, component: Arc<dyn PipelineAware + Send + Sync>) -> Result<()> {
        let component_id = component.component_id();
        info!("Attempting recovery for component: {}", component_id);

        let strategies = {
            let lock = self.recovery_strategies.lock();
            lock.get(component_id)
                .cloned()
                .unwrap_or_else(|| vec![RecoveryStrategy::Restart])
        };

        for strategy in strategies {
            let start_time = std::time::Instant::now();
            
            debug!("Trying recovery strategy: {:?} for component: {}", strategy, component_id);
            
            let result = match self.execute_recovery_strategy(&strategy, component.clone()).await {
                Ok(_) => {
                    let duration = start_time.elapsed().as_millis() as u64;
                    info!("Recovery successful for component {} using strategy {:?} (took {}ms)", 
                          component_id, strategy, duration);
                    
                    RecoveryResult {
                        success: true,
                        strategy_used: strategy,
                        duration_ms: duration,
                        error_message: None,
                    }
                },
                Err(e) => {
                    let duration = start_time.elapsed().as_millis() as u64;
                    warn!("Recovery failed for component {} using strategy {:?}: {} (took {}ms)", 
                          component_id, strategy, e, duration);
                    
                    RecoveryResult {
                        success: false,
                        strategy_used: strategy,
                        duration_ms: duration,
                        error_message: Some(e.to_string()),
                    }
                }
            };

            // Record recovery attempt
            {
                let mut history = self.recovery_history.lock();
                history.entry(component_id.to_string())
                    .or_insert_with(Vec::new)
                    .push(result);
            }

            // If recovery was successful, return early
            if self.recovery_history.lock()
                .get(component_id)
                .and_then(|h| h.last())
                .map(|r| r.success)
                .unwrap_or(false) 
            {
                return Ok(());
            }
        }

        Err(Error::Internal(format!(
            "All recovery strategies failed for component: {}", 
            component_id
        )))
    }

    /// Execute a specific recovery strategy
    async fn execute_recovery_strategy(
        &self,
        strategy: &RecoveryStrategy,
        component: Arc<dyn PipelineAware + Send + Sync>,
    ) -> Result<()> {
        match strategy {
            RecoveryStrategy::Restart => {
                self.simple_restart(component).await
            },
            RecoveryStrategy::GracefulRestart => {
                self.graceful_restart(component).await
            },
            RecoveryStrategy::CircuitBreaker { timeout_seconds } => {
                self.circuit_breaker_recovery(component, *timeout_seconds).await
            },
            RecoveryStrategy::Fallback { backup_component_id } => {
                self.fallback_recovery(component, backup_component_id).await
            },
            RecoveryStrategy::Custom { procedure_name } => {
                self.custom_recovery(component, procedure_name).await
            },
        }
    }

    /// Simple restart recovery
    async fn simple_restart(&self, component: Arc<dyn PipelineAware + Send + Sync>) -> Result<()> {
        debug!("Executing simple restart for component: {}", component.component_id());
        
        // Give the component a chance to recover itself
        timeout(Duration::from_secs(30), component.recover()).await
            .map_err(|_| Error::Internal("Recovery timeout".to_string()))?
    }

    /// Graceful restart with state preservation
    async fn graceful_restart(&self, component: Arc<dyn PipelineAware + Send + Sync>) -> Result<()> {
        debug!("Executing graceful restart for component: {}", component.component_id());
        
        // For now, this is the same as simple restart
        // In a full implementation, this would preserve state
        self.simple_restart(component).await
    }

    /// Circuit breaker recovery
    async fn circuit_breaker_recovery(
        &self,
        component: Arc<dyn PipelineAware + Send + Sync>,
        timeout_seconds: u64,
    ) -> Result<()> {
        debug!("Executing circuit breaker recovery for component: {} (timeout: {}s)", 
               component.component_id(), timeout_seconds);
        
        timeout(Duration::from_secs(timeout_seconds), component.recover()).await
            .map_err(|_| Error::Internal(format!("Circuit breaker recovery timeout after {}s", timeout_seconds)))?
    }

    /// Fallback recovery
    async fn fallback_recovery(
        &self,
        component: Arc<dyn PipelineAware + Send + Sync>,
        _backup_component_id: &str,
    ) -> Result<()> {
        debug!("Executing fallback recovery for component: {}", component.component_id());
        
        // For now, attempt simple recovery
        // In a full implementation, this would switch to a backup component
        self.simple_restart(component).await
    }

    /// Custom recovery procedure
    async fn custom_recovery(
        &self,
        component: Arc<dyn PipelineAware + Send + Sync>,
        procedure_name: &str,
    ) -> Result<()> {
        debug!("Executing custom recovery '{}' for component: {}", 
               procedure_name, component.component_id());
        
        // For now, attempt simple recovery
        // In a full implementation, this would execute named recovery procedures
        self.simple_restart(component).await
    }

    /// Get recovery history for a component
    pub fn get_recovery_history(&self, component_id: &str) -> Vec<RecoveryResult> {
        self.recovery_history
            .lock()
            .get(component_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get recovery statistics
    pub fn get_recovery_stats(&self) -> HashMap<String, (u32, u32)> {
        let mut stats = HashMap::new();
        let history = self.recovery_history.lock();
        
        for (component_id, results) in history.iter() {
            let total = results.len() as u32;
            let successful = results.iter().filter(|r| r.success).count() as u32;
            stats.insert(component_id.clone(), (successful, total));
        }
        
        stats
    }

    /// Clear recovery history for a component
    pub fn clear_history(&self, component_id: &str) {
        self.recovery_history.lock().remove(component_id);
    }
}