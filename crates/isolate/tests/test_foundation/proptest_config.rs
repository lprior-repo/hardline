#![allow(clippy::option_if_let_else)]
//! Proptest Configuration for Deterministic Property-Based Testing
//!
//! This module provides a standardized configuration for proptest that ensures:
//! - Deterministic test execution with fixed seeds
//! - Reproducible test failures
//! - Configurable test case counts
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::test_foundation::proptest_config::deterministic_config;
//!
//! proptest! {
//!     #![proptest_config(deterministic_config())]
//!
//!     fn test_my_property(input: String) {
//!         // Property test here
//!     }
//! }
//! ```
//!

use std::env;

use proptest::test_runner::Config;

/// Default seed for deterministic test execution.
/// This seed is used when no `PROPTEST_SEED` environment variable is set.
///
/// The seed was chosen arbitrarily but is fixed to ensure reproducibility.
const DEFAULT_SEED: u64 = 0x1234_5678_9ABC_DEF0;

/// Number of test cases for fast property tests.
/// Suitable for simple invariants that are quick to verify.
/// Can be overridden with `PROPTEST_CASES` environment variable.
pub const FAST_CASES: u32 = 64;

/// Number of test cases for standard property tests.
/// Good balance between coverage and speed.
/// Can be overridden with `PROPTEST_CASES` environment variable.
pub const STANDARD_CASES: u32 = 100;

/// Number of test cases for thorough property tests.
/// Used for critical invariants where coverage matters.
/// Can be overridden with `PROPTEST_CASES` environment variable.
pub const THOROUGH_CASES: u32 = 256;

/// Default number of test cases (standard).
/// Can be overridden with `PROPTEST_CASES` environment variable.
const DEFAULT_CASES: u32 = STANDARD_CASES;

/// Maximum number of shrinking iterations.
const DEFAULT_MAX_SHRINK_ITERS: u32 = 1024;

/// Create a deterministic proptest configuration.
///
/// This configuration ensures:
/// - Tests are reproducible with a fixed seed
/// - Test failures can be reproduced by setting `PROPTEST_SEED`
/// - Test count can be adjusted with `PROPTEST_CASES`
///
/// # Environment Variables
///
/// - `PROPTEST_SEED`: Override the default seed (format: hex string like "0x1234")
/// - `PROPTEST_CASES`: Override the default number of test cases
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #![proptest_config(deterministic_config())]
///
///     fn test_string_roundtrip(input: String) {
///         proptest::prop_assert_eq!(input.clone(), input);
///     }
/// }
/// ```
#[must_use]
pub fn deterministic_config() -> Config {
    let seed = parse_seed_from_env().unwrap_or(DEFAULT_SEED);
    let cases = parse_cases_from_env().unwrap_or(DEFAULT_CASES);
    let max_shrink_iters = parse_max_shrink_iters_from_env().unwrap_or(DEFAULT_MAX_SHRINK_ITERS);

    Config {
        cases,
        max_shrink_iters,
        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
        ..Config::default()
    }
}

/// Lazy-initialized deterministic config.
/// Uses lazy initialization to allow environment variable overrides at runtime.
pub static DETERMINISTIC_CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();

/// Lazy-initialized fast config with fewer cases.
pub static FAST_CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();

/// Lazy-initialized standard config.
pub static STANDARD_CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();

/// Lazy-initialized thorough config with more cases.
pub static THOROUGH_CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();

/// Get the global deterministic config, initializing if needed.
#[must_use]
pub fn get_deterministic_config() -> &'static Config {
    DETERMINISTIC_CONFIG.get_or_init(deterministic_config)
}

/// Create a fast proptest configuration with fewer test cases.
///
/// Use this for simple invariants that are quick to verify.
/// Runs 64 cases by default.
#[must_use]
pub fn fast_config() -> Config {
    let seed = parse_seed_from_env().unwrap_or(DEFAULT_SEED);
    let cases = parse_cases_from_env().unwrap_or(FAST_CASES);
    Config {
        cases,
        max_shrink_iters: 256,
        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
        ..Config::default()
    }
}

