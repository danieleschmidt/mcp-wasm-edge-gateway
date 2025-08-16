//! Advanced retry mechanisms with exponential backoff and jitter

use crate::{Error, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, info};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to add jitter to delays
    pub use_jitter: bool,
    /// Maximum jitter amount (percentage of delay)
    pub max_jitter: f64,
    /// Timeout for individual attempts
    pub attempt_timeout: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
            max_jitter: 0.1, // 10% jitter
            attempt_timeout: Some(Duration::from_secs(30)),
        }
    }
}

/// Retry strategy for different error types
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// No retry
    NoRetry,
    /// Fixed delay between retries
    FixedDelay(Duration),
    /// Exponential backoff with optional jitter
    ExponentialBackoff {
        config: RetryConfig,
    },
    /// Custom retry logic
    Custom {
        delays: Vec<Duration>,
    },
}

impl RetryStrategy {
    /// Create exponential backoff strategy with default configuration
    pub fn exponential_backoff() -> Self {
        Self::ExponentialBackoff {
            config: RetryConfig::default(),
        }
    }

    /// Create exponential backoff strategy with custom configuration
    pub fn exponential_backoff_with_config(config: RetryConfig) -> Self {
        Self::ExponentialBackoff { config }
    }

    /// Create fixed delay strategy
    pub fn fixed_delay(delay: Duration) -> Self {
        Self::FixedDelay(delay)
    }

    /// Create custom strategy with specific delays
    pub fn custom_delays(delays: Vec<Duration>) -> Self {
        Self::Custom { delays }
    }

    /// Get the strategy that should be used for a specific error
    pub fn for_error(error: &Error) -> Self {
        match error {
            Error::Network(_) => Self::ExponentialBackoff {
                config: RetryConfig {
                    max_attempts: 3,
                    base_delay: Duration::from_millis(1000),
                    max_delay: Duration::from_secs(10),
                    backoff_multiplier: 2.0,
                    use_jitter: true,
                    max_jitter: 0.2,
                    attempt_timeout: Some(Duration::from_secs(30)),
                },
            },
            Error::Timeout(_) => Self::ExponentialBackoff {
                config: RetryConfig {
                    max_attempts: 2,
                    base_delay: Duration::from_millis(2000),
                    max_delay: Duration::from_secs(15),
                    backoff_multiplier: 1.5,
                    use_jitter: true,
                    max_jitter: 0.1,
                    attempt_timeout: Some(Duration::from_secs(45)),
                },
            },
            Error::ResourceExhausted(_) => Self::ExponentialBackoff {
                config: RetryConfig {
                    max_attempts: 5,
                    base_delay: Duration::from_millis(5000),
                    max_delay: Duration::from_secs(60),
                    backoff_multiplier: 1.5,
                    use_jitter: true,
                    max_jitter: 0.3,
                    attempt_timeout: Some(Duration::from_secs(60)),
                },
            },
            Error::Model(_) => Self::ExponentialBackoff {
                config: RetryConfig {
                    max_attempts: 2,
                    base_delay: Duration::from_millis(3000),
                    max_delay: Duration::from_secs(20),
                    backoff_multiplier: 2.0,
                    use_jitter: true,
                    max_jitter: 0.15,
                    attempt_timeout: Some(Duration::from_secs(60)),
                },
            },
            Error::Security(_) | Error::Configuration(_) => Self::NoRetry,
            _ => Self::ExponentialBackoff {
                config: RetryConfig::default(),
            },
        }
    }
}

