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
    /// use scp_core::fix::Fix;
    ///
    /// let fix = Fix::safe(
    ///     "Use different name",
    ///     vec!["scp add isolate-test-2".to_string()],
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
    /// use scp_core::fix::Fix;
    ///
    /// let fix = Fix::risky(
    ///     "Remove existing session",
    ///     vec!["scp remove test".to_string()],
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
    /// use scp_core::fix::Fix;
    ///
    /// let fix = Fix::destructive(
    ///     "Force delete all data",
    ///     vec!["rm -rf .scp".to_string()],
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
            return Err(Error::validation_error(
                "Fix must have at least one command".to_string(),
            ));
        }

        if self.automatic && !matches!(self.impact, FixImpact::Safe | FixImpact::Low) {
            return Err(Error::validation_error(
                "Automatic fixes must be Safe or Low impact".to_string(),
            ));
        }

        Ok(())
    }
}

/// Error wrapper that includes structured fix suggestions.
///
/// This allows errors to carry machine-readable fix information
/// without breaking existing Error API.
#[derive(Debug)]
pub struct ErrorWithFixes {
    /// The underlying error
    pub error: Box<Error>,
    /// Structured fixes (ordered by safety, safest first)
    pub fixes: Vec<Fix>,
}

impl ErrorWithFixes {
    /// Create an error with a single fix.
    #[must_use]
    pub fn new(error: Error, fix: Fix) -> Self {
        Self {
            error: Box::new(error),
            fixes: vec![fix],
        }
    }

    /// Create an error with multiple fixes.
    ///
    /// Fixes should be ordered from safest to most risky.
    #[must_use]
    pub fn with_fixes(error: Error, fixes: Vec<Fix>) -> Self {
        Self {
            error: Box::new(error),
            fixes,
        }
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
            vec!["scp add isolate-test-2".to_string()],
        );

