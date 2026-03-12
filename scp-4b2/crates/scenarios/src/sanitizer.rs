//! Feedback sanitizer - implements information barrier
//!
//! Sanitizes scenario execution feedback at different levels:
//! - Level 1: Pass/fail only
//! - Level 2: +error type
//! - Level 3: +stack trace (no values)
//! - Level 4: +assertion locations (no values)
//! - Level 5: Full (development only)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::runner::ScenarioResult;

/// Feedback sanitization levels
///
/// Controls how much detail is exposed about scenario execution
/// to the agent workspace (information barrier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedbackLevel {
    /// Only pass/fail status - no details whatsoever
    Level1,
    /// Pass/fail + error type (e.g., "assertion failed", "network error")
    Level2,
    /// Level 2 + stack trace patterns (but no actual values)
    Level3,
    /// Level 3 + assertion locations (but no values)
    Level4,
    /// Full details - development only
    #[default]
    Level5,
}

impl FeedbackLevel {
    /// Check if this level exposes error types
    #[must_use]
    pub fn exposes_error_type(&self) -> bool {
        matches!(
            self,
            Self::Level2 | Self::Level3 | Self::Level4 | Self::Level5
        )
    }

    /// Check if this level exposes stack traces
    #[must_use]
    pub fn exposes_stack_trace(&self) -> bool {
        matches!(self, Self::Level3 | Self::Level4 | Self::Level5)
    }

    /// Check if this level exposes assertion locations
    #[must_use]
    pub fn exposes_assertion_locations(&self) -> bool {
        matches!(self, Self::Level4 | Self::Level5)
    }

    /// Check if this level exposes full details
    #[must_use]
    pub fn exposes_full_details(&self) -> bool {
        matches!(self, Self::Level5)
    }

    /// Get the level number
    #[must_use]
    pub fn level(&self) -> u8 {
        match self {
            Self::Level1 => 1,
            Self::Level2 => 2,
            Self::Level3 => 3,
            Self::Level4 => 4,
            Self::Level5 => 5,
        }
    }

    /// Parse from a number (1-5)
    #[must_use]
    pub fn from_level(level: u8) -> Option<Self> {
        match level {
            1 => Some(Self::Level1),
            2 => Some(Self::Level2),
            3 => Some(Self::Level3),
            4 => Some(Self::Level4),
            5 => Some(Self::Level5),
            _ => None,
        }
    }
}

/// Feedback sanitizer - removes sensitive information from scenario results
#[derive(Debug, Clone)]
pub struct Sanitizer {
    level: FeedbackLevel,
}

impl Sanitizer {
    /// Create a new sanitizer with the given level
    #[must_use]
    pub fn new(level: FeedbackLevel) -> Self {
        Self { level }
    }

    /// Create a sanitizer with default (full) level
    #[must_use]
    pub fn with_default_level() -> Self {
        Self {
            level: FeedbackLevel::default(),
        }
    }

    /// Set the sanitization level
    pub fn set_level(&mut self, level: FeedbackLevel) {
        self.level = level;
    }

    /// Get current level
    #[must_use]
    pub fn level(&self) -> FeedbackLevel {
        self.level
    }

    /// Sanitize a scenario result and return safe feedback string
    #[must_use]
    pub fn sanitize_result(&self, result: &ScenarioResult) -> String {
        match self.level {
            FeedbackLevel::Level1 => Self::sanitize_level1(result),
            FeedbackLevel::Level2 => Self::sanitize_level2(result),
            FeedbackLevel::Level3 => Self::sanitize_level3(result),
            FeedbackLevel::Level4 => Self::sanitize_level4(result),
            FeedbackLevel::Level5 => Self::sanitize_level5(result),
        }
    }

    /// Level 1: Pass/fail only
    fn sanitize_level1(result: &ScenarioResult) -> String {
        if result.passed {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        }
    }

    /// Level 2: +error type
    fn sanitize_level2(result: &ScenarioResult) -> String {
        if result.passed {
            return "PASS".to_string();
        }

        // Find first failure and extract error type
        let error_type = result
            .step_results
            .iter()
            .find(|r| !r.passed)
            .and_then(|r| r.error.as_ref())
            .map_or_else(
                || "unknown error".to_string(),
                |e| Self::extract_error_type(e),
            );

        format!("FAIL: {error_type}")
    }

    /// Level 3: +stack trace (no values)
    fn sanitize_level3(result: &ScenarioResult) -> String {
        if result.passed {
            return "PASS".to_string();
        }

        let mut output = vec!["FAIL".to_string()];

        for step_result in &result.step_results {
            if !step_result.passed {
                if let Some(error) = &step_result.error {
                    let error_type = Self::extract_error_type(error);
                    let stack_trace = Self::sanitize_value(error);
                    output.push(format!(
                        "  Step {} ({}) - {}",
                        step_result.step_index, step_result.step_type, error_type
                    ));
                    output.push(format!("    at <{stack_trace}>"));
                }
            }
        }

        output.join("\n")
    }

