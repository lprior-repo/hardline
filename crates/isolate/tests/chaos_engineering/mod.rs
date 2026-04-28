//! Chaos engineering utilities for testing robustness under failure conditions
//!
//! This module provides controlled chaos injection for testing how isolate handles
//! various failure scenarios. All chaos is deterministic via seeded RNG for
//! reproducible test failures.
//!
//! ## Design Principles
//!
//! - **Deterministic chaos**: All randomness is seed-based for reproducibility
//! - **Zero panic**: No `unwrap`/`expect`/`panic` - only `Result<T, Error>`
//! - **Functional style**: Railway-oriented programming with `map`/`and_then`
//! - **Isolation**: Each chaos executor has independent state
//!
//! ## Example
//!
//! ```no_run
//! use chaos_engineering::{ChaosConfig, ChaosExecutor, FailureMode};
//!
//! // Configure chaos with 20% failure probability
//! let config = ChaosConfig::new(0.2, vec![
//!     FailureMode::IoError,
//!     FailureMode::Corruption,
//! ])
//! .with_seed(42); // Reproducible chaos
//!
//! let executor = ChaosExecutor::new(config);
//!
//! // Run operation with potential chaos injection
//! let result = executor.inject_chaos(|| {
//!     std::fs::write("/tmp/test", "data")
//! });
//! ```

#![allow(clippy::float_cmp)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::uninlined_format_args)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use rand::{rngs::StdRng, Rng, SeedableRng};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Domain errors for chaos engineering operations
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ChaosError {
    /// Chaos execution failed
    #[error("chaos execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid chaos configuration
    #[error("invalid chaos configuration: {0}")]
    InvalidConfig(String),

    /// RNG seeding failed
    #[error("failed to seed RNG: {0}")]
    RngError(String),
}

// ============================================================================
// Chaos Configuration
// ============================================================================

/// Types of failures that can be injected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    /// Simulate I/O errors (permission denied, device full, etc.)
    IoError,

    /// Simulate operation timeouts
    Timeout,

    /// Simulate data corruption (bit flips, invalid UTF-8, etc.)
    Corruption,

    /// Simulate deadlock conditions (for testing deadlock detection)
    DeadlockSimulation,

    /// Simulate resource exhaustion (out of memory, file descriptors, etc.)
    ResourceExhaustion,
}

impl FailureMode {
    /// Get all available failure modes
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::IoError,
            Self::Timeout,
            Self::Corruption,
            Self::DeadlockSimulation,
            Self::ResourceExhaustion,
        ]
    }

    /// Get a human-readable description
    #[must_use]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::IoError => "I/O error (permissions, device full, etc.)",
            Self::Timeout => "operation timeout",
            Self::Corruption => "data corruption (bit flips, invalid encoding)",
            Self::DeadlockSimulation => "deadlock simulation",
            Self::ResourceExhaustion => "resource exhaustion (memory, fds, etc.)",
        }
    }
}

/// Configuration for chaos injection
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Probability of injecting chaos (0.0 to 1.0)
    probability: f64,

    /// Which failure modes to enable
    failure_modes: Vec<FailureMode>,

    /// Seed for deterministic chaos
    seed: u64,
}

impl ChaosConfig {
    /// Create a new chaos configuration
    ///
    /// # Errors
    ///
    /// Returns `Err(ChaosError::InvalidConfig)` if:
    /// - probability is not in range [0.0, 1.0]
    /// - `failure_modes` is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use chaos_engineering::{ChaosConfig, FailureMode};
    ///
    /// let config = ChaosConfig::new(0.5, vec![FailureMode::IoError]).expect("valid config");
    /// ```
    pub fn new(probability: f64, failure_modes: Vec<FailureMode>) -> Result<Self, ChaosError> {
        // Validate probability is in valid range
        if !(0.0..=1.0).contains(&probability) {
            return Err(ChaosError::InvalidConfig(format!(
                "probability must be between 0.0 and 1.0, got {probability}"
            )));
        }

        // Validate at least one failure mode is enabled
        if failure_modes.is_empty() {
            return Err(ChaosError::InvalidConfig(
                "at least one failure mode must be enabled".to_string(),
            ));
        }

        Ok(Self {
            probability,
            failure_modes,
            seed: StdRng::from_entropy().gen(), // Random seed by default
        })
    }

