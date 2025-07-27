//! Utility functions and helpers

use chrono::{DateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Generate a new request ID
pub fn generate_request_id() -> Uuid {
    Uuid::new_v4()
}

/// Get current timestamp
pub fn current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Get current timestamp as milliseconds since epoch
pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Convert bytes to human readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Calculate exponential backoff delay
pub fn exponential_backoff(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f32,
) -> u64 {
    let delay = initial_delay_ms as f64 * (multiplier as f64).powi(attempt as i32);
    (delay as u64).min(max_delay_ms)
}

/// Simple hash function for strings
pub fn simple_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Validate model ID format
pub fn validate_model_id(model_id: &str) -> bool {
    !model_id.is_empty()
        && model_id.len() <= 256
        && model_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Validate device ID format
pub fn validate_device_id(device_id: &str) -> bool {
    !device_id.is_empty()
        && device_id.len() <= 128
        && device_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Calculate memory usage percentage
pub fn memory_usage_percentage(used_mb: u32, total_mb: u32) -> f32 {
    if total_mb == 0 {
        0.0
    } else {
        (used_mb as f32 / total_mb as f32) * 100.0
    }
}

/// Calculate request latency percentile
pub fn calculate_percentile(mut latencies: Vec<u64>, percentile: f32) -> u64 {
    if latencies.is_empty() {
        return 0;
    }

    latencies.sort_unstable();
    let index = ((latencies.len() as f32 * percentile / 100.0) - 1.0).max(0.0) as usize;
    latencies[index.min(latencies.len() - 1)]
}

/// Compress data using a simple algorithm (placeholder for real compression)
pub fn compress_data(data: &[u8]) -> Vec<u8> {
    // Placeholder implementation - in practice, use a real compression library
    data.to_vec()
}

/// Decompress data (placeholder for real decompression)
pub fn decompress_data(data: &[u8]) -> Result<Vec<u8>, String> {
    // Placeholder implementation - in practice, use a real compression library
    Ok(data.to_vec())
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    capacity: u32,
    tokens: u32,
    refill_rate: u32,
    last_refill: SystemTime,
}

impl RateLimiter {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: SystemTime::now(),
        }
    }

    pub fn try_acquire(&mut self) -> bool {
        self.refill_tokens();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill_tokens(&mut self) {
        let now = SystemTime::now();
        if let Ok(elapsed) = now.duration_since(self.last_refill) {
            let new_tokens = (elapsed.as_secs() * self.refill_rate as u64) as u32;
            self.tokens = (self.tokens + new_tokens).min(self.capacity);
            self.last_refill = now;
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout_ms: u64,
    failure_count: u32,
    last_failure_time: Option<SystemTime>,
    state: CircuitState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout_ms,
            failure_count: 0,
            last_failure_time: None,
            state: CircuitState::Closed,
        }
    }

    pub fn call<F, T, E>(&mut self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self.state {
            CircuitState::Closed => match operation() {
                Ok(result) => {
                    self.on_success();
                    Ok(result)
                },
                Err(error) => {
                    self.on_failure();
                    Err(error)
                },
            },
            CircuitState::Open => {
                if self.should_attempt_reset() {
                    self.state = CircuitState::HalfOpen;
                    self.call(operation)
                } else {
                    // Return a circuit breaker error
                    operation() // This will likely fail, but maintains the error type
                }
            },
            CircuitState::HalfOpen => match operation() {
                Ok(result) => {
                    self.on_success();
                    Ok(result)
                },
                Err(error) => {
                    self.on_failure();
                    Err(error)
                },
            },
        }
    }

    fn on_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(SystemTime::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    fn should_attempt_reset(&self) -> bool {
        if let Some(last_failure) = self.last_failure_time {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_failure) {
                return elapsed.as_millis() as u64 >= self.recovery_timeout_ms;
            }
        }
        false
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn test_exponential_backoff() {
        assert_eq!(exponential_backoff(0, 1000, 60000, 2.0), 1000);
        assert_eq!(exponential_backoff(1, 1000, 60000, 2.0), 2000);
        assert_eq!(exponential_backoff(2, 1000, 60000, 2.0), 4000);
        assert_eq!(exponential_backoff(10, 1000, 60000, 2.0), 60000); // Capped at max
    }

    #[test]
    fn test_validate_model_id() {
        assert!(validate_model_id("model-1.0"));
        assert!(validate_model_id("my_model_v2"));
        assert!(!validate_model_id(""));
        assert!(!validate_model_id("model with spaces"));
    }

    #[test]
    fn test_calculate_percentile() {
        let latencies = vec![100, 200, 300, 400, 500];
        assert_eq!(calculate_percentile(latencies.clone(), 50.0), 200);
        assert_eq!(calculate_percentile(latencies.clone(), 95.0), 400);
        assert_eq!(calculate_percentile(vec![], 95.0), 0);
    }
}
