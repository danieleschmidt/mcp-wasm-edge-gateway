//! Core pipeline guard implementation

use crate::{HealthMonitor, RecoveryEngine, PipelineState, AlertManager, HealthThresholds, PipelineAware};
use mcp_common::{Error, Result, ComponentHealth, HealthLevel};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use tracing::{info, warn, error, debug};
use tokio::sync::Mutex;
use std::collections::HashMap;

/// Main pipeline guard configuration
#[derive(Clone, Debug)]
pub struct GuardConfig {
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Recovery attempt timeout in seconds
    pub recovery_timeout_seconds: u64,
    /// Maximum consecutive failures before circuit breaker trips
    pub max_consecutive_failures: u32,
    /// Circuit breaker reset timeout in seconds
    pub circuit_breaker_reset_seconds: u64,
    /// Enable automatic recovery
    pub auto_recovery_enabled: bool,
    /// Alert thresholds
    pub health_thresholds: HealthThresholds,
    /// Enable performance monitoring
    pub performance_monitoring: bool,
}

impl GuardConfig {
    /// Create guard config from MCP config
    pub fn from_mcp_config(config: &mcp_common::Config) -> Result<Self> {
        Ok(GuardConfig {
            health_check_interval_seconds: 30,
            recovery_timeout_seconds: 60,
            max_consecutive_failures: 3,
            circuit_breaker_reset_seconds: 300,
            auto_recovery_enabled: true,
            health_thresholds: HealthThresholds::default(),
            performance_monitoring: true,
        })
    }
}

/// Self-healing pipeline guard
pub struct PipelineGuard {
    config: GuardConfig,
    health_monitor: Arc<Mutex<HealthMonitor>>,
    recovery_engine: Arc<RecoveryEngine>,
    alert_manager: Arc<AlertManager>,
    pipeline_state: Arc<RwLock<PipelineState>>,
    registered_components: Arc<Mutex<HashMap<String, Arc<dyn PipelineAware + Send + Sync>>>>,
    _monitoring_handle: tokio::task::JoinHandle<()>,
}

impl PipelineGuard {
    /// Create a new pipeline guard
    pub async fn new(config: GuardConfig) -> Result<Self> {
        info!("Initializing pipeline guard");

        let health_monitor = Arc::new(Mutex::new(HealthMonitor::new(config.health_thresholds.clone())));
        let recovery_engine = Arc::new(RecoveryEngine::new());
        let alert_manager = Arc::new(AlertManager::new());
        let pipeline_state = Arc::new(RwLock::new(PipelineState::new()));
        let registered_components = Arc::new(Mutex::new(HashMap::new()));

        // Start background monitoring
        let monitoring_handle = {
            let health_monitor = health_monitor.clone();
            let recovery_engine = recovery_engine.clone();
            let alert_manager = alert_manager.clone();
            let pipeline_state = pipeline_state.clone();
            let registered_components = registered_components.clone();
            let interval_duration = Duration::from_secs(config.health_check_interval_seconds);
            let auto_recovery = config.auto_recovery_enabled;

            tokio::spawn(async move {
                let mut interval_timer = interval(interval_duration);
                
                loop {
                    interval_timer.tick().await;
                    
                    if let Err(e) = Self::monitoring_cycle(
                        &health_monitor,
                        &recovery_engine,
                        &alert_manager,
                        &pipeline_state,
                        &registered_components,
                        auto_recovery,
                    ).await {
                        error!("Error in monitoring cycle: {}", e);
                    }
                }
            })
        };

        info!("Pipeline guard initialized successfully");

        Ok(PipelineGuard {
            config,
            health_monitor,
            recovery_engine,
            alert_manager,
            pipeline_state,
            registered_components,
            _monitoring_handle: monitoring_handle,
        })
    }

    /// Register a component for monitoring
    pub async fn register_component(&self, component: Arc<dyn PipelineAware + Send + Sync>) -> Result<()> {
        let component_id = component.component_id().to_string();
        
        debug!("Registering component: {}", component_id);
        
        self.registered_components
            .lock()
            .await
            .insert(component_id.clone(), component);

        // Initialize component state
        let mut state = self.pipeline_state.write().await;
        state.add_component(component_id.clone());

        info!("Component registered: {}", component_id);
        Ok(())
    }

