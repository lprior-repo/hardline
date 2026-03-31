//! Partial configuration for explicit-key merge semantics

use serde::{Deserialize, Serialize};

use super::types::{ConflictMode, ValidatedBool};

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
    pub log_resolutions: Option<ValidatedBool>,
}

use crate::Result;

impl super::config::ConflictResolutionConfig {
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
    pub fn merge_partial(&mut self, partial: PartialConflictResolutionConfig) -> Result<()> {
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
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialSessionConfig {
    #[serde(default)]
    pub auto_commit: Option<ValidatedBool>,
    #[serde(default)]
    pub commit_prefix: Option<String>,
    #[serde(default)]
    pub max_sessions: Option<usize>,
}

impl super::config::SessionConfig {
    pub fn merge_partial(&mut self, partial: PartialSessionConfig) -> Result<()> {
        if let Some(auto_commit) = partial.auto_commit {
            self.auto_commit = auto_commit;
        }
        if let Some(commit_prefix) = partial.commit_prefix {
            self.commit_prefix = commit_prefix;
        }
        if let Some(max_sessions) = partial.max_sessions {
            self.max_sessions = max_sessions;
        }
        Ok(())
    }
}