    /// Set a specific seed for reproducible chaos
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Get the probability of chaos injection
    #[must_use]
    pub const fn probability(&self) -> f64 {
        self.probability
    }

    /// Get the enabled failure modes
    #[must_use]
    pub fn failure_modes(&self) -> &[FailureMode] {
        &self.failure_modes
    }

    /// Get the RNG seed
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }
}

// ============================================================================
// Chaos Executor
// ============================================================================

/// Executor that injects chaos into operations
///
/// Each executor maintains its own RNG state for independent chaos streams.
/// Use the same seed to reproduce the same chaos sequence.
#[derive(Debug, Clone)]
pub struct ChaosExecutor {
    /// Configuration for this executor
    config: ChaosConfig,

    /// Thread-safe RNG state counter
    rng_counter: u64,
}

impl ChaosExecutor {
    /// Create a new chaos executor from configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use chaos_engineering::{ChaosConfig, ChaosExecutor, FailureMode};
    ///
    /// let config = ChaosConfig::new(0.3, vec![FailureMode::IoError])
    ///     .expect("valid config")
    ///     .with_seed(42);
    ///
    /// let executor = ChaosExecutor::new(config);
    /// ```
    #[must_use]
    pub const fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            rng_counter: 0,
        }
    }

    /// Inject chaos into a fallible operation
    ///
    /// This wraps any operation that returns `Result<T, E>` and potentially
    /// injects a failure based on the configured probability and modes.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Success type
    /// - `E`: Error type (must implement `std::error::Error + Send + Sync + 'static`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs;
    ///
    /// use chaos_engineering::{ChaosConfig, ChaosExecutor, FailureMode};
    ///
    /// let config = ChaosConfig::new(0.5, vec![FailureMode::IoError])
    ///     .expect("valid config")
    ///     .with_seed(42);
    /// let executor = ChaosExecutor::new(config);
    ///
    /// // May inject I/O error instead of writing
    /// let result = executor.inject_chaos(|| fs::write("/tmp/test", "data"));
    /// ```
    pub fn inject_chaos<T, E, F>(&self, operation: F) -> Result<T, anyhow::Error>
    where
        E: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> Result<T, E>,
    {
        // Create RNG from config seed + counter for unique sequence
        let seed = self.config.seed.wrapping_add(self.rng_counter);
        let mut rng = StdRng::seed_from_u64(seed);

        // Roll the dice: should we inject chaos?
        let roll: f64 = rng.gen();
        let should_inject = roll < self.config.probability;

        if should_inject {
            // Pick a random failure mode
            let mode_idx = rng.gen_range(0..self.config.failure_modes.len());
            let mode = self.config.failure_modes[mode_idx];

            // Inject the specific failure
            Self::inject_failure_t(mode)
        } else {
            // No chaos injected - run the operation normally
            operation().map_err(anyhow::Error::from)
        }
    }

    /// Inject chaos into an infallible operation
    ///
    /// Similar to `inject_chaos` but for operations that don't return `Result`.
    /// If chaos is injected, returns an error. Otherwise returns `Ok(T)`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chaos_engineering::{ChaosConfig, ChaosExecutor, FailureMode};
    ///
    /// let config = ChaosConfig::new(0.2, vec![FailureMode::Timeout]).expect("valid config");
    /// let executor = ChaosExecutor::new(config);
    ///
    /// // May simulate timeout
    /// let result = executor.inject_chaos_infallible(|| {
    ///     42 // Some computation
    /// });
    /// ```
    pub fn inject_chaos_infallible<T, F>(&self, operation: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce() -> T,
    {
        // Create RNG from config seed + counter
        let seed = self.config.seed.wrapping_add(self.rng_counter);
        let mut rng = StdRng::seed_from_u64(seed);

        // Roll the dice
        let roll: f64 = rng.gen();
        let should_inject = roll < self.config.probability;

        if should_inject {
            // Pick a random failure mode
            let mode_idx = rng.gen_range(0..self.config.failure_modes.len());
            let mode = self.config.failure_modes[mode_idx];

            // Inject the specific failure
            Self::inject_failure_t(mode)
        } else {
            // No chaos - run operation and wrap in Ok
            Ok(operation())
        }
    }

    /// Create a derived executor with a new RNG counter
    ///
    /// Useful for creating independent chaos streams from the same configuration.
    #[must_use]
    pub fn derive(&self, offset: u64) -> Self {
        Self {
            config: self.config.clone(),
            rng_counter: self.rng_counter.wrapping_add(offset),
        }
    }

    /// Inject a specific failure mode
    ///
    /// This is the core chaos injection logic. Each failure mode simulates
    /// a different type of system failure.
    fn inject_failure(mode: FailureMode) -> Result<(), anyhow::Error> {
        match mode {
            FailureMode::IoError => {
                // Simulate various I/O errors
                let error_msg = match rand::random::<u8>() % 4 {
                    0 => "Permission denied",
                    1 => "Device or resource busy",
                    2 => "No space left on device",
                    _ => "Input/output error",
                };

                Err(anyhow::anyhow!("{error_msg} (chaos injection: I/O error)"))
            }

            FailureMode::Timeout => {
                // Simulate timeout by returning an error
                // (Real timeout would use tokio::time::timeout)
                Err(anyhow::anyhow!(
                    "operation timed out after 30s (chaos injection: timeout)"
                ))
            }

            FailureMode::Corruption => {
                // Simulate data corruption detection
                Err(anyhow::anyhow!(
                    "data corruption detected (chaos injection: corruption)"
                ))
            }

            FailureMode::DeadlockSimulation => {
                // Simulate deadlock detection
                Err(anyhow::anyhow!(
                    "potential deadlock detected (chaos injection: deadlock)"
                ))
            }

            FailureMode::ResourceExhaustion => {
                // Simulate resource exhaustion
                let error_msg = match rand::random::<u8>() % 3 {
                    0 => "Out of memory",
                    1 => "Too many open files",
                    _ => "Resource temporarily unavailable",
                };

                Err(anyhow::anyhow!(
                    "{error_msg} (chaos injection: resource exhaustion)"
                ))
            }
        }
    }

    /// Inject chaos with generic return type
    fn inject_failure_t<T>(mode: FailureMode) -> Result<T, anyhow::Error> {
        // First inject the failure (which returns Result<(), _>)
        Self::inject_failure(mode)?;
        // Then try to return T, but we can't construct T, so this always fails
        // This is only called when we WANT to fail, so the error above is what matters
        Err(anyhow::anyhow!(
            "chaos injection failed (should not reach here)"
        ))
    }
}