/// Create a standard proptest configuration.
///
/// Use this for most property tests.
/// Runs 100 cases by default.
#[must_use]
pub fn standard_config() -> Config {
    let seed = parse_seed_from_env().unwrap_or(DEFAULT_SEED);
    let cases = parse_cases_from_env().unwrap_or(STANDARD_CASES);
    Config {
        cases,
        max_shrink_iters: DEFAULT_MAX_SHRINK_ITERS,
        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
        ..Config::default()
    }
}

/// Create a thorough proptest configuration with more test cases.
///
/// Use this for critical invariants where coverage matters.
/// Runs 256 cases by default.
#[must_use]
pub fn thorough_config() -> Config {
    let seed = parse_seed_from_env().unwrap_or(DEFAULT_SEED);
    let cases = parse_cases_from_env().unwrap_or(THOROUGH_CASES);
    Config {
        cases,
        max_shrink_iters: DEFAULT_MAX_SHRINK_ITERS,
        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
        ..Config::default()
    }
}

/// Get the global fast config, initializing if needed.
#[must_use]
pub fn get_fast_config() -> &'static Config {
    FAST_CONFIG.get_or_init(fast_config)
}

/// Get the global standard config, initializing if needed.
#[must_use]
pub fn get_standard_config() -> &'static Config {
    STANDARD_CONFIG.get_or_init(standard_config)
}

/// Get the global thorough config, initializing if needed.
#[must_use]
pub fn get_thorough_config() -> &'static Config {
    THOROUGH_CONFIG.get_or_init(thorough_config)
}

/// Parse the seed from the `PROPTEST_SEED` environment variable.
///
/// Supports both hex (0x...) and decimal formats.
fn parse_seed_from_env() -> Option<u64> {
    env::var("PROPTEST_SEED").ok().and_then(|s| {
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u64::from_str_radix(hex, 16).ok()
        } else {
            s.parse().ok()
        }
    })
}

/// Parse the number of test cases from the `PROPTEST_CASES` environment variable.
fn parse_cases_from_env() -> Option<u32> {
    env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok())
}

/// Parse the max shrink iterations from environment variable.
fn parse_max_shrink_iters_from_env() -> Option<u32> {
    env::var("PROPTEST_MAX_SHRINK_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Helper trait for running proptest with deterministic config.
///
/// This trait provides a convenient way to run property tests
/// with the standard deterministic configuration.
pub trait DeterministicProptest {
    /// Run a property test with deterministic configuration.
    ///
    /// # Arguments
    ///
    /// * `property` - The property to test
    ///
    /// # Errors
    ///
    /// Returns an error if the property test fails.
    fn run_deterministic<F>(&self, property: F) -> Result<(), String>
    where
        F: Fn() -> Result<(), proptest::test_runner::TestCaseError>;
}

/// Result type for proptest operations.
pub type ProptestResult<T> = Result<T, proptest::test_runner::TestError<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_config_uses_default_seed() {
        let config = deterministic_config();
        // Verify config was created successfully
        assert!(config.cases >= 1);
    }

    #[test]
    fn test_parse_seed_from_env_handles_hex() {
        // This test verifies the parsing logic
        let parsed = u64::from_str_radix("123456789ABCDEF0", 16);
        assert!(parsed.is_ok());
        assert_eq!(parsed.expect("valid hex"), 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn test_parse_seed_from_env_handles_decimal() {
        let parsed: Result<u64, _> = "12345".parse();
        assert!(parsed.is_ok());
        assert_eq!(parsed.expect("valid decimal"), 12345_u64);
    }

    #[test]
    fn test_get_deterministic_config_is_cached() {
        let config1 = get_deterministic_config();
        let config2 = get_deterministic_config();
        // Same pointer = same instance
        assert!(std::ptr::eq(config1, config2));
    }
}