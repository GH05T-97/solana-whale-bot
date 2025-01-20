// src/execution/retry.rs
use tokio::time::Duration;
use std::future::Future;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
    pub retry_on_errors: Vec<ExecutionErrorType>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
            retry_on_errors: vec![
                ExecutionErrorType::RpcError,
                ExecutionErrorType::NetworkError,
                ExecutionErrorType::TimeoutError,
            ],
        }
    }
}

pub struct RetryHandler {
    config: RetryConfig,
}

impl RetryHandler {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub async fn retry<F, Fut, T>(&self, operation: F) -> Result<T, ExecutionError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ExecutionError>>,
    {
        let mut attempts = 0;
        let mut delay = self.config.initial_delay;

        loop {
            attempts += 1;
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !self.should_retry(&error) || attempts >= self.config.max_attempts {
                        return Err(error);
                    }

                    log::warn!(
                        "Operation failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempts,
                        self.config.max_attempts,
                        error,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        delay.mul_f64(self.config.backoff_factor),
                        self.config.max_delay,
                    );
                }
            }
        }
    }

    fn should_retry(&self, error: &ExecutionError) -> bool {
        self.config.retry_on_errors.contains(&error.error_type())
    }
}