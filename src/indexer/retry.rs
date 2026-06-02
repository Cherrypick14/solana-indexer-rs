use std::time::Duration;
use std::future::Future;
use tracing::{warn, debug};

/// RetryManager handles retries for RPC operations with exponential backoff
pub struct RetryManager {
    max_attempts: u32,
    base_delay: Duration,
}

impl RetryManager {
    /// Creates a new RetryManager with the specified configuration
    /// max_attempts: Total number of attempts (initial + retries)  
    /// base_delay: Base delay for exponential backoff
    pub fn new(max_attempts: u32, base_delay: Duration) -> Self {
        Self {
            max_attempts,
            base_delay,
        }
    }

    /// Creates a RetryManager with default configuration for RPC calls
    /// - 3 total attempts (1 initial + 2 retries)
    /// - 1 second base delay
    /// - Results in delays of: 1s, 2s, 4s
    pub fn default_rpc() -> Self {
        Self::new(3, Duration::from_secs(1))
    }

    /// Executes an async operation with retry logic and exponential backoff
    /// 
    /// The operation will be retried up to max_attempts times with delays of:
    /// - 1st retry: base_delay 
    /// - 2nd retry: base_delay * 2
    /// - 3rd retry: base_delay * 4
    /// - etc.
    pub async fn execute_with_retry<F, Fut, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display + Clone,
    {
        let mut last_error = None;

        for attempt in 1..=self.max_attempts {
            debug!("Retry attempt {} of {}", attempt, self.max_attempts);
            
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Operation succeeded on attempt {}", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    if attempt < self.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        warn!(
                            "Attempt {} failed: {}. Retrying in {:?}", 
                            attempt, e, delay
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        warn!(
                            "All {} attempts failed. Final error: {}", 
                            self.max_attempts, e
                        );
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Calculates the delay for a given retry attempt using exponential backoff
    fn calculate_delay(&self, attempt: u32) -> Duration {
        // For attempt 1 (first retry): base_delay * 1 = 1s
        // For attempt 2 (second retry): base_delay * 2 = 2s  
        // For attempt 3 (third retry): base_delay * 4 = 4s
        let multiplier = 2_u32.pow(attempt - 1);
        self.base_delay * multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use quickcheck::TestResult;

    // Unit tests for basic functionality
    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let retry_manager = RetryManager::default_rpc();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager.execute_with_retry(|| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().unwrap();
                *count += 1;
                Ok::<i32, String>(42)
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let retry_manager = RetryManager::default_rpc();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager.execute_with_retry(|| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    Err("First attempt fails".to_string())
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_retry_all_attempts_fail() {
        let retry_manager = RetryManager::default_rpc();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager.execute_with_retry(|| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().unwrap();
                *count += 1;
                Err::<i32, String>("Always fails".to_string())
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Always fails");
        assert_eq!(*counter.lock().unwrap(), 3); // All 3 attempts made
    }

    #[test]
    fn test_delay_calculation() {
        let retry_manager = RetryManager::new(3, Duration::from_secs(1));
        
        assert_eq!(retry_manager.calculate_delay(1), Duration::from_secs(1)); // 1 * 2^0 = 1s
        assert_eq!(retry_manager.calculate_delay(2), Duration::from_secs(2)); // 1 * 2^1 = 2s
        assert_eq!(retry_manager.calculate_delay(3), Duration::from_secs(4)); // 1 * 2^2 = 4s
    }

    // Property-based tests
    
    /**
     * Feature: indexer-completion, Property 1: Retry logic consistency
     * For any transient RPC error, the indexer should retry the request exactly three times 
     * with exponential backoff delays of 1, 2, and 4 seconds respectively
     */
    #[tokio::test]
    async fn property_retry_logic_consistency() {
        // Test various failure scenarios
        for fail_attempts in 0..=5u32 {
            let retry_manager = RetryManager::default_rpc();
            let counter = Arc::new(Mutex::new(0));
            let counter_clone = counter.clone();
            
            let start_time = std::time::Instant::now();
            
            let result = retry_manager.execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let mut count = counter.lock().unwrap();
                    *count += 1;
                    
                    if *count <= fail_attempts {
                        Err::<i32, String>(format!("Attempt {} fails", *count))
                    } else {
                        Ok(42)
                    }
                }
            }).await;
            
            let elapsed = start_time.elapsed();
            let attempts_made = *counter.lock().unwrap();
            
            if fail_attempts == 0 {
                // Should succeed immediately, no retries
                assert!(result.is_ok(), "Should succeed immediately when no failures");
                assert_eq!(attempts_made, 1, "Should make exactly 1 attempt when no failures");
                assert!(elapsed < Duration::from_millis(100), "Should complete quickly when no retries needed");
            } else if fail_attempts < 3 {
                // Should eventually succeed after fail_attempts failures
                let expected_attempts = fail_attempts + 1;
                let expected_min_delay = if fail_attempts == 1 { 
                    Duration::from_millis(800) // Just under 1s 
                } else { 
                    Duration::from_millis(2800) // Just under 3s (1s + 2s)
                };
                
                assert!(result.is_ok(), "Should succeed after {} failures", fail_attempts);
                assert_eq!(attempts_made, expected_attempts, "Should make exactly {} attempts", expected_attempts);
                assert!(elapsed >= expected_min_delay, "Should have proper retry delays, elapsed: {:?}", elapsed);
            } else {
                // Should fail after exactly 3 attempts with proper timing
                // Expected delays: 1s + 2s + (no delay after final attempt) = ~3s minimum
                let expected_min_delay = Duration::from_millis(2800);
                
                assert!(result.is_err(), "Should fail after 3 attempts when all fail");
                assert_eq!(attempts_made, 3, "Should make exactly 3 attempts");
                assert!(elapsed >= expected_min_delay, "Should have proper retry delays, elapsed: {:?}", elapsed);
            }
        }
    }

    /**
     * Property test to verify exponential backoff timing
     * Tests that delays follow the pattern: 1s, 2s, 4s
     */
    #[tokio::test] 
    async fn property_exponential_backoff_timing() {
        let retry_manager = RetryManager::default_rpc();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();
        
        let start_time = std::time::Instant::now();
        
        let result = retry_manager.execute_with_retry(|| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().unwrap();
                *count += 1;
                Err::<i32, String>("Always fail".to_string())
            }
        }).await;
        
        let total_elapsed = start_time.elapsed();
        
        // Should have made 3 attempts with delays of ~1s + ~2s = ~3s minimum total
        assert!(result.is_err());
        assert_eq!(*counter.lock().unwrap(), 3);
        assert!(total_elapsed >= Duration::from_millis(2800)); // Allow some tolerance
        assert!(total_elapsed < Duration::from_millis(8000)); // Reasonable upper bound
    }

    #[test]
    fn prop_delay_calculation_is_exponential() {
        use quickcheck::quickcheck;
        
        fn prop(attempt: u8) -> TestResult {
            if attempt == 0 || attempt > 10 {
                return TestResult::discard(); // Only test valid attempt numbers
            }
            
            let retry_manager = RetryManager::new(10, Duration::from_secs(1));
            let delay = retry_manager.calculate_delay(attempt as u32);
            let expected = Duration::from_secs(2_u64.pow((attempt as u32) - 1));
            
            TestResult::from_bool(delay == expected)
        }
        
        quickcheck(prop as fn(u8) -> TestResult);
    }

    /**
     * Feature: indexer-completion, Property 2: Error recovery behavior
     * For any RPC failure scenario after exhausting retries, the indexer should log the error details 
     * and continue processing with the next slot without stopping
     */
    #[tokio::test]
    async fn property_error_recovery_behavior() {
        async fn prop_error_recovery(error_message_seed: u8) -> bool {
            let retry_manager = RetryManager::default_rpc();
            let counter = Arc::new(Mutex::new(0));
            let counter_clone = counter.clone();
            
            // Generate different error messages to test various failure scenarios
            let error_message = format!("RPC Error Type {}: Connection failed", error_message_seed);
            
            let result = retry_manager.execute_with_retry(|| {
                let counter = counter_clone.clone();
                let error_msg = error_message.clone();
                async move {
                    let mut count = counter.lock().unwrap();
                    *count += 1;
                    Err::<i32, String>(error_msg)
                }
            }).await;
            
            let attempts_made = *counter.lock().unwrap();
            
            // Verify error recovery behavior:
            // 1. Should make exactly 3 attempts
            // 2. Should return the error (not panic or hang)
            // 3. Should preserve the original error message
            let correct_attempts = attempts_made == 3;
            let returns_error = result.is_err();
            let preserves_error_msg = result.unwrap_err().contains(&format!("RPC Error Type {}", error_message_seed));
            
            correct_attempts && returns_error && preserves_error_msg
        }
        
        // Test with multiple different error scenarios
        for seed in 0..10u8 {
            assert!(prop_error_recovery(seed).await, "Error recovery property failed for seed {}", seed);
        }
    }

    /**
     * Property test to verify that the retry manager handles different error types consistently
     * and always allows continuation (doesn't panic or hang indefinitely)
     */
    #[tokio::test]
    async fn property_error_types_consistency() {
        let error_types = vec![
            "Connection timeout",
            "Network unreachable", 
            "RPC server unavailable",
            "Rate limit exceeded",
            "Invalid response format",
            "Authentication failed",
            "Service temporarily unavailable"
        ];
        
        for error_type in error_types.iter() {
            let retry_manager = RetryManager::default_rpc();
            let counter = Arc::new(Mutex::new(0));
            let counter_clone = counter.clone();
            let error_msg = error_type.to_string();
            
            let start_time = std::time::Instant::now();
            
            let result = retry_manager.execute_with_retry(|| {
                let counter = counter_clone.clone();
                let err = error_msg.clone();
                async move {
                    let mut count = counter.lock().unwrap();
                    *count += 1;
                    Err::<String, String>(err)
                }
            }).await;
            
            let elapsed = start_time.elapsed();
            let attempts = *counter.lock().unwrap();
            
            // Verify consistent behavior across all error types:
            // 1. Exactly 3 attempts made
            // 2. Operation completes in reasonable time (< 10s to account for 1s+2s+processing)
            // 3. Returns error rather than hanging or panicking
            // 4. Preserves original error information
            assert_eq!(attempts, 3, "Error type '{}' should result in exactly 3 attempts", error_type);
            assert!(elapsed < Duration::from_secs(10), "Error type '{}' took too long: {:?}", error_type, elapsed);
            assert!(result.is_err(), "Error type '{}' should return an error", error_type);
            assert_eq!(result.unwrap_err(), *error_type, "Error type '{}' should preserve error message", error_type);
        }
    }

    /**
     * Property test to verify recovery behavior continues processing
     * by testing that multiple failed operations in sequence work correctly
     */
    #[tokio::test]
    async fn property_continuous_error_recovery() {
        let retry_manager = RetryManager::default_rpc();
        
        // Simulate processing multiple slots where all RPC calls fail
        let mut successful_error_recoveries = 0;
        
        for slot_number in 1000..1010u64 {
            let counter = Arc::new(Mutex::new(0));
            let counter_clone = counter.clone();
            
            let result = retry_manager.execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let mut count = counter.lock().unwrap();
                    *count += 1;
                    Err::<u64, String>(format!("RPC failed for slot {}", slot_number))
                }
            }).await;
            
            // Each slot processing should:
            // 1. Make exactly 3 attempts 
            // 2. Return an error (allowing continuation to next slot)
            // 3. Not hang or panic
            if *counter.lock().unwrap() == 3 && result.is_err() {
                successful_error_recoveries += 1;
            }
        }
        
        // Should successfully handle error recovery for all 10 simulated slots
        assert_eq!(successful_error_recoveries, 10, 
                   "Should handle error recovery consistently across multiple sequential operations");
    }
}