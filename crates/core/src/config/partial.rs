//! Partial configuration for explicit-key merge semantics

use std::collections::HashMap;

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
    /// Merge partial config, only updating fields that are Some(value).
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

// ═══════════════════════════════════════════════════════════════════════════
// PartialAgentConfig
// ═══════════════════════════════════════════════════════════════════════════

/// Partial agent configuration for explicit-key merge semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialAgentConfig {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

impl super::config::AgentConfig {
    /// Merge partial config, only updating fields that are Some(value).
    pub fn merge_partial(&mut self, partial: PartialAgentConfig) -> Result<()> {
        if let Some(command) = partial.command {
            self.command = command;
        }
        if let Some(env) = partial.env {
            self.env = env;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PartialHooksConfig
// ═══════════════════════════════════════════════════════════════════════════

/// Partial configuration for explicit-key merge semantics (hooks).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialHooksConfig {
    #[serde(default)]
    pub post_create: Option<Vec<String>>,
    #[serde(default)]
    pub pre_remove: Option<Vec<String>>,
    #[serde(default)]
    pub post_merge: Option<Vec<String>>,
}

impl super::config::HooksConfig {
    /// Merge partial config, only updating fields that are Some(value).
    pub fn merge_partial(&mut self, partial: PartialHooksConfig) -> Result<()> {
        if let Some(post_create) = partial.post_create {
            self.post_create = post_create;
        }
        if let Some(pre_remove) = partial.pre_remove {
            self.pre_remove = pre_remove;
        }
        if let Some(post_merge) = partial.post_merge {
            self.post_merge = post_merge;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::config::config::{
        AgentConfig, ConflictResolutionConfig, HooksConfig, SessionConfig,
    };
    use crate::config::types::{ConflictMode, ValidatedBool};

    // ═══════════════════════════════════════════════════════════════════════════
    // PartialConflictResolutionConfig tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn partial_conflict_resolution_default_is_all_none() {
        let partial = PartialConflictResolutionConfig::default();
        assert!(partial.mode.is_none());
        assert!(partial.autonomy.is_none());
        assert!(partial.security_keywords.is_none());
        assert!(partial.log_resolutions.is_none());
    }

    #[test]
    fn partial_conflict_resolution_merge_updates_mode_only() {
        let mut config = ConflictResolutionConfig::default();
        let original_autonomy = config.autonomy;
        let partial = PartialConflictResolutionConfig {
            mode: Some(ConflictMode::Hybrid),
            autonomy: None,
            security_keywords: None,
            log_resolutions: None,
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.mode, ConflictMode::Hybrid);
        assert_eq!(config.autonomy, original_autonomy);
    }

    #[test]
    fn partial_conflict_resolution_merge_updates_autonomy_only() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            mode: None,
            autonomy: Some(75),
            security_keywords: None,
            log_resolutions: None,
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.autonomy, 75);
        assert_eq!(config.mode, ConflictMode::Manual);
    }

    #[test]
    fn partial_conflict_resolution_merge_updates_security_keywords() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            mode: None,
            autonomy: None,
            security_keywords: Some(vec!["ssh".to_string(), "gpg".to_string()]),
            log_resolutions: None,
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.security_keywords, vec!["ssh", "gpg"]);
    }

    #[test]
    fn partial_conflict_resolution_merge_updates_log_resolutions() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            mode: None,
            autonomy: None,
            security_keywords: None,
            log_resolutions: Some(ValidatedBool::new(false)),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert!(!config.log_resolutions.value());
    }

    #[test]
    fn partial_conflict_resolution_merge_all_none_preserves_all() {
        let config = ConflictResolutionConfig::default();
        let mut config2 = config.clone();
        let partial = PartialConflictResolutionConfig::default();
        config2.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config, config2);
    }

    #[test]
    fn partial_conflict_resolution_merge_all_fields() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            mode: Some(ConflictMode::Auto),
            autonomy: Some(50),
            security_keywords: Some(vec!["secret".to_string()]),
            log_resolutions: Some(ValidatedBool::new(true)),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.mode, ConflictMode::Auto);
        assert_eq!(config.autonomy, 50);
        assert_eq!(config.security_keywords, vec!["secret"]);
        assert!(config.log_resolutions.value());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PartialSessionConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn partial_session_config_default_is_all_none() {
        let partial = PartialSessionConfig::default();
        assert!(partial.auto_commit.is_none());
        assert!(partial.commit_prefix.is_none());
        assert!(partial.max_sessions.is_none());
    }

    #[test]
    fn partial_session_config_merge_updates_commit_prefix() {
        let mut config = SessionConfig::default();
        let partial = PartialSessionConfig {
            auto_commit: None,
            commit_prefix: Some("feat:".to_string()),
            max_sessions: None,
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.commit_prefix, "feat:");
        assert!(!config.auto_commit.value());
        assert_eq!(config.max_sessions, 100);
    }

    #[test]
    fn partial_session_config_merge_updates_max_sessions() {
        let mut config = SessionConfig::default();
        let partial = PartialSessionConfig {
            auto_commit: None,
            commit_prefix: None,
            max_sessions: Some(42),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.max_sessions, 42);
        assert_eq!(config.commit_prefix, "wip:");
    }

    #[test]
    fn partial_session_config_merge_updates_all_fields() {
        let mut config = SessionConfig::default();
        let partial = PartialSessionConfig {
            auto_commit: Some(ValidatedBool::new(true)),
            commit_prefix: Some("fix:".to_string()),
            max_sessions: Some(10),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert!(config.auto_commit.value());
        assert_eq!(config.commit_prefix, "fix:");
        assert_eq!(config.max_sessions, 10);
    }

    #[test]
    fn partial_session_config_merge_none_preserves_all() {
        let config = SessionConfig::with_values(ValidatedBool::new(true), "custom:", 5);
        let mut config2 = config.clone();
        let partial = PartialSessionConfig::default();
        config2.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config, config2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PartialAgentConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn partial_agent_config_default_is_all_none() {
        let partial = PartialAgentConfig::default();
        assert!(partial.command.is_none());
        assert!(partial.env.is_none());
    }

    #[test]
    fn partial_agent_config_merge_updates_both_fields() {
        let mut config = AgentConfig::default();
        let mut env = HashMap::new();
        env.insert("MODEL".to_string(), "opus".to_string());
        let partial = PartialAgentConfig {
            command: Some("claude-opus".to_string()),
            env: Some(env.clone()),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.command, "claude-opus");
        assert_eq!(config.env, env);
    }

    #[test]
    fn partial_agent_config_merge_replaces_existing_env() {
        let mut env_old = HashMap::new();
        env_old.insert("OLD_KEY".to_string(), "old_val".to_string());
        let mut config = AgentConfig::with_values("claude", env_old);

        let mut env_new = HashMap::new();
        env_new.insert("NEW_KEY".to_string(), "new_val".to_string());
        let partial = PartialAgentConfig {
            command: None,
            env: Some(env_new.clone()),
        };
        config.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config.env, env_new);
        assert!(!config.env.contains_key("OLD_KEY"));
    }

    #[test]
    fn partial_agent_config_merge_none_preserves_all() {
        let mut env = HashMap::new();
        env.insert("K".to_string(), "v".to_string());
        let config = AgentConfig::with_values("my-agent", env);
        let mut config2 = config.clone();
        let partial = PartialAgentConfig::default();
        config2.merge_partial(partial).expect("merge should succeed");
        assert_eq!(config, config2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PartialHooksConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn partial_hooks_config_merge_updates_post_create_and_post_merge() {
        let mut hooks = HooksConfig::with_values(
            vec!["old_create".to_string()],
            vec!["old_remove".to_string()],
            vec!["old_merge".to_string()],
        );
        let partial = PartialHooksConfig {
            post_create: Some(vec!["new_create".to_string()]),
            pre_remove: None,
            post_merge: Some(vec!["new_merge".to_string()]),
        };
        hooks.merge_partial(partial).expect("should merge");
        assert_eq!(hooks.post_create, vec!["new_create"]);
        assert_eq!(hooks.pre_remove, vec!["old_remove"]);
        assert_eq!(hooks.post_merge, vec!["new_merge"]);
    }

    #[test]
    fn partial_hooks_config_merge_replaces_with_empty_vec() {
        let mut hooks = HooksConfig::with_values(
            vec!["cmd".to_string()],
            vec![],
            vec![],
        );
        let partial = PartialHooksConfig {
            post_create: Some(vec![]),
            pre_remove: None,
            post_merge: None,
        };
        hooks.merge_partial(partial).expect("should merge");
        assert!(hooks.post_create.is_empty());
    }

    #[test]
    fn partial_hooks_config_none_preserves_all() {
        let hooks = HooksConfig::with_values(
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        );
        let mut hooks2 = hooks.clone();
        let partial = PartialHooksConfig::default();
        hooks2.merge_partial(partial).expect("should merge");
        assert_eq!(hooks, hooks2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Partial config serde roundtrips
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn partial_conflict_resolution_serde_roundtrip() {
        let partial = PartialConflictResolutionConfig {
            mode: Some(ConflictMode::Auto),
            autonomy: Some(99),
            security_keywords: Some(vec!["key".to_string()]),
            log_resolutions: Some(ValidatedBool::new(false)),
        };
        let json = serde_json::to_string(&partial).expect("serialize");
        let deserialized: PartialConflictResolutionConfig =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(partial, deserialized);
    }

    #[test]
    fn partial_conflict_resolution_serde_all_none_roundtrip() {
        let partial = PartialConflictResolutionConfig::default();
        let json = serde_json::to_string(&partial).expect("serialize");
        let deserialized: PartialConflictResolutionConfig =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(partial, deserialized);
    }

    #[test]
    fn partial_session_config_serde_roundtrip() {
        let partial = PartialSessionConfig {
            auto_commit: Some(ValidatedBool::new(true)),
            commit_prefix: Some("feat:".to_string()),
            max_sessions: Some(50),
        };
        let json = serde_json::to_string(&partial).expect("serialize");
        let deserialized: PartialSessionConfig =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(partial, deserialized);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Merge does not validate: merging invalid values is allowed
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn merge_allows_autonomy_above_100() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            autonomy: Some(200),
            ..Default::default()
        };
        // merge_partial itself does not validate autonomy range
        assert!(config.merge_partial(partial).is_ok());
        assert_eq!(config.autonomy, 200);
    }

    #[test]
    fn merge_allows_empty_security_keywords() {
        let mut config = ConflictResolutionConfig::default();
        let partial = PartialConflictResolutionConfig {
            security_keywords: Some(vec![]),
            ..Default::default()
        };
        assert!(config.merge_partial(partial).is_ok());
        assert!(config.security_keywords.is_empty());
    }

    #[test]
    fn merge_allows_empty_agent_command() {
        let mut config = AgentConfig::default();
        let partial = PartialAgentConfig {
            command: Some(String::new()),
            env: None,
        };
        assert!(config.merge_partial(partial).is_ok());
        assert!(config.command.is_empty());
    }

    #[test]
    fn merge_allows_zero_max_sessions() {
        let mut config = SessionConfig::default();
        let partial = PartialSessionConfig {
            max_sessions: Some(0),
            ..Default::default()
        };
        assert!(config.merge_partial(partial).is_ok());
        assert_eq!(config.max_sessions, 0);
    }
}
