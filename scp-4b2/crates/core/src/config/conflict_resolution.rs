#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution configuration
//!
//! This module provides configuration for conflict resolution behavior,
//! including mode selection (auto/manual/hybrid), autonomy levels,
//! security keyword detection, and audit logging.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// CONFLICT MODE ENUM
// ═══════════════════════════════════════════════════════════════════════════

/// Conflict resolution mode
///
/// Defines how conflicts are resolved:
/// - **Auto**: Fully automatic resolution by AI
/// - **Manual**: All conflicts require human intervention
/// - **Hybrid**: AI auto-resolves safe conflicts based on autonomy and keywords
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictMode {
    /// Fully automatic resolution
    ///
    /// All conflicts are resolved by AI without human intervention.
    /// Use with caution - recommended only for CI environments with tests.
    Auto,

    /// Fully manual resolution
    ///
    /// All conflicts require human intervention. AI may suggest resolutions,
    /// but humans must approve them.
    #[default]
    Manual,

    /// Hybrid mode
    ///
    /// AI auto-resolves safe conflicts based on autonomy level and security keywords.
    /// Risky conflicts (those matching security keywords) require human review.
    Hybrid,
}

impl std::fmt::Display for ConflictMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Manual => write!(f, "manual"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl FromStr for ConflictMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid conflict mode: {s}. Must be one of: auto, manual, hybrid"
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFLICT RESOLUTION CONFIG
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for conflict resolution behavior
///
/// This configuration controls how conflicts are detected, analyzed,
/// and resolved across different environments (development, CI, production).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictResolutionConfig {
    /// Resolution mode: auto, manual, or hybrid
    pub mode: ConflictMode,

    /// Autonomy level for AI-controlled resolution (0-100)
    ///
    /// - 0: Fully manual - all conflicts require human approval
    /// - 1-49: Conservative AI - suggest resolutions, require approval
    /// - 50-89: Balanced AI - auto-resolve safe conflicts, prompt for risky ones
    /// - 90-99: Aggressive AI - auto-resolve most conflicts, prompt only for security
    /// - 100: Fully autonomous - AI resolves all conflicts without prompting
    pub autonomy: u8,

    /// Security keywords that trigger human review
    ///
    /// Conflicts in files containing these keywords require human approval
    /// regardless of autonomy level. Examples: "password", "token", "secret",
    /// "`api_key`", "`private_key`", "credential", etc.
    pub security_keywords: Vec<String>,

    /// Whether to log all conflict resolutions to audit trail
    ///
    /// When true, all resolutions (auto and manual) are logged to the
    /// `conflict_resolutions` table with timestamp, agent, file, strategy, and reason.
    pub log_resolutions: bool,
}

impl ConflictResolutionConfig {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns `Error::ValidationError` if:
    /// - autonomy is not in range 0-100
    /// - `security_keywords` is empty
    /// - mode is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate autonomy range
        if self.autonomy > 100 {
            return Err(Error::ValidationError {
                message: format!("autonomy must be 0-100, got {}", self.autonomy),
                field: Some("autonomy".to_string()),
                value: Some(self.autonomy.to_string()),
                constraints: vec!["0 <= autonomy <= 100".to_string()],
            });
        }

        // Validate security_keywords is non-empty
        if self.security_keywords.is_empty() {
            return Err(Error::ValidationError {
                message: "security_keywords must not be empty".to_string(),
                field: Some("security_keywords".to_string()),
                value: None,
                constraints: vec!["security_keywords.len() > 0".to_string()],
            });
        }