        assert_eq!(fix.description, "Use different name");
        assert_eq!(fix.commands, vec!["scp add isolate-test-2"]);
        assert!(fix.automatic);
        assert_eq!(fix.impact, FixImpact::Safe);
        assert!(fix.rationale.is_none());
    }

    #[test]
    fn test_risky_fix_creation() {
        let fix = Fix::risky(
            "Remove existing session",
            vec!["scp remove test".to_string()],
            "Will delete existing session and all its data",
        );

        assert_eq!(fix.description, "Remove existing session");
        assert_eq!(fix.commands, vec!["scp remove test"]);
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
            vec!["rm -rf .scp".to_string()],
            "WARNING: This will delete all session data irreversibly",
        );

        assert_eq!(fix.description, "Force delete all data");
        assert_eq!(fix.commands, vec!["rm -rf .scp"]);
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
        let error = Error::validation_error("Session 'test' already exists".to_string());
        let fix = Fix::safe("Use different name", vec!["scp add test2".to_string()]);
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
        let error = Error::validation_error("Session 'test' already exists".to_string());
        let fixes = vec![
            Fix::safe("Use different name", vec!["scp add test2".to_string()]),
            Fix::risky(
                "Remove existing",
                vec!["scp remove test".to_string()],
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
        let error = Error::validation_error("Test error".to_string());
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
        let error = Error::validation_error("Test error".to_string());
        let fixes = vec![
            Fix::safe("Auto 1", vec!["cmd1".to_string()]),
            Fix::risky("Manual", vec!["cmd2".to_string()], "Risky"),
            Fix::safe("Auto 2", vec!["cmd3".to_string()]),
        ];
        let error_with_fixes = ErrorWithFixes::with_fixes(error, fixes);

        let auto_count = error_with_fixes.automatic_fixes().count();
        assert_eq!(auto_count, 2);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FIX IMPACT ENUM VARIANTS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_fix_impact_all_variants_construct() {
        let _safe = FixImpact::Safe;
        let _low = FixImpact::Low;
        let _medium = FixImpact::Medium;
        let _high = FixImpact::High;
        let _destructive = FixImpact::Destructive;
    }

    #[test]
    fn test_fix_impact_debug() {
        assert!(format!("{:?}", FixImpact::Safe).contains("Safe"));
        assert!(format!("{:?}", FixImpact::Low).contains("Low"));
        assert!(format!("{:?}", FixImpact::Medium).contains("Medium"));
        assert!(format!("{:?}", FixImpact::High).contains("High"));
        assert!(format!("{:?}", FixImpact::Destructive).contains("Destructive"));
    }

    #[test]
    fn test_fix_impact_clone_and_copy() {
        let impact = FixImpact::High;
        let copied = impact;
        let cloned = impact.clone();
        assert_eq!(impact, copied);
        assert_eq!(impact, cloned);
    }

    #[test]
    fn test_fix_impact_equality() {
        assert_eq!(FixImpact::Safe, FixImpact::Safe);
        assert_eq!(FixImpact::Destructive, FixImpact::Destructive);
        assert_ne!(FixImpact::Safe, FixImpact::Destructive);
        assert_ne!(FixImpact::Low, FixImpact::Medium);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FIX STRUCT FIELD ACCESS
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_fix_safe_has_low_impact_not_destructive() {
        let fix = Fix::safe("desc", vec!["cmd".to_string()]);
        assert_eq!(fix.impact, FixImpact::Safe);
        assert_ne!(fix.impact, FixImpact::Destructive);
        assert_ne!(fix.impact, FixImpact::High);
    }

    #[test]
    fn test_fix_risky_medium_impact() {
        let fix = Fix::risky("desc", vec!["cmd".to_string()], "rationale");
        assert_eq!(fix.impact, FixImpact::Medium);
    }

    #[test]
    fn test_fix_destructive_impact() {
        let fix = Fix::destructive("desc", vec!["cmd".to_string()], "warning");
        assert_eq!(fix.impact, FixImpact::Destructive);
    }

    #[test]
    fn test_fix_description_field() {
        let fix = Fix::safe("Rename the session", vec!["mv old new".to_string()]);
        assert_eq!(fix.description, "Rename the session");
    }

    #[test]
    fn test_fix_commands_field() {
        let cmds = vec![
            "git stash".to_string(),
            "git pull".to_string(),
            "git stash pop".to_string(),
        ];
        let fix = Fix::safe("Sync changes", cmds.clone());
        assert_eq!(fix.commands.len(), 3);
        assert_eq!(fix.commands, cmds);
    }

    #[test]
    fn test_fix_automatic_field_safe() {
        let fix = Fix::safe("desc", vec!["cmd".to_string()]);
        assert!(fix.automatic);
    }

    #[test]
    fn test_fix_automatic_field_risky() {
        let fix = Fix::risky("desc", vec!["cmd".to_string()], "r");
        assert!(!fix.automatic);
    }

    #[test]
    fn test_fix_automatic_field_destructive() {
        let fix = Fix::destructive("desc", vec!["cmd".to_string()], "w");
        assert!(!fix.automatic);
    }

    #[test]
    fn test_fix_rationale_none_for_safe() {
        let fix = Fix::safe("desc", vec!["cmd".to_string()]);
        assert!(fix.rationale.is_none());
    }

    #[test]
    fn test_fix_rationale_some_for_risky() {
        let fix = Fix::risky("desc", vec!["cmd".to_string()], "because reasons");
        assert!(fix.rationale.is_some());
        assert_eq!(fix.rationale.as_deref(), Some("because reasons"));
    }

    #[test]
    fn test_fix_rationale_some_for_destructive() {
        let fix = Fix::destructive("desc", vec!["cmd".to_string()], "data loss warning");
        assert!(fix.rationale.is_some());
        assert_eq!(fix.rationale.as_deref(), Some("data loss warning"));
    }

    #[test]
    fn test_fix_clone() {
        let fix = Fix::safe("desc", vec!["cmd".to_string()]);
        let cloned = fix.clone();
        assert_eq!(fix, cloned);
    }

    #[test]
    fn test_fix_equality() {
        let f1 = Fix::safe("same", vec!["cmd".to_string()]);
        let f2 = Fix::safe("same", vec!["cmd".to_string()]);
        let f3 = Fix::safe("different", vec!["cmd".to_string()]);
        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_fix_debug() {
        let fix = Fix::safe("Test description", vec!["echo hello".to_string()]);
        let debug_str = format!("{fix:?}");
        assert!(debug_str.contains("Fix"));
        assert!(debug_str.contains("Test description"));
    }

    #[test]
    fn test_fix_validate_medium_impact_not_automatic_ok() {
        let fix = Fix {
            description: "Manual medium fix".to_string(),
            commands: vec!["some command".to_string()],
            automatic: false,
            impact: FixImpact::Medium,
            rationale: None,
        };
        assert!(fix.validate().is_ok());
    }

    #[test]
    fn test_fix_validate_high_impact_not_automatic_ok() {
        let fix = Fix {
            description: "Manual high fix".to_string(),
            commands: vec!["dangerous cmd".to_string()],
            automatic: false,
            impact: FixImpact::High,
            rationale: None,
        };
        assert!(fix.validate().is_ok());
    }

    #[test]
    fn test_fix_validate_destructive_not_automatic_ok() {
        let fix = Fix::destructive(
            "Delete everything",
            vec!["rm -rf /".to_string()],
            "irreversible",
        );
        assert!(fix.validate().is_ok());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ERROR WITH FIXES DISPLAY & ERROR TRAIT
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_error_with_fixes_display() {
        let error = Error::validation_error("Something went wrong".to_string());
        let fix = Fix::safe("Do X", vec!["x".to_string()]);
        let ewf = ErrorWithFixes::new(error, fix);

        let display = format!("{ewf}");
        assert!(display.contains("Something went wrong"));
    }

    #[test]
    fn test_error_with_fixes_debug() {
        let error = Error::validation_error("err".to_string());
        let fix = Fix::safe("f", vec!["c".to_string()]);
        let ewf = ErrorWithFixes::new(error, fix);

        let debug_str = format!("{ewf:?}");
        assert!(debug_str.contains("ErrorWithFixes"));
    }

    #[test]
    fn test_error_with_fixes_empty_fixes_list() {
        let error = Error::validation_error("err".to_string());
        let ewf = ErrorWithFixes::with_fixes(error, vec![]);
        assert!(ewf.fixes().is_empty());
        assert!(ewf.first_automatic_fix().is_none());
        assert_eq!(ewf.automatic_fixes().count(), 0);
    }

    #[test]
    fn test_error_with_fixes_source_is_underlying_error() {
        let error = Error::validation_error("base error".to_string());
        let fix = Fix::safe("f", vec!["c".to_string()]);
        let ewf = ErrorWithFixes::new(error, fix);

        let source = std::error::Error::source(&ewf);
        assert!(source.is_some());
    }

    #[test]
    fn test_error_with_fixes_first_automatic_none_when_all_manual() {
        let error = Error::validation_error("err".to_string());
        let fixes = vec![
            Fix::risky("manual 1", vec!["a".to_string()], "r1"),
            Fix::destructive("manual 2", vec!["b".to_string()], "r2"),
        ];
        let ewf = ErrorWithFixes::with_fixes(error, fixes);

        assert!(ewf.first_automatic_fix().is_none());
    }

    #[test]
    fn test_error_with_fixes_first_automatic_returns_first_safe() {
        let error = Error::validation_error("err".to_string());
        let fixes = vec![
            Fix::safe("first auto", vec!["a".to_string()]),
            Fix::safe("second auto", vec!["b".to_string()]),
            Fix::risky("manual", vec!["c".to_string()], "r"),
        ];
        let ewf = ErrorWithFixes::with_fixes(error, fixes);

        let first_auto = ewf.first_automatic_fix();
        assert!(first_auto.is_some());
        assert_eq!(
            first_auto.map(|f| &f.description),
            Some(&"first auto".to_string())
        );
    }

    // ── Serde roundtrip tests ──────────────────────────────────────────────────

    #[test]
    fn test_fix_impact_serde_roundtrip_all_variants() {
        for impact in [
            FixImpact::Safe,
            FixImpact::Low,
            FixImpact::Medium,
            FixImpact::High,
            FixImpact::Destructive,
        ] {
            let json = serde_json::to_string(&impact).expect("serialize ok");
            let deserialized: FixImpact = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(impact, deserialized, "Roundtrip failed for {impact:?}");
        }
    }

    #[test]
    fn test_fix_impact_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&FixImpact::Safe).expect("ok"),
            "\"safe\""
        );
        assert_eq!(
            serde_json::to_string(&FixImpact::Low).expect("ok"),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&FixImpact::Medium).expect("ok"),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&FixImpact::High).expect("ok"),
            "\"high\""
        );
        assert_eq!(
            serde_json::to_string(&FixImpact::Destructive).expect("ok"),
            "\"destructive\""
        );
    }

    #[test]
    fn test_fix_serde_roundtrip_with_options() {
        let fix = Fix::safe("desc", vec!["cmd1".to_string(), "cmd2".to_string()]);
        let json = serde_json::to_string(&fix).expect("serialize ok");
        let deserialized: Fix = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(fix, deserialized);
    }

    #[test]
    fn test_fix_serde_roundtrip_with_rationale() {
        let fix = Fix::risky("desc", vec!["cmd".to_string()], "rationale text");
        let json = serde_json::to_string(&fix).expect("serialize ok");
        let deserialized: Fix = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(fix, deserialized);
        assert!(json.contains("rationale text"));
    }

    #[test]
    fn test_fix_serde_with_empty_commands_vec() {
        let fix = Fix {
            description: "empty".to_string(),
            commands: vec![],
            automatic: false,
            impact: FixImpact::Safe,
            rationale: None,
        };
        let json = serde_json::to_string(&fix).expect("serialize ok");
        let deserialized: Fix = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(fix, deserialized);
        assert!(deserialized.commands.is_empty());
    }
}
