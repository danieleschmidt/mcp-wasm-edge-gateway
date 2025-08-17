//! Circuit breaker implementation for resilient service calls

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn, info};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests fail fast
    Open,
    /// Circuit is half-open, testing if service is recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit (number of failures)
    pub failure_threshold: u32,
    /// Success threshold to close circuit from half-open (number of successes)
    pub success_threshold: u32,
    /// Timeout before moving from open to half-open
    pub timeout: Duration,
    /// Window size for tracking failures
    pub window_size: u32,
    /// Minimum requests before considering circuit state
    pub minimum_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            window_size: 10,
            minimum_requests: 3,
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

#[derive(Debug)]
struct CircuitBreakerState {
    current_state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    next_attempt: Option<Instant>,
    consecutive_successes: u32,
    recent_calls: Vec<CallResult>,
}

#[derive(Debug, Clone)]
struct CallResult {
    timestamp: Instant,
    success: bool,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            name,
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                current_state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                next_attempt: None,
                consecutive_successes: 0,
                recent_calls: Vec::new(),
            })),
        }
    }

    /// Check if a call should be allowed
    pub async fn should_allow_call(&self) -> bool {
        let mut state = self.state.write().await;
        
        // Clean up old calls outside the window
        let window_start = Instant::now() - Duration::from_secs(300); // 5 minute window
        state.recent_calls.retain(|call| call.timestamp > window_start);

        match state.current_state {
            CircuitState::Closed => {
                // Always allow calls when circuit is closed
                true
            }
            CircuitState::Open => {
                // Check if enough time has passed to try again
                if let Some(next_attempt) = state.next_attempt {
                    if Instant::now() >= next_attempt {
                        info!("Circuit breaker '{}' transitioning to half-open", self.name);
                        state.current_state = CircuitState::HalfOpen;
                        state.consecutive_successes = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited calls to test if service is recovered
                true
            }
        }
    }

    /// Record the result of a call
    pub async fn record_call_result(&self, success: bool) {
        let mut state = self.state.write().await;
        
        // Add to recent calls
        state.recent_calls.push(CallResult {
            timestamp: Instant::now(),
            success,
        });

        // Keep only recent calls within window
        if state.recent_calls.len() > self.config.window_size as usize {
            state.recent_calls.drain(0..1);
        }

        match state.current_state {
            CircuitState::Closed => {
                if success {
                    state.success_count += 1;
                    state.consecutive_successes += 1;
                    state.failure_count = 0; // Reset failure count on success
                } else {
                    state.failure_count += 1;
                    state.consecutive_successes = 0;
                    state.last_failure_time = Some(Instant::now());
                    
                    // Check if we should open the circuit
                    if self.should_open_circuit(&state) {
                        warn!("Circuit breaker '{}' opening due to {} failures", 
                              self.name, state.failure_count);
                        state.current_state = CircuitState::Open;
                        state.next_attempt = Some(Instant::now() + self.config.timeout);
                    }
                }
            }
            CircuitState::HalfOpen => {
                if success {
                    state.consecutive_successes += 1;
                    
                    // Check if we should close the circuit
                    if state.consecutive_successes >= self.config.success_threshold {
                        info!("Circuit breaker '{}' closing after {} consecutive successes", 
                              self.name, state.consecutive_successes);
                        state.current_state = CircuitState::Closed;
                        state.failure_count = 0;
                        state.consecutive_successes = 0;
                    }
                } else {
                    warn!("Circuit breaker '{}' reopening due to failure in half-open state", 
                          self.name);
                    state.current_state = CircuitState::Open;
                    state.next_attempt = Some(Instant::now() + self.config.timeout);
                    state.consecutive_successes = 0;
                    state.failure_count += 1;
                }
            }
            CircuitState::Open => {
                // In open state, we shouldn't be recording calls
                // This might happen due to race conditions
                debug!("Recording call result while circuit breaker '{}' is open", self.name);
            }
        }
    }

    /// Check if circuit should be opened based on current state
    fn should_open_circuit(&self, state: &CircuitBreakerState) -> bool {
        // Need minimum number of calls before considering opening
        if state.recent_calls.len() < self.config.minimum_requests as usize {
            return false;
        }

        // Calculate failure rate in recent window
        let recent_failures = state.recent_calls.iter()
            .filter(|call| !call.success)
            .count();
        
        let total_recent_calls = state.recent_calls.len();
        let failure_rate = recent_failures as f64 / total_recent_calls as f64;

        // Open if failure rate exceeds threshold
        let threshold_rate = self.config.failure_threshold as f64 / self.config.window_size as f64;
        failure_rate >= threshold_rate
    }

    /// Get current circuit breaker state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.current_state.clone()
    }

    /// Get circuit breaker statistics
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        
        let recent_calls = &state.recent_calls;
        let total_calls = recent_calls.len();
        let successful_calls = recent_calls.iter().filter(|call| call.success).count();
        let failed_calls = total_calls - successful_calls;
        
        let success_rate = if total_calls > 0 {
            (successful_calls as f64 / total_calls as f64) * 100.0
        } else {
            100.0
        };

        CircuitBreakerStats {
            name: self.name.clone(),
            current_state: state.current_state.clone(),
            total_calls: total_calls as u32,
            successful_calls: successful_calls as u32,
            failed_calls: failed_calls as u32,
            success_rate,
            consecutive_successes: state.consecutive_successes,
            last_failure_time: state.last_failure_time,
            next_attempt_time: state.next_attempt,
        }
    }

    /// Reset circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        info!("Resetting circuit breaker '{}'", self.name);
        
        state.current_state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.consecutive_successes = 0;
        state.last_failure_time = None;
        state.next_attempt = None;
        state.recent_calls.clear();
    }

    /// Force circuit to open state
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        warn!("Forcing circuit breaker '{}' to open state", self.name);
        
        state.current_state = CircuitState::Open;
        state.next_attempt = Some(Instant::now() + self.config.timeout);
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub current_state: CircuitState,
    pub total_calls: u32,
    pub successful_calls: u32,
    pub failed_calls: u32,
    pub success_rate: f64,
    pub consecutive_successes: u32,
    pub last_failure_time: Option<Instant>,
    pub next_attempt_time: Option<Instant>,
}