        // Validate mode (already enforced by enum, but check for completeness)
        match self.mode {
            ConflictMode::Auto | ConflictMode::Manual | ConflictMode::Hybrid => Ok(()),
        }
    }

    /// Check if a file requires human review based on security keywords
    ///
    /// Returns true if the file path contains any security keyword.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use isolate_core::config::conflict_resolution::{ConflictMode, ConflictResolutionConfig};
    ///
    /// let config = ConflictResolutionConfig {
    ///     mode: ConflictMode::Manual,
    ///     autonomy: 0,
    ///     security_keywords: vec!["password".to_string()],
    ///     log_resolutions: true,
    /// };
    ///
    /// assert!(config.requires_human_review("src/auth/password.rs"));
    /// assert!(!config.requires_human_review("src/lib.rs"));
    /// ```
    #[must_use]
    pub fn requires_human_review(&self, file_path: &str) -> bool {
        let file_path_lower = file_path.to_lowercase();
        self.security_keywords
            .iter()
            .any(|keyword| file_path_lower.contains(&keyword.to_lowercase()))
    }

    /// Check if resolution can proceed automatically based on autonomy
    ///
    /// Returns true if:
    /// - mode is Auto, OR
    /// - mode is Hybrid AND autonomy >= threshold AND file doesn't contain security keywords
    ///
    /// # Examples
    ///
    /// ```rust
    /// use isolate_core::config::conflict_resolution::{ConflictMode, ConflictResolutionConfig};
    ///
    /// // Manual mode never auto-resolves
    /// let manual = ConflictResolutionConfig {
    ///     mode: ConflictMode::Manual,
    ///     autonomy: 100,
    ///     security_keywords: vec!["password".to_string()],
    ///     log_resolutions: true,
    /// };
    /// assert!(!manual.can_auto_resolve(None));
    ///
    /// // Auto mode always allows
    /// let auto = ConflictResolutionConfig {
    ///     mode: ConflictMode::Auto,
    ///     autonomy: 0,
    ///     security_keywords: vec!["password".to_string()],
    ///     log_resolutions: true,
    /// };
    /// assert!(auto.can_auto_resolve(None));
    /// ```
    #[must_use]
    pub fn can_auto_resolve(&self, file_path: Option<&str>) -> bool {
        match self.mode {
            ConflictMode::Auto => true,
            ConflictMode::Manual => false,
            ConflictMode::Hybrid => file_path.map_or(self.autonomy >= 50, |path| {
                !self.requires_human_review(path) && self.autonomy >= 50
            }),
        }
    }
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            mode: ConflictMode::Manual, // Safest default
            autonomy: 0,                // Fully manual by default
            security_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "credential".to_string(),
            ],
            log_resolutions: true, // Audit trail is important
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARTIAL CONFIG STRUCTURES (explicit-key merge semantics)
// ═══════════════════════════════════════════════════════════════════════════

/// Partial configuration with Option<T> fields for explicit-key merge semantics
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialConflictResolutionConfig {
    #[serde(default)]
    pub mode: Option<ConflictMode>,
    #[serde(default)]
    pub autonomy: Option<u8>,
    #[serde(default)]
    pub security_keywords: Option<Vec<String>>,
    #[serde(default)]
    pub log_resolutions: Option<bool>,
}

impl ConflictResolutionConfig {
    /// Merge partial config, only updating fields that are Some(value)
    ///
    /// This method implements explicit-key merge semantics: only fields
    /// that are Some(value) in the partial config will override the
    /// corresponding fields in self. Fields that are None will NOT
    /// reset the values in self.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use isolate_core::config::conflict_resolution::{
    ///     ConflictMode, ConflictResolutionConfig, PartialConflictResolutionConfig,
    /// };
    ///
    /// let mut config = ConflictResolutionConfig::default();
    /// let original_autonomy = config.autonomy;
    ///
    /// // Merge partial config that only sets mode
    /// let partial = PartialConflictResolutionConfig {
    ///     mode: Some(ConflictMode::Hybrid),
    ///     autonomy: None,
    ///     security_keywords: None,
    ///     log_resolutions: None,
    /// };
    ///
    /// config.merge_partial(partial);
    ///
    /// assert_eq!(config.mode, ConflictMode::Hybrid);
    /// assert_eq!(config.autonomy, original_autonomy); // Preserved
    /// ```
    pub fn merge_partial(&mut self, partial: PartialConflictResolutionConfig) {
        if let Some(mode) = partial.mode {
            self.mode = mode;
        }
        if let Some(autonomy) = partial.autonomy {
            self.autonomy = autonomy;
        }
        if let Some(security_keywords) = partial.security_keywords {
            self.security_keywords = security_keywords;
        }
        if let Some(log_resolutions) = partial.log_resolutions {
            self.log_resolutions = log_resolutions;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::redundant_clone)]

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // HAPPY PATH TESTS (HP-001 to HP-025)
    // ═══════════════════════════════════════════════════════════════════════════

    mod happy_path {
        use super::*;

