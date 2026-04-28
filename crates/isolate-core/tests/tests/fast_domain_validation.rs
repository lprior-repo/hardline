//! Fast in-process domain validation tests.
//!
//! This module tests domain validation rules using pure functions without:
//! - Subprocess spawning (no `Command::new`)
//! - File I/O
//! - Network I/O
//!
//! # Performance Comparison
//!
//! | Test Type | Typical Time |
//! |-----------|--------------|
//! | Subprocess validation test | 100-500ms |
//! | Pure validation test (this file) | <1ms |
//!
//! This is a 100-500x speedup for validation tests.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use isolate_core::validation::domain::{
    validate_agent_id, validate_bead_id, validate_session_name,
};

// =============================================================================
// Session Name Validation Tests
// =============================================================================

mod session_name_validation {
    use super::*;

    /// GIVEN: A valid session name
    /// WHEN: Validating
    /// THEN: Validation passes
    #[test]
    fn valid_session_names() {
        // Simple names
        assert!(validate_session_name("my-session").is_ok());
        assert!(validate_session_name("my_session").is_ok());
        assert!(validate_session_name("session123").is_ok());
        assert!(validate_session_name("Session").is_ok());
        assert!(validate_session_name("ABC").is_ok());

        // Names with underscores
        assert!(validate_session_name("feature_branch").is_ok());
        assert!(validate_session_name("bug_fix_123").is_ok());

        // Names with hyphens
        assert!(validate_session_name("feature-branch").is_ok());
        assert!(validate_session_name("hotfix-urgent").is_ok());

        // Single letter (minimum valid)
        assert!(validate_session_name("a").is_ok());

        // Max length (63 chars)
        let max_name = "a".repeat(63);
        assert!(validate_session_name(&max_name).is_ok());
    }

    /// GIVEN: An empty session name
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn empty_session_name_rejected() {
        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("   ").is_err());
    }

    /// GIVEN: A session name that's too long
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn too_long_session_name_rejected() {
        let long_name = "a".repeat(64);
        assert!(validate_session_name(&long_name).is_err());
    }

    /// GIVEN: A session name not starting with a letter
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn numeric_start_rejected() {
        assert!(validate_session_name("123-session").is_err());
        assert!(validate_session_name("-session").is_err());
        assert!(validate_session_name("_session").is_err());
    }

    /// GIVEN: A session name with invalid characters
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn invalid_characters_rejected() {
        // Dots not allowed
        assert!(validate_session_name("my.session").is_err());

        // Slashes not allowed
        assert!(validate_session_name("my/session").is_err());

        // Spaces not allowed
        assert!(validate_session_name("my session").is_err());

        // Special chars not allowed
        assert!(validate_session_name("my@session").is_err());
        assert!(validate_session_name("my#session").is_err());
        assert!(validate_session_name("my!session").is_err());
    }

    /// GIVEN: A session name with leading/trailing whitespace
    /// WHEN: Validating
    /// THEN: Whitespace is trimmed and validation passes
    #[test]
    fn whitespace_is_trimmed() {
        assert!(validate_session_name("  my-session  ").is_ok());
        assert!(validate_session_name("\tmy-session\t").is_ok());
        assert!(validate_session_name("\nmy-session\n").is_ok());
    }
}

// =============================================================================
// Agent ID Validation Tests
// =============================================================================

mod agent_id_validation {
    use super::*;

    /// GIVEN: A valid agent ID
    /// WHEN: Validating
    /// THEN: Validation passes
    #[test]
    fn valid_agent_ids() {
        // Simple IDs
        assert!(validate_agent_id("agent-123").is_ok());
        assert!(validate_agent_id("agent_456").is_ok());
        assert!(validate_agent_id("agent789").is_ok());

        // IDs with colons (for host:port style)
        assert!(validate_agent_id("agent:123").is_ok());
        assert!(validate_agent_id("host:8080").is_ok());

        // IDs with dots (for domain style)
        assert!(validate_agent_id("agent.example").is_ok());
        assert!(validate_agent_id("agent.example.com").is_ok());

        // Complex combinations
        assert!(validate_agent_id("agent-123.example:8080").is_ok());

        // Single character
        assert!(validate_agent_id("a").is_ok());

        // Max length (128 chars)
        let max_id = "a".repeat(128);
        assert!(validate_agent_id(&max_id).is_ok());
    }

    /// GIVEN: An empty agent ID
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn empty_agent_id_rejected() {
        assert!(validate_agent_id("").is_err());
    }

    /// GIVEN: An agent ID that's too long
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn too_long_agent_id_rejected() {
        let long_id = "a".repeat(129);
        assert!(validate_agent_id(&long_id).is_err());
    }

    /// GIVEN: An agent ID with invalid characters
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn invalid_characters_in_agent_id_rejected() {
        // Slashes
        assert!(validate_agent_id("agent/123").is_err());

        // Spaces
        assert!(validate_agent_id("agent 123").is_err());

        // Special chars
        assert!(validate_agent_id("agent@123").is_err());
        assert!(validate_agent_id("agent#123").is_err());
    }
}

// =============================================================================
// Bead ID Validation Tests
// =============================================================================

mod bead_id_validation {
    use super::*;

