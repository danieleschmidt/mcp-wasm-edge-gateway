//! Pipeline state management and tracking

use mcp_common::HealthLevel;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Status of a pipeline component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentStatus {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}

impl From<bool> for ComponentStatus {
    fn from(healthy: bool) -> Self {
        if healthy {
            ComponentStatus::Healthy
        } else {
            ComponentStatus::Failed
        }
    }
}

/// Overall pipeline status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineStatus {
    Healthy,    // All components healthy
    Degraded,   // Some components degraded but operational
    Critical,   // Critical components failed
    Failed,     // Multiple components failed
}

/// Component information and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub id: String,
    pub status: ComponentStatus,
    pub last_health_check: DateTime<Utc>,
    pub last_status_change: DateTime<Utc>,
    pub consecutive_failures: u32,
    pub total_failures: u64,
    pub metrics: HashMap<String, f64>,
}

impl ComponentInfo {
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        ComponentInfo {
            id,
            status: ComponentStatus::Unknown,
            last_health_check: now,
            last_status_change: now,
            consecutive_failures: 0,
            total_failures: 0,
            metrics: HashMap::new(),
        }
    }

    pub fn update_health(&mut self, healthy: bool) {
        let now = Utc::now();
        let new_status = ComponentStatus::from(healthy);
        
        if new_status != self.status {
            self.last_status_change = now;
            
            if !healthy {
                self.consecutive_failures += 1;
                self.total_failures += 1;
            } else {
                self.consecutive_failures = 0;
            }
        }
        
        self.status = new_status;
        self.last_health_check = now;
    }

    pub fn update_metrics(&mut self, metrics: HashMap<String, f64>) {
        self.metrics = metrics;
    }
}

/// Pipeline state tracking
#[derive(Debug, Clone)]
pub struct PipelineState {
    pub started_at: DateTime<Utc>,
    pub components: HashMap<String, ComponentInfo>,
    pub last_status_update: DateTime<Utc>,
}

impl PipelineState {
    /// Create a new pipeline state
    pub fn new() -> Self {
        PipelineState {
            started_at: Utc::now(),
            components: HashMap::new(),
            last_status_update: Utc::now(),
        }
    }

    /// Add a component to tracking
    pub fn add_component(&mut self, component_id: String) {
        let component_info = ComponentInfo::new(component_id.clone());
        self.components.insert(component_id, component_info);
        self.last_status_update = Utc::now();
    }

    /// Remove a component from tracking
    pub fn remove_component(&mut self, component_id: &str) {
        self.components.remove(component_id);
        self.last_status_update = Utc::now();
    }

    /// Update component health status
    pub fn update_component_health(&mut self, component_id: &str, healthy: bool) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.update_health(healthy);
            self.last_status_update = Utc::now();
        }
    }

    /// Update component metrics
    pub fn update_component_metrics(&mut self, component_id: &str, metrics: HashMap<String, f64>) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.update_metrics(metrics);
        }
    }

    /// Get overall pipeline health
    pub fn get_overall_health(&self) -> HealthLevel {
        if self.components.is_empty() {
            return HealthLevel::Healthy;
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut failed_count = 0;

        for component in self.components.values() {
            match component.status {
                ComponentStatus::Healthy => healthy_count += 1,
                ComponentStatus::Degraded => degraded_count += 1,
                ComponentStatus::Failed => failed_count += 1,
                ComponentStatus::Unknown => degraded_count += 1,
            }
        }

        let total = self.components.len();
        let failure_rate = failed_count as f64 / total as f64;
        let degraded_rate = (degraded_count + failed_count) as f64 / total as f64;

        if failure_rate > 0.5 {
            HealthLevel::Critical
        } else if degraded_rate > 0.3 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        }
    }

    /// Get pipeline status
    pub fn get_pipeline_status(&self) -> PipelineStatus {
        match self.get_overall_health() {
            HealthLevel::Healthy => PipelineStatus::Healthy,
            HealthLevel::Degraded => PipelineStatus::Degraded,
            HealthLevel::Critical => PipelineStatus::Critical,
        }
    }

    /// Get component count
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Get healthy component count
    pub fn healthy_component_count(&self) -> usize {
        self.components
            .values()
            .filter(|c| c.status == ComponentStatus::Healthy)
            .count()
    }

    /// Get failed component count
    pub fn failed_component_count(&self) -> usize {
        self.components
            .values()
            .filter(|c| c.status == ComponentStatus::Failed)
            .count()
    }

    /// Get degraded component count
    pub fn degraded_component_count(&self) -> usize {
        self.components
            .values()
            .filter(|c| c.status == ComponentStatus::Degraded)
            .count()
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        Utc::now()
            .signed_duration_since(self.started_at)
            .num_seconds() as u64
    }

    /// Get component by ID
    pub fn get_component(&self, component_id: &str) -> Option<&ComponentInfo> {
        self.components.get(component_id)
    }

    /// Get all components
    pub fn get_all_components(&self) -> &HashMap<String, ComponentInfo> {
        &self.components
    }

    /// Get components by status
    pub fn get_components_by_status(&self, status: ComponentStatus) -> Vec<&ComponentInfo> {
        self.components
            .values()
            .filter(|c| c.status == status)
            .collect()
    }

    /// Get components with consecutive failures above threshold
    pub fn get_problematic_components(&self, failure_threshold: u32) -> Vec<&ComponentInfo> {
        self.components
            .values()
            .filter(|c| c.consecutive_failures >= failure_threshold)
            .collect()
    }

    /// Reset component failure counters
    pub fn reset_component_failures(&mut self, component_id: &str) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.consecutive_failures = 0;
            component.total_failures = 0;
        }
    }

    /// Get pipeline summary
    pub fn get_summary(&self) -> PipelineSummary {
        PipelineSummary {
            total_components: self.component_count(),
            healthy_components: self.healthy_component_count(),
            degraded_components: self.degraded_component_count(),
            failed_components: self.failed_component_count(),
            overall_status: self.get_pipeline_status(),
            uptime_seconds: self.uptime_seconds(),
            last_update: self.last_status_update,
        }
    }
}

/// Pipeline summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub total_components: usize,
    pub healthy_components: usize,
    pub degraded_components: usize,
    pub failed_components: usize,
    pub overall_status: PipelineStatus,
    pub uptime_seconds: u64,
    pub last_update: DateTime<Utc>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::new()
    }
}