        #[test]
        fn hp_001_default_config_is_safe() {
            let config = ConflictResolutionConfig::default();

            assert_eq!(config.mode, ConflictMode::Manual);
            assert_eq!(config.autonomy, 0);
            assert!(config.security_keywords.len() >= 5);
            assert!(config.log_resolutions);
        }

        #[test]
        fn hp_002_valid_config_passes_validation() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 50,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
        }

        #[test]
        fn hp_003_manual_mode_never_auto_resolves() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 100,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.can_auto_resolve(None));
            assert!(!config.can_auto_resolve(Some("safe_file.rs")));
        }

        #[test]
        fn hp_004_auto_mode_allows_auto_resolution() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 0,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(None));
            assert!(config.can_auto_resolve(Some("any_file.rs")));
        }

        #[test]
        fn hp_005_hybrid_mode_respects_autonomy_threshold() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 60,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(Some("safe_file.rs")));
        }

        #[test]
        fn hp_006_hybrid_mode_blocks_below_threshold() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 30,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.can_auto_resolve(Some("safe_file.rs")));
        }

        #[test]
        fn hp_007_security_keywords_trigger_review() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/auth/password.rs"));
        }

        #[test]
        fn hp_008_security_keywords_case_insensitive() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: vec!["PASSWORD".to_string()],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/auth/password.txt"));
            assert!(config.requires_human_review("src/auth/PASSWORD.txt"));
            assert!(config.requires_human_review("src/auth/PassWord.txt"));
        }

        #[test]
        fn hp_009_multiple_keywords_all_trigger_review() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec![
                    "password".to_string(),
                    "token".to_string(),
                    "secret".to_string(),
                ],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/auth/password.rs"));
            assert!(config.requires_human_review("src/api/token.rs"));
            assert!(config.requires_human_review("src/config/secret.rs"));
        }

        #[test]
        fn hp_010_security_overrides_autonomy() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.can_auto_resolve(Some("src/auth/password.rs")));
        }

        #[test]
        fn hp_011_autonomy_minimum_valid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
        }

        #[test]
        fn hp_012_autonomy_maximum_valid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 100,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
        }

        #[test]
        fn hp_013_all_modes_valid() {
            let keywords = vec!["key".to_string()];

            let auto = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 80,
                security_keywords: keywords.clone(),
                log_resolutions: true,
            };

            let manual = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: keywords.clone(),
                log_resolutions: true,
            };

            let hybrid = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 50,
                security_keywords: keywords.clone(),
                log_resolutions: true,
            };

            assert!(auto.validate().is_ok());
            assert!(manual.validate().is_ok());
            assert!(hybrid.validate().is_ok());
        }

        #[test]
        fn hp_014_log_resolutions_true() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(config.log_resolutions);
        }

        #[test]
        fn hp_015_log_resolutions_false() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 100,
                security_keywords: vec!["key".to_string()],
                log_resolutions: false,
            };

            assert!(!config.log_resolutions);
        }

        #[test]
        fn hp_021_mode_display_correct() {
            assert_eq!(ConflictMode::Auto.to_string(), "auto");
            assert_eq!(ConflictMode::Manual.to_string(), "manual");
            assert_eq!(ConflictMode::Hybrid.to_string(), "hybrid");
        }

        #[test]
        fn hp_022_mode_fromstr_correct() {
            assert_eq!(
                ConflictMode::from_str("auto").ok(),
                Some(ConflictMode::Auto)
            );
            assert_eq!(
                ConflictMode::from_str("manual").ok(),
                Some(ConflictMode::Manual)
            );
            assert_eq!(
                ConflictMode::from_str("hybrid").ok(),
                Some(ConflictMode::Hybrid)
            );
        }

        #[test]
        fn hp_023_mode_fromstr_case_insensitive() {
            assert_eq!(
                ConflictMode::from_str("AUTO").ok(),
                Some(ConflictMode::Auto)
            );
            assert_eq!(
                ConflictMode::from_str("Manual").ok(),
                Some(ConflictMode::Manual)
            );
            assert_eq!(
                ConflictMode::from_str("HyBrId").ok(),
                Some(ConflictMode::Hybrid)
            );
        }

        #[test]
        fn hp_024_mode_fromstr_invalid() {
            let result = ConflictMode::from_str("invalid_mode");
            assert!(result.is_err());
        }

        #[test]
        fn hp_025_production_config_valid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 70,
                security_keywords: vec![
                    "password".to_string(),
                    "token".to_string(),
                    "secret".to_string(),
                    "api_key".to_string(),
                    "private_key".to_string(),
                    "credential".to_string(),
                ],
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EDGE CASE TESTS (EC-001 to EC-025)
    // ═══════════════════════════════════════════════════════════════════════════

    mod edge_cases {
        use super::*;

        #[test]
        fn ec_001_autonomy_below_minimum_invalid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 255, // Would be -1 if signed
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(config.validate().is_err());
        }

        #[test]
        fn ec_002_autonomy_above_maximum_invalid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 101,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            let result = config.validate();
            assert!(result.is_err());

            if let Err(Error::ValidationError { field, .. }) = result {
                assert_eq!(field, Some("autonomy".to_string()));
            } else {
                panic!("Expected ValidationError with autonomy field");
            }
        }

        #[test]
        fn ec_003_empty_keywords_invalid() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: vec![],
                log_resolutions: true,
            };

            let result = config.validate();
            assert!(result.is_err());

            if let Err(Error::ValidationError { field, .. }) = result {
                assert_eq!(field, Some("security_keywords".to_string()));
            } else {
                panic!("Expected ValidationError with security_keywords field");
            }
        }

        #[test]
        fn ec_004_safe_file_no_review() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.requires_human_review("README.md"));
            assert!(!config.requires_human_review("src/lib.rs"));
            assert!(!config.requires_human_review("tests/test.rs"));
        }

        #[test]
        fn ec_005_keyword_substring_triggers_review() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/config/api_key.rs"));
            assert!(config.requires_human_review("src/auth/private_key.rs"));
            assert!(config.requires_human_review("src/encryption/secret_key.rs"));
        }

        #[test]
        fn ec_006_hybrid_autonomy_at_threshold() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 50,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(Some("safe_file.rs")));
        }

        #[test]
        fn ec_007_hybrid_autonomy_below_threshold() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 49,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.can_auto_resolve(Some("safe_file.rs")));
        }

        #[test]
        fn ec_008_auto_resolve_no_file_path() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 60,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(None));
        }

        #[test]
        fn ec_009_manual_mode_no_file_path() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 100,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(!config.can_auto_resolve(None));
        }

        #[test]
        fn ec_010_auto_mode_no_file_path() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 0,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(None));
        }

        #[test]
        fn ec_011_partial_mode_only() {
            let mut base = ConflictResolutionConfig::default();
            let original_autonomy = base.autonomy;
            let original_keywords = base.security_keywords.clone();
            let original_log = base.log_resolutions;

            let partial = PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Auto),
                autonomy: None,
                security_keywords: None,
                log_resolutions: None,
            };

            base.merge_partial(partial);

            assert_eq!(base.mode, ConflictMode::Auto);
            assert_eq!(base.autonomy, original_autonomy);
            assert_eq!(base.security_keywords, original_keywords);
            assert_eq!(base.log_resolutions, original_log);
        }

        #[test]
        fn ec_012_partial_autonomy_only() {
            let mut base = ConflictResolutionConfig::default();
            let original_mode = base.mode;

            let partial = PartialConflictResolutionConfig {
                mode: None,
                autonomy: Some(80),
                security_keywords: None,
                log_resolutions: None,
            };

            base.merge_partial(partial);

            assert_eq!(base.mode, original_mode);
            assert_eq!(base.autonomy, 80);
        }

        #[test]
        fn ec_013_partial_keywords_only() {
            let mut base = ConflictResolutionConfig::default();
            let original_mode = base.mode;
            let original_autonomy = base.autonomy;

            let new_keywords = vec!["token".to_string()];
            let partial = PartialConflictResolutionConfig {
                mode: None,
                autonomy: None,
                security_keywords: Some(new_keywords.clone()),
                log_resolutions: None,
            };

            base.merge_partial(partial);

            assert_eq!(base.mode, original_mode);
            assert_eq!(base.autonomy, original_autonomy);
            assert_eq!(base.security_keywords, new_keywords);
        }

        #[test]
        fn ec_014_partial_log_only() {
            let mut base = ConflictResolutionConfig::default();
            let original_mode = base.mode;

            let partial = PartialConflictResolutionConfig {
                mode: None,
                autonomy: None,
                security_keywords: None,
                log_resolutions: Some(false),
            };

            base.merge_partial(partial);

            assert_eq!(base.mode, original_mode);
            assert!(!base.log_resolutions);
        }

        #[test]
        fn ec_015_partial_all_fields() {
            let mut base = ConflictResolutionConfig::default();

            let partial = PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Auto),
                autonomy: Some(90),
                security_keywords: Some(vec!["test".to_string()]),
                log_resolutions: Some(false),
            };

            base.merge_partial(partial);

            assert_eq!(base.mode, ConflictMode::Auto);
            assert_eq!(base.autonomy, 90);
            assert_eq!(base.security_keywords, vec!["test".to_string()]);
            assert!(!base.log_resolutions);
        }

        #[test]
        fn ec_016_empty_partial_no_changes() {
            let mut base = ConflictResolutionConfig::default();
            let original = base.clone();

            let partial = PartialConflictResolutionConfig::default();
            base.merge_partial(partial);

            assert_eq!(base, original);
        }

        #[test]
        fn ec_017_multiple_partial_merges() {
            let mut base = ConflictResolutionConfig::default();

            let partial1 = PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Hybrid),
                autonomy: None,
                security_keywords: None,
                log_resolutions: None,
            };

            let partial2 = PartialConflictResolutionConfig {
                mode: None,
                autonomy: Some(70),
                security_keywords: None,
                log_resolutions: None,
            };

            base.merge_partial(partial1);
            base.merge_partial(partial2);

            assert_eq!(base.mode, ConflictMode::Hybrid);
            assert_eq!(base.autonomy, 70);
        }

        #[test]
        fn ec_018_later_merge_overrides() {
            let mut base = ConflictResolutionConfig::default();

            let partial1 = PartialConflictResolutionConfig {
                mode: None,
                autonomy: Some(50),
                security_keywords: None,
                log_resolutions: None,
            };

            let partial2 = PartialConflictResolutionConfig {
                mode: None,
                autonomy: Some(80),
                security_keywords: None,
                log_resolutions: None,
            };

            base.merge_partial(partial1);
            base.merge_partial(partial2);

            assert_eq!(base.autonomy, 80);
        }

        #[test]
        fn ec_024_keyword_special_chars() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec!["api-key".to_string()],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/config/api-key.rs"));
        }

        #[test]
        fn ec_025_many_keywords_works() {
            let keywords: Vec<String> = (0..100).map(|i| format!("keyword_{i}")).collect();

            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 80,
                security_keywords: keywords.clone(),
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
            assert!(config.requires_human_review("keyword_50.rs"));
            assert!(!config.requires_human_review("safe.rs"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRACT VERIFICATION TESTS (CV-001 to CV-015)
    // ═══════════════════════════════════════════════════════════════════════════

    mod contract_verification {
        use super::*;

        #[test]
        fn cv_001_pre_conf_001_autonomy_range() {
            for autonomy in [0, 1, 50, 99, 100] {
                let config = ConflictResolutionConfig {
                    mode: ConflictMode::Manual,
                    autonomy,
                    security_keywords: vec!["key".to_string()],
                    log_resolutions: true,
                };
                assert!(
                    config.validate().is_ok(),
                    "autonomy={autonomy} should be valid"
                );
            }
        }

        #[test]
        fn cv_002_pre_conf_002_keywords_nonempty() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 0,
                security_keywords: vec!["test".to_string()],
                log_resolutions: true,
            };

            assert!(config.validate().is_ok());
        }

        #[test]
        fn cv_003_pre_conf_003_mode_valid() {
            for mode in [
                ConflictMode::Auto,
                ConflictMode::Manual,
                ConflictMode::Hybrid,
            ] {
                let config = ConflictResolutionConfig {
                    mode,
                    autonomy: 50,
                    security_keywords: vec!["key".to_string()],
                    log_resolutions: true,
                };
                assert!(config.validate().is_ok(), "mode={mode:?} should be valid");
            }
        }

        #[test]
        fn cv_005_post_load_002_sensible_defaults() {
            let config = ConflictResolutionConfig::default();

            assert_eq!(config.mode, ConflictMode::Manual);
            assert_eq!(config.autonomy, 0);
            assert!(config.security_keywords.len() >= 5);
            assert!(config.log_resolutions);
        }

        #[test]
        fn cv_006_inv_data_001_autonomy_invariant() {
            let configs = vec![
                ConflictResolutionConfig::default(),
                ConflictResolutionConfig {
                    mode: ConflictMode::Auto,
                    autonomy: 100,
                    security_keywords: vec!["key".to_string()],
                    log_resolutions: true,
                },
            ];

            for config in configs {
                assert!(config.autonomy <= 100, "autonomy invariant violated");
            }
        }

        #[test]
        fn cv_007_inv_data_002_keywords_invariant() {
            let config = ConflictResolutionConfig::default();
            assert!(!config.security_keywords.is_empty());
        }

        #[test]
        fn cv_008_inv_sec_001_security_overrides_autonomy() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 100,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("src/auth/password.rs"));
            assert!(!config.can_auto_resolve(Some("src/auth/password.rs")));
        }

        #[test]
        fn cv_009_inv_sec_002_manual_never_auto() {
            for autonomy in [0, 50, 100] {
                let config = ConflictResolutionConfig {
                    mode: ConflictMode::Manual,
                    autonomy,
                    security_keywords: vec!["key".to_string()],
                    log_resolutions: true,
                };

                assert!(!config.can_auto_resolve(None));
                assert!(!config.can_auto_resolve(Some("any_file.rs")));
            }
        }

        #[test]
        fn cv_010_inv_sec_003_auto_always_allows() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 0,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(config.can_auto_resolve(None));
            assert!(config.can_auto_resolve(Some("any_file.rs")));
        }

        #[test]
        fn cv_011_inv_conf_001_hierarchy_precedence() {
            let mut config = ConflictResolutionConfig::default();
            assert_eq!(config.mode, ConflictMode::Manual);

            // Apply "global" override
            config.merge_partial(PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Hybrid),
                autonomy: Some(50),
                security_keywords: None,
                log_resolutions: None,
            });
            assert_eq!(config.mode, ConflictMode::Hybrid);
            assert_eq!(config.autonomy, 50);

            // Apply "project" override
            config.merge_partial(PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Auto),
                autonomy: None,
                security_keywords: None,
                log_resolutions: None,
            });
            assert_eq!(config.mode, ConflictMode::Auto);
            assert_eq!(config.autonomy, 50); // Preserved from global
        }

        #[test]
        fn cv_012_inv_conf_002_partial_preserves_fields() {
            let mut base = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 60,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            base.merge_partial(PartialConflictResolutionConfig {
                mode: Some(ConflictMode::Auto),
                autonomy: None,
                security_keywords: None,
                log_resolutions: None,
            });

            assert_eq!(base.mode, ConflictMode::Auto);
            assert_eq!(base.autonomy, 60);
            assert_eq!(base.security_keywords, vec!["password".to_string()]);
            assert!(base.log_resolutions);
        }

        #[test]
        fn cv_013_post_res_001_mode_respected() {
            let auto = ConflictResolutionConfig {
                mode: ConflictMode::Auto,
                autonomy: 0,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            let manual = ConflictResolutionConfig {
                mode: ConflictMode::Manual,
                autonomy: 100,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            let hybrid = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 60,
                security_keywords: vec!["key".to_string()],
                log_resolutions: true,
            };

            assert!(auto.can_auto_resolve(None));
            assert!(!manual.can_auto_resolve(None));
            assert!(hybrid.can_auto_resolve(None));
        }

        #[test]
        fn cv_014_post_res_002_all_keywords_checked() {
            let config = ConflictResolutionConfig {
                mode: ConflictMode::Hybrid,
                autonomy: 90,
                security_keywords: vec![
                    "password".to_string(),
                    "token".to_string(),
                    "secret".to_string(),
                ],
                log_resolutions: true,
            };

            assert!(config.requires_human_review("password.rs"));
            assert!(config.requires_human_review("token.rs"));
            assert!(config.requires_human_review("secret.rs"));
        }

        #[test]
        fn cv_015_post_res_003_security_always_review() {
            for mode in [
                ConflictMode::Auto,
                ConflictMode::Manual,
                ConflictMode::Hybrid,
            ] {
                for autonomy in [0, 50, 100] {
                    let config = ConflictResolutionConfig {
                        mode,
                        autonomy,
                        security_keywords: vec!["password".to_string()],
                        log_resolutions: true,
                    };

                    assert!(
                        config.requires_human_review("src/auth/password.rs"),
                        "mode={mode:?}, autonomy={autonomy} should require review"
                    );
                }
            }
        }
    }
}
