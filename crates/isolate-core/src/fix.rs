#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

//! Structured fix suggestions for errors.
//!
//! This module provides machine-readable fix information that AI agents
//! can use to automatically resolve errors.

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Impact level of a fix operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixImpact {
    /// No side effects, always reversible
    Safe,
    /// Minimal risk, easy to undo
    Low,
    /// Some risk, manual undo possible
    Medium,
    /// Significant risk, difficult to undo
    High,
    /// Data loss, irreversible
    Destructive,
}

/// A structured fix for an error, providing actionable commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    /// Human-readable description of what this fix does
    pub description: String,
    /// Shell commands to execute (in order)
    pub commands: Vec<String>,
    /// Can this fix be applied automatically without user confirmation?
    pub automatic: bool,
    /// Risk/impact level of this fix
    pub impact: FixImpact,
    /// Optional rationale of why this fix works or what it does
    #[serde(rename = "rationale")]
    pub rationale: Option<String>,
}

impl Fix {
    /// Create a safe fix that can be applied automatically.
    ///
    /// # Examples
    ///
    /// ```
    /// use isolate_core::fix::Fix;
    ///
    /// let fix = Fix::safe(
    ///     "Use different name",
    ///     vec!["isolate add isolate-test-2".to_string()],
    /// );
    /// assert!(fix.automatic);
    /// ```
    #[must_use]
    pub fn safe(description: impl Into<String>, commands: Vec<String>) -> Self {
        Self {
            description: description.into(),
            commands,
            automatic: true,
            impact: FixImpact::Safe,
            rationale: None,
        }
    }