    /// Level 4: +assertion locations (no values)
    fn sanitize_level4(result: &ScenarioResult) -> String {
        if result.passed {
            return "PASS".to_string();
        }

        let mut output = vec!["FAIL".to_string()];

        for step_result in &result.step_results {
            if !step_result.passed {
                if let Some(error) = &step_result.error {
                    let error_type = Self::extract_error_type(error);
                    output.push(format!(
                        "  Step {} ({}) - {}",
                        step_result.step_index, step_result.step_type, error_type
                    ));
                    output.push(format!("    Assertion at step {}", step_result.step_index));
                }
            }
        }

        output.join("\n")
    }

    /// Level 5: Full (development only)
    fn sanitize_level5(result: &ScenarioResult) -> String {
        let mut output = vec![format!("Scenario: {}", result.scenario_name)];

        if result.passed {
            output.push("PASS".to_string());
        } else {
            output.push("FAIL".to_string());

            for step_result in &result.step_results {
                let status = if step_result.passed { "OK" } else { "FAIL" };
                output.push(format!(
                    "  Step {} ({}): {}",
                    step_result.step_index, step_result.step_type, status
                ));

                if let Some(error) = &step_result.error {
                    output.push(format!("    Error: {error}"));
                }
            }
        }

        output.join("\n")
    }

    /// Extract just the error type from an error message
    fn extract_error_type(error: &str) -> String {
        let lower = error.to_lowercase();

        if lower.contains("assertion") || lower.contains("assert") {
            "assertion failed".to_string()
        } else if lower.contains("network") || lower.contains("connection") {
            "network error".to_string()
        } else if lower.contains("timeout") {
            "timeout".to_string()
        } else if lower.contains("parse") || lower.contains("serializ") {
            "parse error".to_string()
        } else if lower.contains("extract") {
            "extraction error".to_string()
        } else {
            "execution error".to_string()
        }
    }

    /// Sanitize a value - removes potentially sensitive data
    fn sanitize_value(value: &str) -> String {
        // Remove anything that looks like a value (strings, numbers in specific patterns)
        // Keep structural elements like "Step", "assert", "equals"
        let sanitized = value
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '@' || c == '.' || c == ':' {
                    '*'
                } else {
                    c
                }
            })
            .collect();

        sanitized
    }

    /// Check if an agent would be blocked from reading scenarios at this level
    #[must_use]
    pub fn blocks_scenario_access(&self) -> bool {
        // Information barrier is always in effect
        true
    }
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::with_default_level()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::StepResult;

    fn create_test_result() -> ScenarioResult {
        ScenarioResult {
            scenario_name: "Test Scenario".to_string(),
            passed: false,
            step_results: vec![
                StepResult {
                    step_index: 0,
                    step_type: "http".to_string(),
                    passed: true,
                    error: None,
                },
                StepResult {
                    step_index: 1,
                    step_type: "extract".to_string(),
                    passed: true,
                    error: None,
                },
                StepResult {
                    step_index: 2,
                    step_type: "assert".to_string(),
                    passed: false,
                    error: Some(
                        "Assertion failed: expected 'test-123' but got 'wrong-value'".to_string(),
                    ),
                },
            ],
        }
    }

    #[test]
    fn test_level1_only_pass_fail() {
        let sanitizer = Sanitizer::new(FeedbackLevel::Level1);
        let result = create_test_result();

        let output = sanitizer.sanitize_result(&result);
        assert_eq!(output, "FAIL");
    }

    #[test]
    fn test_level1_passing_scenario() {
        let sanitizer = Sanitizer::new(FeedbackLevel::Level1);
        let result = ScenarioResult {
            scenario_name: "Test".to_string(),
            passed: true,
            step_results: vec![],
        };

        let output = sanitizer.sanitize_result(&result);
        assert_eq!(output, "PASS");
    }

    #[test]
    fn test_level2_error_type() {
        let sanitizer = Sanitizer::new(FeedbackLevel::Level2);
        let result = create_test_result();

        let output = sanitizer.sanitize_result(&result);
        assert!(output.contains("FAIL"));
        assert!(output.contains("assertion failed"));
    }

    #[test]
    fn test_level5_full_details() {
        let sanitizer = Sanitizer::new(FeedbackLevel::Level5);
        let result = create_test_result();

        let output = sanitizer.sanitize_result(&result);
        assert!(output.contains("Test Scenario"));
        assert!(output.contains("FAIL"));
        assert!(output.contains("Step 2"));
    }

    #[test]
    fn test_extract_error_type() {
        let _sanitizer = Sanitizer::new(FeedbackLevel::Level1);

        assert_eq!(
            Sanitizer::extract_error_type("Assertion failed: expected 'test'"),
            "assertion failed"
        );
        assert_eq!(
            Sanitizer::extract_error_type("Network connection refused"),
            "network error"
        );
        assert_eq!(
            Sanitizer::extract_error_type("Request timeout after 30s"),
            "timeout"
        );
    }

    #[test]
    fn test_feedback_level_from_number() {
        assert_eq!(FeedbackLevel::from_level(1), Some(FeedbackLevel::Level1));
        assert_eq!(FeedbackLevel::from_level(3), Some(FeedbackLevel::Level3));
        assert_eq!(FeedbackLevel::from_level(5), Some(FeedbackLevel::Level5));
        assert_eq!(FeedbackLevel::from_level(6), None);
    }

    #[test]
    fn test_blocks_scenario_access() {
        let sanitizer = Sanitizer::new(FeedbackLevel::Level1);
        assert!(sanitizer.blocks_scenario_access());
    }
}
