//! Circuit breaker implementation for fault tolerance and resilience
//!
//! Implements the circuit breaker pattern to prevent cascading failures
//! and provide fast failure responses when downstream services are unhealthy.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing fast
    HalfOpen,  // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit from half-open
    pub success_threshold: u32,
    /// Timeout before trying half-open state
    pub timeout: Duration,
    /// Window size for tracking failures
    pub window_size: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            window_size: Duration::from_secs(300),
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            name: name.into(),
        }
    }

    /// Check if the circuit breaker allows the request
    pub async fn can_execute(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = Instant::now().elapsed().as_secs();
                if now - last_failure >= self.config.timeout.as_secs() {
                    drop(state);
                    self.transition_to_half_open().await;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful execution
    pub async fn record_success(&self) {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.config.success_threshold {
                    drop(state);
                    self.transition_to_closed().await;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset failure count
                self.failure_count.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Record a failed execution
    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure_time.store(
            Instant::now().elapsed().as_secs(),
            Ordering::Relaxed,
        );

        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to_open().await;
                }
            }
            CircuitState::HalfOpen => {
                drop(state);
                self.transition_to_open().await;
            }
            CircuitState::Open => {
                // Already open, just record the failure
            }
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Get metrics
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            name: self.name.clone(),
            state: self.state().await,
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
        }
    }

    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        warn!(
            "Circuit breaker '{}' opened due to {} failures",
            self.name,
            self.failure_count.load(Ordering::Relaxed)
        );
    }

    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;
        self.success_count.store(0, Ordering::Relaxed);
        info!(
            "Circuit breaker '{}' transitioned to half-open for testing",
            self.name
        );
    }

    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        info!(
            "Circuit breaker '{}' closed - service recovered",
            self.name
        );
    }
}

/// Circuit breaker metrics for monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
}

/// Error type for circuit breaker failures
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker '{0}' is open - requests are being rejected")]
    CircuitOpen(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Execute a function with circuit breaker protection
pub async fn execute_with_circuit_breaker<F, T, E>(
    circuit_breaker: &CircuitBreaker,
    operation: F,
) -> Result<T, CircuitBreakerError>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    if !circuit_breaker.can_execute().await {
        return Err(CircuitBreakerError::CircuitOpen(circuit_breaker.name.clone()));
    }

    match operation.await {
        Ok(result) => {
            circuit_breaker.record_success().await;
            Ok(result)
        }
        Err(e) => {
            circuit_breaker.record_failure().await;
            Err(CircuitBreakerError::ExecutionFailed(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            window_size: Duration::from_secs(60),
        };
        
        let cb = CircuitBreaker::new("test", config);
        
        // Initially closed
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.can_execute().await);
        
        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
        
        cb.record_failure().await; // Should open
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.can_execute().await);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            window_size: Duration::from_secs(60),
        };
        
        let cb = CircuitBreaker::new("test", config);
        
        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(60)).await;
        
        // Should allow execution and transition to half-open
        assert!(cb.can_execute().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }
}