    /// Create a risky fix that requires manual confirmation.
    ///
    /// # Examples
    ///
    /// ```
    /// use isolate_core::fix::Fix;
    ///
    /// let fix = Fix::risky(
    ///     "Remove existing session",
    ///     vec!["isolate remove test".to_string()],
    ///     "Will delete existing session and all its data",
    /// );
    /// assert!(!fix.automatic);
    /// ```
    #[must_use]
    pub fn risky(
        description: impl Into<String>,
        commands: Vec<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            commands,
            automatic: false,
            impact: FixImpact::Medium,
            rationale: Some(rationale.into()),
        }
    }

    /// Create a destructive fix with high warning level.
    ///
    /// # Examples
    ///
    /// ```
    /// use isolate_core::fix::Fix;
    ///
    /// let fix = Fix::destructive(
    ///     "Force delete all data",
    ///     vec!["rm -rf .isolate".to_string()],
    ///     "WARNING: This will delete all session data irreversibly",
    /// );
    /// assert!(!fix.automatic);
    /// ```
    #[must_use]
    pub fn destructive(
        description: impl Into<String>,
        commands: Vec<String>,
        warning: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            commands,
            automatic: false,
            impact: FixImpact::Destructive,
            rationale: Some(warning.into()),
        }
    }

    /// Validate that this fix is well-formed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Commands list is empty
    /// - Automatic fix has non-safe impact level
    pub fn validate(&self) -> Result<(), Error> {
        if self.commands.is_empty() {
            return Err(Error::ValidationError {
                message: "Fix must have at least one command".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        if self.automatic && !matches!(self.impact, FixImpact::Safe | FixImpact::Low) {
            return Err(Error::ValidationError {
                message: "Automatic fixes must be Safe or Low impact".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        Ok(())
    }
}

/// Error wrapper that includes structured fix suggestions.
///
/// This allows errors to carry machine-readable fix information
/// without breaking existing Error API.
#[derive(Debug, Clone)]
pub struct ErrorWithFixes {
    /// The underlying error
    pub error: Error,
    /// Structured fixes (ordered by safety, safest first)
    pub fixes: Vec<Fix>,
}

impl ErrorWithFixes {
    /// Create an error with a single fix.
    #[must_use]
    pub fn new(error: Error, fix: Fix) -> Self {
        Self {
            error,
            fixes: vec![fix],
        }
    }

    /// Create an error with multiple fixes.
    ///
    /// Fixes should be ordered from safest to most risky.
    #[must_use]
    pub const fn with_fixes(error: Error, fixes: Vec<Fix>) -> Self {
        Self { error, fixes }
    }

    /// Get all fixes for this error.
    #[must_use]
    pub fn fixes(&self) -> &[Fix] {
        &self.fixes
    }

    /// Get the first automatic fix, if one exists.
    #[must_use]
    pub fn first_automatic_fix(&self) -> Option<&Fix> {
        self.fixes.iter().find(|fix| fix.automatic)
    }

    /// Get all automatic fixes.
    pub fn automatic_fixes(&self) -> impl Iterator<Item = &Fix> {
        self.fixes.iter().filter(|fix| fix.automatic)
    }
}

impl std::fmt::Display for ErrorWithFixes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for ErrorWithFixes {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_fix_creation() {
        let fix = Fix::safe(
            "Use different name",
            vec!["isolate add isolate-test-2".to_string()],
        );

        assert_eq!(fix.description, "Use different name");
        assert_eq!(fix.commands, vec!["isolate add isolate-test-2"]);
        assert!(fix.automatic);
        assert_eq!(fix.impact, FixImpact::Safe);
        assert!(fix.rationale.is_none());
    }

    #[test]
    fn test_risky_fix_creation() {
        let fix = Fix::risky(
            "Remove existing session",
            vec!["isolate remove test".to_string()],
            "Will delete existing session and all its data",
        );

        assert_eq!(fix.description, "Remove existing session");
        assert_eq!(fix.commands, vec!["isolate remove test"]);
        assert!(!fix.automatic);
        assert_eq!(fix.impact, FixImpact::Medium);
        assert_eq!(
            fix.rationale,
            Some("Will delete existing session and all its data".to_string())
        );
    }

    #[test]
    fn test_destructive_fix_creation() {
        let fix = Fix::destructive(
            "Force delete all data",
            vec!["rm -rf .isolate".to_string()],
            "WARNING: This will delete all session data irreversibly",
        );

        assert_eq!(fix.description, "Force delete all data");
        assert_eq!(fix.commands, vec!["rm -rf .isolate"]);
        assert!(!fix.automatic);
        assert_eq!(fix.impact, FixImpact::Destructive);
        assert_eq!(
            fix.rationale,
            Some("WARNING: This will delete all session data irreversibly".to_string())
        );
    }

    #[test]
    fn test_fix_validate_empty_commands() {
        let fix = Fix {
            description: "Empty fix".to_string(),
            commands: Vec::new(),
            automatic: false,
            impact: FixImpact::Safe,
            rationale: None,
        };

        let result = fix.validate();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("at least one command"));
        }
    }

    #[test]
    fn test_fix_validate_automatic_must_be_safe() {
        let fix = Fix {
            description: "Dangerous automatic fix".to_string(),
            commands: vec!["rm -rf /".to_string()],
            automatic: true,
            impact: FixImpact::Destructive,
            rationale: None,
        };

        let result = fix.validate();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Safe or Low impact"));
        }
    }

    #[test]
    fn test_fix_validate_low_impact_can_be_automatic() {
        let fix = Fix {
            description: "Low risk fix".to_string(),
            commands: vec!["echo test".to_string()],
            automatic: true,
            impact: FixImpact::Low,
            rationale: None,
        };

        assert!(fix.validate().is_ok());
    }

    #[test]
    fn test_error_with_fixes_creation() {
        let error = Error::ValidationError {
            message: "Session 'test' already exists".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let fix = Fix::safe("Use different name", vec!["isolate add test2".to_string()]);
        let error_with_fixes = ErrorWithFixes::new(error, fix);

        assert_eq!(error_with_fixes.fixes().len(), 1);

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(
                error_with_fixes.fixes()[0].description,
                "Use different name"
            );
        }
    }

    #[test]
    fn test_error_with_multiple_fixes() {
        let error = Error::ValidationError {
            message: "Session 'test' already exists".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let fixes = vec![
            Fix::safe("Use different name", vec!["isolate add test2".to_string()]),
            Fix::risky(
                "Remove existing",
                vec!["isolate remove test".to_string()],
                "Will delete session",
            ),
        ];
        let error_with_fixes = ErrorWithFixes::with_fixes(error, fixes);

        assert_eq!(error_with_fixes.fixes().len(), 2);

        #[allow(clippy::indexing_slicing)]
        {
            assert!(error_with_fixes.fixes()[0].automatic);
            assert!(!error_with_fixes.fixes()[1].automatic);
        }
    }

    #[test]
    fn test_first_automatic_fix() {
        let error = Error::ValidationError {
            message: "Test error".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let fixes = vec![
            Fix::risky(
                "Manual fix",
                vec!["manual".to_string()],
                "Requires confirmation",
            ),
            Fix::safe("Auto fix", vec!["auto".to_string()]),
        ];
        let error_with_fixes = ErrorWithFixes::with_fixes(error, fixes);

        let auto_fix = error_with_fixes.first_automatic_fix();
        assert!(auto_fix.is_some());
        if let Some(fix) = auto_fix {
            assert_eq!(fix.description, "Auto fix");
        }
    }

    #[test]
    fn test_automatic_fixes_iterator() {
        let error = Error::ValidationError {
            message: "Test error".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let fixes = vec![
            Fix::safe("Auto 1", vec!["cmd1".to_string()]),
            Fix::risky("Manual", vec!["cmd2".to_string()], "Risky"),
            Fix::safe("Auto 2", vec!["cmd3".to_string()]),
        ];
        let error_with_fixes = ErrorWithFixes::with_fixes(error, fixes);

        let auto_count = error_with_fixes.automatic_fixes().count();
        assert_eq!(auto_count, 2);
    }
}
