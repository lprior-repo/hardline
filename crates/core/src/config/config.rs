//! Domain configuration structs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Result;

use super::types::{ConflictMode, ValidatedBool};

// ═══════════════════════════════════════════════════════════════════════════
// HooksConfig
// ═══════════════════════════════════════════════════════════════════════════

/// Hook commands triggered at lifecycle events.
///
/// Each field is a list of shell commands executed sequentially.
/// Empty lists mean no hooks are run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HooksConfig {
    /// Commands to run after workspace creation.
    pub post_create: Vec<String>,
    /// Commands to run before workspace removal.
    pub pre_remove: Vec<String>,
    /// Commands to run after merge.
    pub post_merge: Vec<String>,
}

impl HooksConfig {
    /// Create a new HooksConfig with all fields empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific hook commands.
    #[must_use]
    pub fn with_values(
        post_create: Vec<String>,
        pre_remove: Vec<String>,
        post_merge: Vec<String>,
    ) -> Self {
        Self {
            post_create,
            pre_remove,
            post_merge,
        }
    }

    /// Whether any hooks are configured.
    #[must_use]
    pub fn has_hooks(&self) -> bool {
        !self.post_create.is_empty() || !self.pre_remove.is_empty() || !self.post_merge.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ConflictResolutionConfig
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictResolutionConfig {
    pub mode: ConflictMode,
    pub autonomy: u8,
    pub security_keywords: Vec<String>,
    pub log_resolutions: ValidatedBool,
}

impl ConflictResolutionConfig {
    pub fn validate(&self) -> Result<()> {
        if self.autonomy > 100 {
            return Err(crate::Error::validation_error(format!(
                "autonomy must be 0-100, got {}",
                self.autonomy
            )));
        }

        if self.security_keywords.is_empty() {
            return Err(crate::Error::validation_error(
                "security_keywords must not be empty".to_string(),
            ));
        }

        match self.mode {
            ConflictMode::Auto | ConflictMode::Manual | ConflictMode::Hybrid => Ok(()),
        }
    }

    #[must_use]
    pub fn requires_human_review(&self, file_path: &str) -> bool {
        let file_path_lower = file_path.to_lowercase();
        self.security_keywords
            .iter()
            .any(|keyword| file_path_lower.contains(&keyword.to_lowercase()))
    }

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
            mode: ConflictMode::Manual,
            autonomy: 0,
            security_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "credential".to_string(),
            ],
            log_resolutions: ValidatedBool::new(true),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AgentConfig
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for the AI agent command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    /// The CLI command used to invoke the agent (e.g. "claude").
    pub command: String,
    /// Environment variables passed to the agent process.
    pub env: HashMap<String, String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            env: HashMap::new(),
        }
    }
}

impl AgentConfig {
    /// Create a new AgentConfig with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific command and empty env.
    #[must_use]
    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            env: HashMap::new(),
        }
    }

    /// Create with a specific command and env map.
    #[must_use]
    pub fn with_values(command: impl Into<String>, env: HashMap<String, String>) -> Self {
        Self {
            command: command.into(),
            env,
        }
    }

    /// Validate the agent configuration.
    pub fn validate(&self) -> Result<()> {
        if self.command.is_empty() {
            return Err(crate::Error::validation_error(
                "agent.command must not be empty".to_string(),
            ));
        }

        // Reject commands containing shell metacharacters to prevent injection
        let dangerous_chars = ['|', '&', ';', '$', '`', '(', ')', '<', '>', '\n', '\r'];
        if self.command.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(crate::Error::validation_error(
                "agent.command contains shell metacharacters".to_string(),
            ));
        }

        // Validate env keys: must not be empty and must be valid env var names
        for key in self.env.keys() {
            if key.is_empty() {
                return Err(crate::Error::validation_error(
                    "agent.env key must not be empty".to_string(),
                ));
            }
            if !key
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            {
                return Err(crate::Error::validation_error(format!(
                    "agent.env key '{}' must start with a letter or underscore",
                    key
                )));
            }
            if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(crate::Error::validation_error(format!(
                    "agent.env key '{}' contains invalid characters",
                    key
                )));
            }
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionConfig
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for session management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    /// Whether to auto-commit changes.
    pub auto_commit: ValidatedBool,
    /// Prefix prepended to auto-commit messages.
    pub commit_prefix: String,
    /// Maximum number of concurrent sessions.
    pub max_sessions: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_commit: ValidatedBool::new(false),
            commit_prefix: "wip:".to_string(),
            max_sessions: 100,
        }
    }
}

impl SessionConfig {
    /// Create a new SessionConfig with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific values.
    #[must_use]
    pub fn with_values(
        auto_commit: ValidatedBool,
        commit_prefix: impl Into<String>,
        max_sessions: usize,
    ) -> Self {
        Self {
            auto_commit,
            commit_prefix: commit_prefix.into(),
            max_sessions,
        }
    }