/// Retry executor for operations that may fail
pub struct RetryExecutor {
    strategy: RetryStrategy,
    operation_name: String,
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(strategy: RetryStrategy, operation_name: String) -> Self {
        Self {
            strategy,
            operation_name,
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
        E: Into<Error> + std::fmt::Debug + Clone,
    {
        match &self.strategy {
            RetryStrategy::NoRetry => {
                debug!("Executing {} without retry", self.operation_name);
                operation().await.map_err(|e| e.into())
            }
            RetryStrategy::FixedDelay(delay) => {
                self.execute_with_fixed_delay(*delay, operation).await
            }
            RetryStrategy::ExponentialBackoff { config } => {
                self.execute_with_exponential_backoff(config, operation).await
            }
            RetryStrategy::Custom { delays } => {
                self.execute_with_custom_delays(delays, operation).await
            }
        }
    }

    /// Execute with fixed delay between retries
    async fn execute_with_fixed_delay<F, T, E>(&self, delay: Duration, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
        E: Into<Error> + std::fmt::Debug + Clone,
    {
        let mut last_error = None;

        for attempt in 1..=3 {
            debug!("Executing {} (attempt {}/3)", self.operation_name, attempt);

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Operation {} succeeded on attempt {}", self.operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error = e.into();
                    warn!("Operation {} failed on attempt {}: {:?}", self.operation_name, attempt, error);
                    last_error = Some(error);

                    if attempt < 3 {
                        debug!("Waiting {:?} before retry", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("No error recorded".to_string())))
    }

    /// Execute with exponential backoff
    async fn execute_with_exponential_backoff<F, T, E>(
        &self,
        config: &RetryConfig,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
        E: Into<Error> + std::fmt::Debug + Clone,
    {
        let mut last_error = None;
        let mut current_delay = config.base_delay;

        for attempt in 1..=config.max_attempts {
            debug!(
                "Executing {} (attempt {}/{})",
                self.operation_name, attempt, config.max_attempts
            );

            let result = if let Some(timeout) = config.attempt_timeout {
                tokio::time::timeout(timeout, operation()).await
            } else {
                Ok(operation().await)
            };

            match result {
                Ok(Ok(result)) => {
                    if attempt > 1 {
                        info!(
                            "Operation {} succeeded on attempt {}",
                            self.operation_name, attempt
                        );
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    let error = e.into();
                    warn!(
                        "Operation {} failed on attempt {}: {:?}",
                        self.operation_name, attempt, error
                    );
                    last_error = Some(error);
                }
                Err(_) => {
                    let error = Error::Timeout(format!(
                        "Operation {} timed out on attempt {}",
                        self.operation_name, attempt
                    ));
                    warn!("{:?}", error);
                    last_error = Some(error);
                }
            }

            // Don't sleep after the last attempt
            if attempt < config.max_attempts {
                let delay_with_jitter = if config.use_jitter {
                    add_jitter(current_delay, config.max_jitter)
                } else {
                    current_delay
                };

                debug!("Waiting {:?} before retry", delay_with_jitter);
                sleep(delay_with_jitter).await;

                // Calculate next delay with exponential backoff
                current_delay = Duration::from_millis(
                    ((current_delay.as_millis() as f64) * config.backoff_multiplier) as u64,
                );
                current_delay = current_delay.min(config.max_delay);
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("No error recorded".to_string())))
    }

    /// Execute with custom delays
    async fn execute_with_custom_delays<F, T, E>(
        &self,
        delays: &[Duration],
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
        E: Into<Error> + std::fmt::Debug + Clone,
    {
        let max_attempts = delays.len() + 1;
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            debug!(
                "Executing {} (attempt {}/{})",
                self.operation_name, attempt, max_attempts
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!(
                            "Operation {} succeeded on attempt {}",
                            self.operation_name, attempt
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error = e.into();
                    warn!(
                        "Operation {} failed on attempt {}: {:?}",
                        self.operation_name, attempt, error
                    );
                    last_error = Some(error);

                    // Sleep with custom delay if not the last attempt
                    if attempt <= delays.len() {
                        let delay = delays[attempt - 1];
                        debug!("Waiting {:?} before retry", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("No error recorded".to_string())))
    }
}

/// Add jitter to a delay to avoid thundering herd problems
fn add_jitter(delay: Duration, max_jitter: f64) -> Duration {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Use thread ID as seed for reproducible but varied jitter
    let mut hasher = DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    let seed = hasher.finish();

    // Simple linear congruential generator
    let random = ((seed.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) & 0x7fff;
    let jitter_factor = (random as f64 / 32767.0) * max_jitter;

    let delay_ms = delay.as_millis() as f64;
    let jitter_ms = delay_ms * jitter_factor;
    let final_delay_ms = delay_ms + jitter_ms;

    Duration::from_millis(final_delay_ms as u64)
}

/// Convenience function to retry an operation
pub async fn retry_operation<F, T, E>(
    operation_name: &str,
    strategy: RetryStrategy,
    operation: F,
) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
    E: Into<Error> + std::fmt::Debug + Clone,
{
    let executor = RetryExecutor::new(strategy, operation_name.to_string());
    executor.execute(operation).await
}

/// Convenience function to retry with error-specific strategy
pub async fn retry_for_error<F, T, E>(
    operation_name: &str,
    sample_error: &Error,
    operation: F,
) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send + 'static>>,
    E: Into<Error> + std::fmt::Debug + Clone,
{
    let strategy = RetryStrategy::for_error(sample_error);
    retry_operation(operation_name, strategy, operation).await
}

/// Retry policy for specific operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub network_requests: RetryStrategy,
    pub model_inference: RetryStrategy,
    pub database_operations: RetryStrategy,
    pub file_operations: RetryStrategy,
    pub queue_operations: RetryStrategy,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            network_requests: RetryStrategy::exponential_backoff_with_config(RetryConfig {
                max_attempts: 3,
                base_delay: Duration::from_millis(1000),
                max_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
                use_jitter: true,
                max_jitter: 0.2,
                attempt_timeout: Some(Duration::from_secs(30)),
            }),
            model_inference: RetryStrategy::exponential_backoff_with_config(RetryConfig {
                max_attempts: 2,
                base_delay: Duration::from_millis(3000),
                max_delay: Duration::from_secs(20),
                backoff_multiplier: 2.0,
                use_jitter: true,
                max_jitter: 0.15,
                attempt_timeout: Some(Duration::from_secs(60)),
            }),
            database_operations: RetryStrategy::exponential_backoff_with_config(RetryConfig {
                max_attempts: 5,
                base_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(5),
                backoff_multiplier: 1.5,
                use_jitter: true,
                max_jitter: 0.1,
                attempt_timeout: Some(Duration::from_secs(10)),
            }),
            file_operations: RetryStrategy::exponential_backoff_with_config(RetryConfig {
                max_attempts: 3,
                base_delay: Duration::from_millis(1000),
                max_delay: Duration::from_secs(5),
                backoff_multiplier: 2.0,
                use_jitter: false,
                max_jitter: 0.0,
                attempt_timeout: Some(Duration::from_secs(15)),
            }),
            queue_operations: RetryStrategy::exponential_backoff_with_config(RetryConfig {
                max_attempts: 5,
                base_delay: Duration::from_millis(2000),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 1.5,
                use_jitter: true,
                max_jitter: 0.25,
                attempt_timeout: Some(Duration::from_secs(30)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_operation() {
        let strategy = RetryStrategy::exponential_backoff();
        let executor = RetryExecutor::new(strategy, "test_op".to_string());

        let result = executor
            .execute(|| {
                Box::pin(async { 
                    Ok::<i32, Error>(42) 
                })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_on_failure() {
        let strategy = RetryStrategy::fixed_delay(Duration::from_millis(10));
        let executor = RetryExecutor::new(strategy, "test_op".to_string());
        let counter = Arc::new(AtomicU32::new(0));

        let result = executor
            .execute(|| {
                let counter = counter.clone();
                Box::pin(async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(Error::Network("temporary failure".to_string()))
                    } else {
                        Ok(42)
                    }
                })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 3 attempts
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let strategy = RetryStrategy::fixed_delay(Duration::from_millis(1));
        let executor = RetryExecutor::new(strategy, "test_op".to_string());

        let result = executor
            .execute(|| {
                Box::pin(async { 
                    Err::<i32, Error>(Error::Network("persistent failure".to_string()))
                })
            })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_jitter_calculation() {
        let delay = Duration::from_millis(1000);
        let jittered = add_jitter(delay, 0.1);

        // Jittered delay should be within 10% of original
        let original_ms = delay.as_millis() as f64;
        let jittered_ms = jittered.as_millis() as f64;
        let diff_percent = (jittered_ms - original_ms).abs() / original_ms;

        assert!(diff_percent <= 0.1);
    }
}