/// Execute a function with circuit breaker protection
pub async fn with_circuit_breaker<F, T, E>(
    circuit_breaker: &CircuitBreaker,
    operation: F,
) -> Result<T, CircuitBreakerError<E>>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    // Check if call should be allowed
    if !circuit_breaker.should_allow_call().await {
        return Err(CircuitBreakerError::CircuitOpen);
    }

    // Execute the operation
    let result = operation.await;
    
    // Record the result
    match &result {
        Ok(_) => circuit_breaker.record_call_result(true).await,
        Err(_) => circuit_breaker.record_call_result(false).await,
    }

    // Map the result
    result.map_err(CircuitBreakerError::OperationFailed)
}

/// Circuit breaker error types
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, call was rejected
    CircuitOpen,
    /// The underlying operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::CircuitOpen => None,
            CircuitBreakerError::OperationFailed(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            window_size: 5,
            minimum_requests: 2,
        };
        
        let cb = CircuitBreaker::new("test".to_string(), config);
        
        // Should start in closed state
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert!(cb.should_allow_call().await);
        
        // Record some successful calls
        cb.record_call_result(true).await;
        cb.record_call_result(true).await;
        
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            window_size: 5,
            minimum_requests: 2,
        };
        
        let cb = CircuitBreaker::new("test".to_string(), config);
        
        // Record enough failures to open circuit
        cb.record_call_result(false).await;
        cb.record_call_result(false).await;
        cb.record_call_result(false).await;
        
        // Circuit should be open now
        assert_eq!(cb.get_state().await, CircuitState::Open);
        assert!(!cb.should_allow_call().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            window_size: 5,
            minimum_requests: 2,
        };
        
        let cb = CircuitBreaker::new("test".to_string(), config);
        
        // Open the circuit
        cb.record_call_result(false).await;
        cb.record_call_result(false).await;
        cb.record_call_result(false).await;
        
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(60)).await;
        
        // Should transition to half-open
        assert!(cb.should_allow_call().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }
}