    /// Validate the session configuration.
    pub fn validate(&self) -> Result<()> {
        if self.max_sessions == 0 {
            return Err(crate::Error::validation_error(
                "session.max_sessions must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VcsConfig
// ═══════════════════════════════════════════════════════════════════════════

use super::types::{AuthSourceType, ForgeType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchTemplate {
    pub name: String,
    pub pattern: String,
    pub description: Option<String>,
}

impl Default for BranchTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            pattern: String::new(),
            description: None,
        }
    }
}

impl BranchTemplate {
    pub fn new(name: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pattern: pattern.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VcsConfig {
    pub forge: ForgeType,
    pub default_branch: String,
    pub branch_templates: Vec<BranchTemplate>,
}

impl Default for VcsConfig {
    fn default() -> Self {
        Self {
            forge: ForgeType::default(),
            default_branch: "main".to_string(),
            branch_templates: Vec::new(),
        }
    }
}

impl VcsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_forge(mut self, forge: ForgeType) -> Self {
        self.forge = forge;
        self
    }

    pub fn with_default_branch(mut self, branch: impl Into<String>) -> Self {
        self.default_branch = branch.into();
        self
    }

    pub fn add_template(mut self, template: BranchTemplate) -> Self {
        self.branch_templates.push(template);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AuthConfig
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthConfig {
    pub preferred_source: AuthSourceType,
    pub allow_github_token_env: bool,
    pub allow_stax_token_env: bool,
    pub allow_credentials_file: bool,
    pub allow_gh_cli: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            preferred_source: AuthSourceType::default(),
            allow_github_token_env: false,
            allow_stax_token_env: true,
            allow_credentials_file: true,
            allow_gh_cli: true,
        }
    }
}

impl AuthConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_preferred_source(mut self, source: AuthSourceType) -> Self {
        self.preferred_source = source;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigScope;

    // ═══════════════════════════════════════════════════════════════════════════
    // AgentConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn agent_config_default_command_is_claude() {
        assert_eq!(AgentConfig::default().command, "claude");
    }

    #[test]
    fn agent_config_default_env_is_empty() {
        assert!(AgentConfig::default().env.is_empty());
    }

    #[test]
    fn agent_config_with_command_leaves_env_empty() {
        let config = AgentConfig::with_command("my-agent");
        assert_eq!(config.command, "my-agent");
        assert!(config.env.is_empty());
    }

    #[test]
    fn agent_config_validate_rejects_newlines_in_command() {
        let config = AgentConfig::with_command("claude\nrm -rf /");
        assert!(config.validate().is_err());
    }

    #[test]
    fn agent_config_validate_rejects_carriage_return_in_command() {
        let config = AgentConfig::with_command("claude\rmalicious");
        assert!(config.validate().is_err());
    }

    #[test]
    fn agent_config_equality() {
        let mut env1 = HashMap::new();
        env1.insert("K".to_string(), "v".to_string());
        let a = AgentConfig::with_values("claude", env1.clone());
        let b = AgentConfig::with_values("claude", env1);
        assert_eq!(a, b);
    }

    #[test]
    fn agent_config_inequality_different_command() {
        let a = AgentConfig::with_command("claude");
        let b = AgentConfig::with_command("opus");
        assert_ne!(a, b);
    }

    #[test]
    fn agent_config_clone() {
        let config = AgentConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_config_default_auto_commit_is_false() {
        assert!(!SessionConfig::default().auto_commit.value());
    }

    #[test]
    fn session_config_default_commit_prefix_is_wip() {
        assert_eq!(SessionConfig::default().commit_prefix, "wip:");
    }

    #[test]
    fn session_config_default_max_sessions_is_100() {
        assert_eq!(SessionConfig::default().max_sessions, 100);
    }

    #[test]
    fn session_config_validate_accepts_max_sessions_of_1() {
        let config = SessionConfig::with_values(ValidatedBool::new(false), "wip:", 1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn session_config_equality() {
        let a = SessionConfig::with_values(ValidatedBool::new(true), "feat:", 50);
        let b = SessionConfig::with_values(ValidatedBool::new(true), "feat:", 50);
        assert_eq!(a, b);
    }

    #[test]
    fn session_config_clone() {
        let config = SessionConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HooksConfig additional tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn hooks_config_inequality() {
        let a = HooksConfig::with_values(vec!["x".into()], vec![], vec![]);
        let b = HooksConfig::with_values(vec!["y".into()], vec![], vec![]);
        assert_ne!(a, b);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ConflictResolutionConfig tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn conflict_resolution_default_mode_is_manual() {
        let config = ConflictResolutionConfig::default();
        assert_eq!(config.mode, ConflictMode::Manual);
    }

    #[test]
    fn conflict_resolution_default_autonomy_is_zero() {
        let config = ConflictResolutionConfig::default();
        assert_eq!(config.autonomy, 0);
    }

    #[test]
    fn conflict_resolution_default_security_keywords_not_empty() {
        let config = ConflictResolutionConfig::default();
        assert!(!config.security_keywords.is_empty());
    }

    #[test]
    fn conflict_resolution_default_log_resolutions_is_true() {
        let config = ConflictResolutionConfig::default();
        assert!(config.log_resolutions.value());
    }

    #[test]
    fn conflict_resolution_default_has_expected_keywords() {
        let config = ConflictResolutionConfig::default();
        assert!(config.security_keywords.contains(&"password".to_string()));
        assert!(config.security_keywords.contains(&"token".to_string()));
        assert!(config.security_keywords.contains(&"secret".to_string()));
        assert!(config.security_keywords.contains(&"key".to_string()));
        assert!(config.security_keywords.contains(&"credential".to_string()));
    }

    #[test]
    fn conflict_resolution_validate_accepts_valid_config() {
        let config = ConflictResolutionConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn conflict_resolution_validate_rejects_autonomy_above_100() {
        let config = ConflictResolutionConfig {
            autonomy: 101,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("autonomy must be 0-100"));
    }

    #[test]
    fn conflict_resolution_validate_accepts_autonomy_of_100() {
        let config = ConflictResolutionConfig {
            autonomy: 100,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn conflict_resolution_validate_rejects_empty_security_keywords() {
        let config = ConflictResolutionConfig {
            security_keywords: vec![],
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("security_keywords must not be empty"));
    }

    #[test]
    fn conflict_resolution_validate_accepts_all_modes() {
        for mode in [
            ConflictMode::Auto,
            ConflictMode::Manual,
            ConflictMode::Hybrid,
        ] {
            let config = ConflictResolutionConfig {
                mode,
                ..Default::default()
            };
            assert!(config.validate().is_ok(), "mode {mode:?} should validate");
        }
    }

    #[test]
    fn conflict_resolution_requires_human_review_matches_keyword() {
        let config = ConflictResolutionConfig::default();
        assert!(config.requires_human_review("config/passwords.yaml"));
        assert!(config.requires_human_review("src/token_store.rs"));
        assert!(config.requires_human_review("secrets/env"));
    }

    #[test]
    fn conflict_resolution_requires_human_review_case_insensitive() {
        let config = ConflictResolutionConfig::default();
        assert!(config.requires_human_review("CONFIG/PASSWORDS.YAML"));
        assert!(config.requires_human_review("Src/Token_Store.rs"));
    }

    #[test]
    fn conflict_resolution_requires_human_review_no_match() {
        let config = ConflictResolutionConfig::default();
        assert!(!config.requires_human_review("src/main.rs"));
        assert!(!config.requires_human_review("README.md"));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_auto_mode() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Auto,
            ..Default::default()
        };
        assert!(config.can_auto_resolve(Some("secret/key")));
        assert!(config.can_auto_resolve(None));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_manual_mode() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Manual,
            ..Default::default()
        };
        assert!(!config.can_auto_resolve(Some("safe/file.rs")));
        assert!(!config.can_auto_resolve(None));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_hybrid_low_autonomy() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Hybrid,
            autonomy: 30,
            ..Default::default()
        };
        assert!(!config.can_auto_resolve(Some("safe/file.rs")));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_hybrid_high_autonomy_safe_file() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Hybrid,
            autonomy: 80,
            ..Default::default()
        };
        assert!(config.can_auto_resolve(Some("src/main.rs")));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_hybrid_high_autonomy_sensitive_file() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Hybrid,
            autonomy: 80,
            ..Default::default()
        };
        assert!(!config.can_auto_resolve(Some("secret/key")));
    }

    #[test]
    fn conflict_resolution_can_auto_resolve_hybrid_no_path_uses_autonomy() {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Hybrid,
            autonomy: 50,
            ..Default::default()
        };
        assert!(config.can_auto_resolve(None));

        let config_low = ConflictResolutionConfig {
            mode: ConflictMode::Hybrid,
            autonomy: 49,
            ..Default::default()
        };
        assert!(!config_low.can_auto_resolve(None));
    }

    #[test]
    fn conflict_resolution_equality() {
        let a = ConflictResolutionConfig::default();
        let b = ConflictResolutionConfig::default();
        assert_eq!(a, b);
    }

    #[test]
    fn conflict_resolution_clone() {
        let config = ConflictResolutionConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ConfigScope tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn config_scope_global_display() {
        assert_eq!(format!("{}", ConfigScope::Global), "Global");
    }

    #[test]
    fn config_scope_project_display() {
        assert_eq!(format!("{}", ConfigScope::Project), "Project");
    }

    #[test]
    fn config_scope_env_display() {
        assert_eq!(format!("{}", ConfigScope::Env), "Env");
    }

    #[test]
    fn config_scope_default_is_global() {
        assert_eq!(ConfigScope::default(), ConfigScope::Global);
    }

    #[test]
    fn config_scope_all_variants_exhaustive_match() {
        let scopes = [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env];
        assert_eq!(scopes.len(), 3);
        for scope in scopes {
            // Ensure Display works for all variants without panic
            let _s = format!("{scope}");
        }
    }
}
