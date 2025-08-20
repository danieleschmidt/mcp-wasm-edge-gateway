//! Health monitoring functionality for pipeline components

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Health thresholds for monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthThresholds {
    /// Maximum acceptable error rate (0.0 - 1.0)
    pub max_error_rate: f64,
    /// Maximum acceptable response time in milliseconds
    pub max_response_time_ms: f64,
    /// Minimum acceptable throughput (requests per second)
    pub min_throughput_rps: f64,
    /// Maximum acceptable memory usage in MB
    pub max_memory_usage_mb: f64,
    /// Maximum acceptable CPU usage (0.0 - 1.0)
    pub max_cpu_usage: f64,
    /// Number of consecutive failures before marking as unhealthy
    pub consecutive_failure_threshold: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        HealthThresholds {
            max_error_rate: 0.1,           // 10% error rate
            max_response_time_ms: 5000.0,  // 5 second response time
            min_throughput_rps: 1.0,       // 1 request per second minimum
            max_memory_usage_mb: 512.0,    // 512MB memory usage
            max_cpu_usage: 0.8,            // 80% CPU usage
            consecutive_failure_threshold: 3,
        }
    }
}

/// Health assessment result
#[derive(Debug, Clone)]
pub struct HealthAssessment {
    pub is_healthy: bool,
    pub reason: String,
    pub severity_score: f64,  // 0.0 (healthy) to 1.0 (critical)
}

/// Component health monitor
pub struct HealthMonitor {
    thresholds: HealthThresholds,
    component_failures: HashMap<String, u32>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(thresholds: HealthThresholds) -> Self {
        HealthMonitor {
            thresholds,
            component_failures: HashMap::new(),
        }
    }

    /// Assess component health based on status and metrics
    pub async fn assess_component_health(
        &mut self,
        component_id: &str,
        is_healthy: bool,
        metrics: &HashMap<String, f64>,
    ) -> HealthAssessment {
        debug!("Assessing health for component: {}", component_id);

        // Check basic health status first
        if !is_healthy {
            let failure_count = self.component_failures
                .entry(component_id.to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            if *failure_count >= self.thresholds.consecutive_failure_threshold {
                warn!("Component {} has {} consecutive failures", component_id, failure_count);
                return HealthAssessment {
                    is_healthy: false,
                    reason: format!("Component failed {} consecutive health checks", failure_count),
                    severity_score: 1.0,
                };
            }
        } else {
            // Reset failure count on successful health check
            self.component_failures.remove(component_id);
        }

        // Check metrics-based health
        let mut issues = Vec::new();
        let mut max_severity: f32 = 0.0;

        // Error rate check
        if let Some(&error_rate) = metrics.get("error_rate") {
            if error_rate > self.thresholds.max_error_rate {
                let severity = (error_rate / self.thresholds.max_error_rate).min(1.0);
                issues.push(format!("High error rate: {:.2}%", error_rate * 100.0));
                max_severity = max_severity.max(severity as f32);
            }
        }

        // Response time check
        if let Some(&response_time) = metrics.get("avg_response_time_ms") {
            if response_time > self.thresholds.max_response_time_ms {
                let severity = (response_time / self.thresholds.max_response_time_ms - 1.0).min(1.0);
                issues.push(format!("High response time: {:.0}ms", response_time));
                max_severity = max_severity.max(severity as f32);
            }
        }

        // Throughput check
        if let Some(&throughput) = metrics.get("throughput_rps") {
            if throughput < self.thresholds.min_throughput_rps {
                let severity = (1.0 - throughput / self.thresholds.min_throughput_rps).min(1.0);
                issues.push(format!("Low throughput: {:.2} rps", throughput));
                max_severity = max_severity.max(severity as f32);
            }
        }

        // Memory usage check
        if let Some(&memory_usage) = metrics.get("memory_usage_mb") {
            if memory_usage > self.thresholds.max_memory_usage_mb {
                let severity = (memory_usage / self.thresholds.max_memory_usage_mb - 1.0).min(1.0);
                issues.push(format!("High memory usage: {:.0}MB", memory_usage));
                max_severity = max_severity.max(severity as f32);
            }
        }

        // CPU usage check
        if let Some(&cpu_usage) = metrics.get("cpu_usage") {
            if cpu_usage > self.thresholds.max_cpu_usage {
                let severity = (cpu_usage / self.thresholds.max_cpu_usage - 1.0).min(1.0);
                issues.push(format!("High CPU usage: {:.1}%", cpu_usage * 100.0));
                max_severity = max_severity.max(severity as f32);
            }
        }

        if issues.is_empty() && is_healthy {
            HealthAssessment {
                is_healthy: true,
                reason: "All metrics within acceptable thresholds".to_string(),
                severity_score: 0.0,
            }
        } else {
            HealthAssessment {
                is_healthy: false,
                reason: if issues.is_empty() {
                    "Component reporting unhealthy status".to_string()
                } else {
                    issues.join("; ")
                },
                severity_score: max_severity as f64,
            }
        }
    }

    /// Update health thresholds
    pub fn update_thresholds(&mut self, thresholds: HealthThresholds) {
        self.thresholds = thresholds;
    }

    /// Get current thresholds
    pub fn thresholds(&self) -> &HealthThresholds {
        &self.thresholds
    }

    /// Get failure counts for all components
    pub fn get_failure_counts(&self) -> &HashMap<String, u32> {
        &self.component_failures
    }

    /// Reset failure count for a component
    pub fn reset_failure_count(&mut self, component_id: &str) {
        self.component_failures.remove(component_id);
    }
}