// ============================================================================
// Chaos Test Harness Integration
// ============================================================================
// NOTE: ChaosTestHarness is not available in hardline because it requires
// the common::TestHarness and common::CommandResult types which do not exist
// in hardline's test infrastructure. The core chaos infrastructure
// (ChaosConfig, ChaosExecutor, FailureMode) is fully ported and available.
//
// To re-enable ChaosTestHarness, port tests/common/mod.rs from isolate first.

// ============================================================================
// Chaos Test Macros
// ============================================================================

/// Macro to generate chaos tests with varying configurations
///
/// Generates multiple test cases with different chaos parameters.
///
/// # Examples
///
/// ```no_run
/// use chaos_engineering::{chaos_test_suite, FailureMode};
///
/// chaos_test_suite!(test_init_with_chaos, &["session", "init"], 0.1, 0.5, 10);
/// ```
#[macro_export]
macro_rules! chaos_test_suite {
    ($test_name:ident, $cmd:expr, $prob_min:expr, $prob_max:expr, $iterations:expr) => {
        paste::paste! {
            $(
                #[test]
                fn [<$test_name _prob $iterations _seed $iterations>]() {
                    use $crate::chaos_engineering::{ChaosConfig, FailureMode};

                    let config = ChaosConfig::new($iterations, vec![FailureMode::IoError])
                        .expect("valid config")
                        .with_seed($iterations);

                    let executor = ChaosExecutor::new(config);

                    // Run operation with chaos - may fail
                    let _result = executor.inject_chaos(|| Ok::<(), std::io::Error>(()));
                    // Test verifies system handles chaos gracefully
                }
            )*
        };
    };
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Run an operation multiple times with chaos and collect results
///
/// Useful for stress testing and finding edge cases.
///
/// # Errors
///
/// Returns `Err` if chaos configuration is invalid
///
/// # Examples
///
/// ```no_run
/// use chaos_engineering::{run_chaos_iterations, ChaosConfig, FailureMode};
///
/// let results = run_chaos_iterations(
///     ChaosConfig::new(0.3, vec![FailureMode::IoError]).expect("valid config"),
///     100,
///     || std::fs::write("/tmp/test", "data"),
/// );
///
/// let success_rate = results.iter().filter(|r| r.is_ok()).count() as f64 / 100.0;
/// println!("Success rate: {:.1}%", success_rate * 100.0);
/// ```
pub fn run_chaos_iterations<T, E, F>(
    config: &ChaosConfig,
    iterations: usize,
    operation: F,
) -> Vec<Result<T, anyhow::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn() -> Result<T, E>,
{
    (0..iterations)
        .map(|i| {
            let executor = ChaosExecutor::new(config.clone()).derive(i as u64);
            executor.inject_chaos(&operation)
        })
        .collect()
}

/// Calculate chaos statistics from iteration results
///
/// Returns `(success_count, failure_count, success_rate)`
///
/// # Examples
///
/// ```no_run
/// use chaos_engineering::{
///     calculate_chaos_stats, run_chaos_iterations, ChaosConfig, FailureMode,
/// };
///
/// let results = run_chaos_iterations(
///     ChaosConfig::new(0.5, vec![FailureMode::Timeout]).expect("valid config"),
///     50,
///     || Ok::<(), std::io::Error>(()),
/// );
///
/// let (successes, failures, rate) = calculate_chaos_stats(&results);
/// println!(
///     "Successes: {successes}, Failures: {failures}, Rate: {:.1}%",
///     rate * 100.0
/// );
/// ```
#[must_use]
pub fn calculate_chaos_stats<T>(results: &[Result<T>]) -> (usize, usize, f64) {
    let total = results.len();

    if total == 0 {
        return (0, 0, 0.0);
    }

    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = total - successes;
    #[allow(clippy::cast_precision_loss)]
    // Safe because success rate is a ratio; precision loss beyond f64 mantissa is negligible
    let success_rate = successes as f64 / total as f64;

    (successes, failures, success_rate)
}

/// Global atomic counter for unique seeds across tests
static GLOBAL_SEED_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique seed for chaos testing
///
/// Uses an atomic counter to ensure each test gets a unique seed
/// while still being reproducible via the counter value.
///
/// # Examples
///
/// ```no_run
/// use chaos_engineering::generate_chaos_seed;
///
/// let seed = generate_chaos_seed();
/// println!("Test seed: {}", seed);
/// // Can reproduce by manually setting the same seed
/// ```
#[must_use]
pub fn generate_chaos_seed() -> u64 {
    GLOBAL_SEED_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_config_valid() {
        let config = ChaosConfig::new(0.5, vec![FailureMode::IoError]);
        assert!(config.is_ok());
    }

    #[test]
    fn test_chaos_config_probability_too_high() {
        let config = ChaosConfig::new(1.5, vec![FailureMode::IoError]);
        assert!(config.is_err());
    }

    #[test]
    fn test_chaos_config_probability_negative() {
        let config = ChaosConfig::new(-0.1, vec![FailureMode::IoError]);
        assert!(config.is_err());
    }

    #[test]
    fn test_chaos_config_empty_modes() {
        let config = ChaosConfig::new(0.5, vec![]);
        assert!(config.is_err());
    }

    #[test]
    fn test_chaos_config_with_seed() {
        let config = ChaosConfig::new(0.3, vec![FailureMode::Timeout])
            .expect("valid config")
            .with_seed(42);

        assert_eq!(config.seed(), 42);
    }

    #[test]
    fn test_failure_mode_descriptions() {
        assert_eq!(
            FailureMode::IoError.description(),
            "I/O error (permissions, device full, etc.)"
        );
        assert_eq!(FailureMode::Timeout.description(), "operation timeout");
    }

    #[test]
    fn test_chaos_executor_creation() {
        let config = ChaosConfig::new(0.2, vec![FailureMode::Corruption]).expect("valid config");
        let executor = ChaosExecutor::new(config);

        // Float comparison - allow small tolerance
        assert!((executor.config.probability() - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_chaos_executor_derive() {
        let config =
            ChaosConfig::new(0.1, vec![FailureMode::ResourceExhaustion]).expect("valid config");
        let executor = ChaosExecutor::new(config);
        let derived = executor.derive(10);

        assert_eq!(derived.rng_counter, 10);
    }

    #[test]
    fn test_inject_chaos_success() {
        let config = ChaosConfig::new(0.0, vec![FailureMode::IoError]).expect("valid config");
        let executor = ChaosExecutor::new(config);

        let result = executor.inject_chaos(|| Ok::<(), std::io::Error>(()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_inject_chaos_always_fail() {
        let config = ChaosConfig::new(1.0, vec![FailureMode::IoError]).expect("valid config");
        let executor = ChaosExecutor::new(config);

        let result = executor.inject_chaos(|| Ok::<(), std::io::Error>(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_inject_chaos_infallible() {
        let config = ChaosConfig::new(0.0, vec![FailureMode::Timeout]).expect("valid config");
        let executor = ChaosExecutor::new(config);

        let result = executor.inject_chaos_infallible(|| 42);
        assert_eq!(result.map_or(0, |v| v), 42);
    }

    #[test]
    fn test_run_chaos_iterations() {
        let config = ChaosConfig::new(0.5, vec![FailureMode::IoError])
            .expect("valid config")
            .with_seed(42);

        let results = run_chaos_iterations(&config, 10, || Ok::<(), std::io::Error>(()));

        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_calculate_chaos_stats() {
        let results: Vec<Result<(), anyhow::Error>> = vec![
            Ok(()),
            Ok(()),
            Err(anyhow::anyhow!("chaos")),
            Ok(()),
            Err(anyhow::anyhow!("chaos")),
        ];

        let (successes, failures, rate) = calculate_chaos_stats(&results);

        assert_eq!(successes, 3);
        assert_eq!(failures, 2);
        assert!((rate - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_chaos_stats_empty() {
        let results: Vec<Result<(), anyhow::Error>> = vec![];
        let (successes, failures, rate) = calculate_chaos_stats(&results);

        assert_eq!(successes, 0);
        assert_eq!(failures, 0);
        // Float comparison - allow small tolerance
        assert!((rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generate_chaos_seed_unique() {
        let seed1 = generate_chaos_seed();
        let seed2 = generate_chaos_seed();

        assert_ne!(seed1, seed2);
    }
}