    /// GIVEN: A valid bead ID
    /// WHEN: Validating
    /// THEN: Validation passes
    #[test]
    fn valid_bead_ids() {
        // Standard format: bd- followed by hex characters
        assert!(validate_bead_id("bd-abc123").is_ok());
        assert!(validate_bead_id("bd-ABC123DEF456").is_ok());
        assert!(validate_bead_id("bd-1234567890abcdef").is_ok());
        assert!(validate_bead_id("bd-a").is_ok());
    }

    /// GIVEN: An invalid bead ID
    /// WHEN: Validating
    /// THEN: Validation fails
    #[test]
    fn invalid_bead_ids_rejected() {
        // Missing prefix
        assert!(validate_bead_id("abc123").is_err());

        // Empty
        assert!(validate_bead_id("").is_err());

        // Wrong prefix
        assert!(validate_bead_id("bead-abc123").is_err());

        // Non-hex after prefix
        assert!(validate_bead_id("bd-xyz").is_err());
        assert!(validate_bead_id("bd-feature").is_err());
        assert!(validate_bead_id("bd-").is_err());
    }
}

// =============================================================================
// Composed Validation Tests
// =============================================================================

mod composed_validation {
    use super::*;

    /// GIVEN: Multiple identifiers to validate
    /// WHEN: Using composed validation
    /// THEN: All must pass for success
    #[test]
    fn composed_validation_all_must_pass() {
        let session = "my-session";
        let agent = "agent-123";

        // Both valid
        let result = validate_session_name(session).and_then(|()| validate_agent_id(agent));
        assert!(result.is_ok());
    }

    /// GIVEN: One invalid identifier in composition
    /// WHEN: Using composed validation
    /// THEN: Fails on first error
    #[test]
    fn composed_validation_fails_on_first_error() {
        let session = "123-invalid"; // Invalid start
        let agent = "agent-123";

        let result = validate_session_name(session).and_then(|()| validate_agent_id(agent));
        assert!(result.is_err());
    }

    /// GIVEN: Multiple validations to run
    /// WHEN: Using iterator pattern
    /// THEN: Can collect all errors or success
    #[test]
    fn validation_iterator_pattern() {
        let inputs: [(&str, bool); 5] = [
            ("valid-session", true),
            ("invalid-session!", false),
            ("another-valid", true),
            ("", false),
            ("123-start", false),
        ];

        let results: Vec<_> = inputs
            .iter()
            .map(|(name, _expected)| (*name, validate_session_name(name).is_ok()))
            .collect();

        for ((name, expected), (_, actual)) in inputs.iter().zip(results.iter()) {
            assert_eq!(
                *expected, *actual,
                "Validation result mismatch for '{name}'"
            );
        }
    }
}

// =============================================================================
// Property-Based Validation Tests (manual)
// =============================================================================

mod property_tests {
    use super::*;

    /// Property: Any valid session name can be round-tripped through validation
    #[test]
    fn valid_names_are_idempotent() {
        let valid_names: [&str; 13] = [
            "a",
            "ab",
            "abc",
            "session",
            "my-session",
            "my_session",
            "Session123",
            "feature-branch-name",
            "bug_fix_123",
            "A",
            "ABC",
            "a1",
            "a-1",
        ];

        for name in valid_names {
            // First validation
            let first = validate_session_name(name);
            // Second validation (should be identical)
            let second = validate_session_name(name);

            assert_eq!(
                first.is_ok(),
                second.is_ok(),
                "Idempotency failed for '{name}'"
            );
        }
    }

    /// Property: Invalid names are consistently rejected
    #[test]
    fn invalid_names_always_rejected() {
        let invalid_names: [&str; 10] = [
            "",
            "   ",
            "123",
            "-start",
            "_start",
            "has space",
            "has.dot",
            "has/slash",
            "has@at",
            "#hash",
        ];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(result.is_err(), "Expected '{name}' to be rejected");
        }
    }

    /// Property: Length boundaries are exact
    #[test]
    fn length_boundaries_are_exact() {
        // Exactly 63 chars should pass
        let exactly_max = "a".repeat(63);
        assert!(validate_session_name(&exactly_max).is_ok());

        // 64 chars should fail
        let too_long = "a".repeat(64);
        assert!(validate_session_name(&too_long).is_err());

        // 1 char should pass
        assert!(validate_session_name("a").is_ok());

        // 0 chars should fail
        assert!(validate_session_name("").is_err());
    }
}

// =============================================================================
// Performance Benchmark
// =============================================================================

#[cfg(test)]
mod benchmarks {
    use std::time::Instant;

    use super::*;

    /// Benchmark: How fast can we validate 10,000 session names?
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn benchmark_session_name_validation() {
        let iterations = 10_000;
        let test_names: Vec<String> = (0..iterations).map(|i| format!("session-{i}")).collect();

        let start = Instant::now();

        let valid_count = test_names
            .iter()
            .filter(|name| validate_session_name(name).is_ok())
            .count();

        let elapsed = start.elapsed();

        println!(
            "Validation: {} operations in {:?} ({:.2} ops/ms)",
            iterations,
            elapsed,
            iterations as f64 / elapsed.as_millis().max(1) as f64
        );

        assert_eq!(valid_count, iterations);
        // Should complete in under 100ms
        assert!(elapsed.as_millis() < 100);
    }
}