    /// Get current pipeline health status
    pub async fn get_health_status(&self) -> Result<ComponentHealth> {
        let state = self.pipeline_state.read().await;
        let overall_health = state.get_overall_health();

        Ok(ComponentHealth {
            status: overall_health,
            message: match overall_health {
                HealthLevel::Healthy => "All pipeline components healthy".to_string(),
                HealthLevel::Degraded => "Some pipeline components degraded".to_string(),
                HealthLevel::Critical => "Critical pipeline failures detected".to_string(),
                HealthLevel::Unknown => "Pipeline health status unknown".to_string(),
            },
            last_check: chrono::Utc::now(),
            metrics: self.get_pipeline_metrics().await,
        })
    }

    /// Get pipeline metrics
    pub async fn get_pipeline_metrics(&self) -> HashMap<String, f32> {
        let mut metrics = HashMap::new();
        
        let state = self.pipeline_state.read().await;
        metrics.insert("total_components".to_string(), state.component_count() as f32);
        metrics.insert("healthy_components".to_string(), state.healthy_component_count() as f32);
        metrics.insert("failed_components".to_string(), state.failed_component_count() as f32);
        metrics.insert("uptime_seconds".to_string(), state.uptime_seconds() as f32);

        // Add component-specific metrics
        let components = self.registered_components.lock().await;
        for (component_id, component) in components.iter() {
            let component_metrics = component.get_metrics().await;
            for (key, value) in component_metrics {
                metrics.insert(format!("{}_{}", component_id, key), value as f32);
            }
        }

        metrics
    }

    /// Force a health check cycle
    pub async fn force_health_check(&self) -> Result<()> {
        info!("Forcing health check cycle");
        
        Self::monitoring_cycle(
            &self.health_monitor,
            &self.recovery_engine,
            &self.alert_manager,
            &self.pipeline_state,
            &self.registered_components,
            self.config.auto_recovery_enabled,
        ).await
    }

    /// Manually trigger recovery for a specific component
    pub async fn recover_component(&self, component_id: &str) -> Result<()> {
        info!("Manually triggering recovery for component: {}", component_id);

        let components = self.registered_components.lock().await;
        if let Some(component) = components.get(component_id) {
            self.recovery_engine.recover_component(component.clone()).await
        } else {
            Err(Error::Internal(format!("Component not found: {}", component_id)))
        }
    }

    /// Internal monitoring cycle
    async fn monitoring_cycle(
        health_monitor: &Arc<Mutex<HealthMonitor>>,
        recovery_engine: &RecoveryEngine,
        alert_manager: &AlertManager,
        pipeline_state: &Arc<RwLock<PipelineState>>,
        registered_components: &Arc<Mutex<HashMap<String, Arc<dyn PipelineAware + Send + Sync>>>>,
        auto_recovery: bool,
    ) -> Result<()> {
        debug!("Starting monitoring cycle");

        let components = {
            let lock = registered_components.lock().await;
            lock.clone()
        };

        for (component_id, component) in components {
            // Check component health
            let is_healthy = component.is_healthy().await;
            let metrics = component.get_metrics().await;

            // Update pipeline state
            {
                let mut state = pipeline_state.write().await;
                state.update_component_health(&component_id, is_healthy);
                state.update_component_metrics(&component_id, metrics.clone());
            }

            // Analyze health and trigger alerts/recovery if needed
            let health_assessment = {
                let mut monitor = health_monitor.lock().await;
                monitor.assess_component_health(&component_id, is_healthy, &metrics).await
            };
            
            if !health_assessment.is_healthy {
                warn!("Component {} is unhealthy: {}", component_id, health_assessment.reason);
                
                // Send alert
                alert_manager.send_component_alert(&component_id, &health_assessment.reason).await?;

                // Trigger recovery if enabled
                if auto_recovery {
                    info!("Triggering automatic recovery for component: {}", component_id);
                    if let Err(e) = recovery_engine.recover_component(component.clone()).await {
                        error!("Recovery failed for component {}: {}", component_id, e);
                        alert_manager.send_recovery_failed_alert(&component_id, &e.to_string()).await?;
                    } else {
                        info!("Recovery completed for component: {}", component_id);
                        alert_manager.send_recovery_success_alert(&component_id).await?;
                    }
                }
            }
        }

        debug!("Monitoring cycle completed");
        Ok(())
    }

    /// Get pipeline configuration
    pub fn config(&self) -> &GuardConfig {
        &self.config
    }

    /// Shutdown the pipeline guard
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down pipeline guard");
        
        // The monitoring handle will be dropped and cancelled automatically
        
        info!("Pipeline guard shutdown complete");
        Ok(())
    }
}

impl Drop for PipelineGuard {
    fn drop(&mut self) {
        // Cancel the monitoring task
        self._monitoring_handle.abort();
    }
}