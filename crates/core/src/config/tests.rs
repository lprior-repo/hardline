//! Tests for AgentConfig, SessionConfig, HooksConfig and their Partial variants
#![allow(clippy::redundant_clone)]

use std::collections::HashMap;

use crate::config::{
    AgentConfig, HooksConfig, PartialAgentConfig, PartialHooksConfig,
    PartialSessionConfig, SessionConfig,
};
use crate::config::types::ValidatedBool;

// ═══════════════════════════════════════════════════════════════════════════
// AgentConfig tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn agent_config_default_values() {
    let config = AgentConfig::default();
    assert_eq!(config.command, "claude");
    assert!(config.env.is_empty());
}

#[test]
fn agent_config_new_returns_defaults() {
    let config = AgentConfig::new();
    assert_eq!(config.command, "claude");
    assert!(config.env.is_empty());
}

#[test]
fn agent_config_with_command() {
    let config = AgentConfig::with_command("opus");
    assert_eq!(config.command, "opus");
    assert!(config.env.is_empty());
}

#[test]
fn agent_config_with_values() {
    let mut env = HashMap::new();
    env.insert("API_KEY".to_string(), "secret".to_string());
    let config = AgentConfig::with_values("claude-opus", env.clone());
    assert_eq!(config.command, "claude-opus");
    assert_eq!(config.env, env);
}

#[test]
fn agent_config_validate_accepts_valid_command() {
    let config = AgentConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn agent_config_validate_rejects_empty_command() {
    let config = AgentConfig::with_command("");
    assert!(config.validate().is_err());
    let err_msg = format!("{:?}", config.validate().unwrap_err());
    assert!(err_msg.contains("agent.command must not be empty"));
}

#[test]
fn agent_config_validate_rejects_shell_metacharacters() {
    let dangerous = ["|", "&", ";", "$", "`", "(", ")", "<", ">"];
    for cmd in &dangerous {
        let config = AgentConfig::with_command(*cmd);
        assert!(config.validate().is_err(), "Command '{cmd}' should be rejected");
    }
}

#[test]
fn agent_config_validate_rejects_empty_env_key() {
    let mut env = HashMap::new();
    env.insert("".to_string(), "val".to_string());
    let config = AgentConfig::with_values("claude", env);
    assert!(config.validate().is_err());
}

#[test]
fn agent_config_validate_rejects_invalid_env_key_start() {
    let mut env = HashMap::new();
    env.insert("1BAD".to_string(), "val".to_string());
    let config = AgentConfig::with_values("claude", env);
    assert!(config.validate().is_err());
}

#[test]
fn agent_config_validate_rejects_invalid_env_key_chars() {
    let mut env = HashMap::new();
    env.insert("BAD-KEY".to_string(), "val".to_string());
    let config = AgentConfig::with_values("claude", env);
    assert!(config.validate().is_err());
}

#[test]
fn agent_config_validate_accepts_valid_env_keys() {
    let mut env = HashMap::new();
    env.insert("API_KEY".to_string(), "secret".to_string());
    env.insert("_PRIVATE".to_string(), "value".to_string());
    env.insert("My_Var".to_string(), "ok".to_string());
    let config = AgentConfig::with_values("claude", env);
    assert!(config.validate().is_ok());
}

#[test]
fn agent_config_serde_roundtrip() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "val".to_string());
    let config = AgentConfig::with_values("claude", env);
    let json = serde_json::to_string(&config).expect("serialize");
    let deserialized: AgentConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config, deserialized);
}

#[test]
fn agent_config_toml_roundtrip() {
    let config = AgentConfig::default();
    let toml = toml::to_string(&config).expect("serialize toml");
    let deserialized: AgentConfig = toml::from_str(&toml).expect("deserialize toml");
    assert_eq!(config, deserialized);
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionConfig tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn session_config_default_values() {
    let config = SessionConfig::default();
    assert_eq!(config.auto_commit, ValidatedBool::new(false));
    assert_eq!(config.commit_prefix, "wip:");
    assert_eq!(config.max_sessions, 100);
}

#[test]
fn session_config_new_returns_defaults() {
    let config = SessionConfig::new();
    assert_eq!(config.commit_prefix, "wip:");
    assert_eq!(config.max_sessions, 100);
}

#[test]
fn session_config_with_values() {
    let config = SessionConfig::with_values(
        ValidatedBool::new(true),
        "feat:",
        50,
    );
    assert!(config.auto_commit.value());
    assert_eq!(config.commit_prefix, "feat:");
    assert_eq!(config.max_sessions, 50);
}

#[test]
fn session_config_validate_accepts_valid() {
    let config = SessionConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn session_config_validate_rejects_zero_max_sessions() {
    let config = SessionConfig::with_values(ValidatedBool::new(false), "wip:", 0);
    assert!(config.validate().is_err());
    let err_msg = format!("{:?}", config.validate().unwrap_err());
    assert!(err_msg.contains("session.max_sessions must be greater than 0"));
}

#[test]
fn session_config_serde_roundtrip() {
    let config = SessionConfig::with_values(ValidatedBool::new(true), "feat:", 50);
    let json = serde_json::to_string(&config).expect("serialize");
    let deserialized: SessionConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config, deserialized);
}

#[test]
fn session_config_toml_roundtrip() {
    let config = SessionConfig::default();
    let toml = toml::to_string(&config).expect("serialize toml");
    let deserialized: SessionConfig = toml::from_str(&toml).expect("deserialize toml");
    assert_eq!(config, deserialized);
}

// ═══════════════════════════════════════════════════════════════════════════
// PartialAgentConfig merge tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn partial_agent_config_merge_updates_command() {
    let mut config = AgentConfig::default();
    let partial = PartialAgentConfig {
        command: Some("opus".to_string()),
        env: None,
    };
    config.merge_partial(partial).expect("merge should succeed");
    assert_eq!(config.command, "opus");
    assert!(config.env.is_empty(), "env should remain empty when not set");
}

#[test]
fn partial_agent_config_merge_updates_env() {
    let mut config = AgentConfig::default();
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "val".to_string());
    let partial = PartialAgentConfig {
        command: None,
        env: Some(env),
    };
    config.merge_partial(partial).expect("merge should succeed");
    assert_eq!(config.command, "claude", "command should remain default");
    assert_eq!(config.env.len(), 1);
}

#[test]
fn partial_agent_config_merge_all_none_preserves_defaults() {
    let config = AgentConfig::default();
    let mut config2 = config.clone();
    let partial = PartialAgentConfig::default();
    config2.merge_partial(partial).expect("merge should succeed");
    assert_eq!(config, config2);
}

#[test]
fn partial_agent_config_serde_roundtrip() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "val".to_string());
    let partial = PartialAgentConfig {
        command: Some("opus".to_string()),
        env: Some(env),
    };
    let json = serde_json::to_string(&partial).expect("serialize");
    let deserialized: PartialAgentConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(partial, deserialized);
}

// ═══════════════════════════════════════════════════════════════════════════
// PartialSessionConfig merge tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn partial_session_config_merge_updates_auto_commit() {
    let mut config = SessionConfig::default();
    let partial = PartialSessionConfig {
        auto_commit: Some(ValidatedBool::new(true)),
        commit_prefix: None,
        max_sessions: None,
    };
    config.merge_partial(partial).expect("merge should succeed");
    assert!(config.auto_commit.value());
    assert_eq!(config.commit_prefix, "wip:");
    assert_eq!(config.max_sessions, 100);
}

#[test]
fn partial_session_config_merge_all_none_preserves_defaults() {
    let config = SessionConfig::default();
    let mut config2 = config.clone();
    let partial = PartialSessionConfig::default();
    config2.merge_partial(partial).expect("merge should succeed");
    assert_eq!(config, config2);
}

// ═══════════════════════════════════════════════════════════════════════════
// HooksConfig tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn hooks_config_default_is_empty() {
    let hooks = HooksConfig::default();
    assert!(hooks.post_create.is_empty());
    assert!(hooks.pre_remove.is_empty());
    assert!(hooks.post_merge.is_empty());
}

#[test]
fn hooks_config_new_is_empty() {
    let hooks = HooksConfig::new();
    assert!(hooks.post_create.is_empty());
    assert!(hooks.pre_remove.is_empty());
    assert!(hooks.post_merge.is_empty());
}

#[test]
fn hooks_config_with_values_sets_fields() {
    let hooks = HooksConfig::with_values(
        vec!["echo create".to_string()],
        vec!["echo remove".to_string()],
        vec!["echo merge".to_string()],
    );
    assert_eq!(hooks.post_create, vec!["echo create"]);
    assert_eq!(hooks.pre_remove, vec!["echo remove"]);
    assert_eq!(hooks.post_merge, vec!["echo merge"]);
}

#[test]
fn hooks_config_has_hooks_false_when_empty() {
    let hooks = HooksConfig::default();
    assert!(!hooks.has_hooks());
}

#[test]
fn hooks_config_has_hooks_true_with_post_create() {
    let hooks = HooksConfig::with_values(
        vec!["cmd".to_string()],
        vec![],
        vec![],
    );
    assert!(hooks.has_hooks());
}

#[test]
fn hooks_config_has_hooks_true_with_pre_remove() {
    let hooks = HooksConfig::with_values(
        vec![],
        vec!["cmd".to_string()],
        vec![],
    );
    assert!(hooks.has_hooks());
}

#[test]
fn hooks_config_has_hooks_true_with_post_merge() {
    let hooks = HooksConfig::with_values(
        vec![],
        vec![],
        vec!["cmd".to_string()],
    );
    assert!(hooks.has_hooks());
}

#[test]
fn hooks_config_equality() {
    let a = HooksConfig::with_values(
        vec!["a".to_string()],
        vec!["b".to_string()],
        vec!["c".to_string()],
    );
    let b = HooksConfig::with_values(
        vec!["a".to_string()],
        vec!["b".to_string()],
        vec!["c".to_string()],
    );
    assert_eq!(a, b);
}

#[test]
fn hooks_config_clone() {
    let hooks = HooksConfig::with_values(
        vec!["echo".to_string()],
        vec![],
        vec!["merge".to_string()],
    );
    let cloned = hooks.clone();
    assert_eq!(hooks, cloned);
}

#[test]
fn hooks_config_serde_roundtrip() {
    let hooks = HooksConfig::with_values(
        vec!["echo 'created'".to_string(), "git status".to_string()],
        vec!["echo 'removing'".to_string()],
        vec![],
    );
    let json = serde_json::to_string(&hooks).expect("should serialize");
    let deserialized: HooksConfig = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(hooks, deserialized);
}

#[test]
fn hooks_config_toml_roundtrip() {
    let hooks = HooksConfig::with_values(
        vec!["echo create".to_string()],
        vec!["echo remove".to_string()],
        vec!["echo merge".to_string()],
    );
    let toml_str = toml::to_string(&hooks).expect("should serialize to toml");
    let deserialized: HooksConfig = toml::from_str(&toml_str).expect("should deserialize from toml");
    assert_eq!(hooks, deserialized);
}

// ═══════════════════════════════════════════════════════════════════════════
// PartialHooksConfig merge tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn partial_hooks_config_default_is_all_none() {
    let partial = PartialHooksConfig::default();
    assert!(partial.post_create.is_none());
    assert!(partial.pre_remove.is_none());
    assert!(partial.post_merge.is_none());
}

#[test]
fn partial_hooks_config_merge_updates_only_some_fields() {
    let mut hooks = HooksConfig::with_values(
        vec!["original_create".to_string()],
        vec!["original_remove".to_string()],
        vec!["original_merge".to_string()],
    );

    let partial = PartialHooksConfig {
        post_create: Some(vec!["new_create".to_string()]),
        pre_remove: None,
        post_merge: None,
    };

    hooks.merge_partial(partial).expect("should merge");
    assert_eq!(hooks.post_create, vec!["new_create"]);
    assert_eq!(hooks.pre_remove, vec!["original_remove"]);
    assert_eq!(hooks.post_merge, vec!["original_merge"]);
}

#[test]
fn partial_hooks_config_merge_updates_all_fields() {
    let mut hooks = HooksConfig::default();

    let partial = PartialHooksConfig {
        post_create: Some(vec!["a".to_string()]),
        pre_remove: Some(vec!["b".to_string(), "c".to_string()]),
        post_merge: Some(vec!["d".to_string()]),
    };

    hooks.merge_partial(partial).expect("should merge");
    assert_eq!(hooks.post_create, vec!["a"]);
    assert_eq!(hooks.pre_remove, vec!["b", "c"]);
    assert_eq!(hooks.post_merge, vec!["d"]);
}

#[test]
fn partial_hooks_config_merge_none_preserves_originals() {
    let mut hooks = HooksConfig::with_values(
        vec!["keep".to_string()],
        vec!["keep_too".to_string()],
        vec!["keep_three".to_string()],
    );

    let partial = PartialHooksConfig::default();

    hooks.merge_partial(partial).expect("should merge");
    assert_eq!(hooks.post_create, vec!["keep"]);
    assert_eq!(hooks.pre_remove, vec!["keep_too"]);
    assert_eq!(hooks.post_merge, vec!["keep_three"]);
}

#[test]
fn partial_hooks_config_serde_roundtrip() {
    let partial = PartialHooksConfig {
        post_create: Some(vec!["echo".to_string()]),
        pre_remove: None,
        post_merge: Some(vec!["merge".to_string()]),
    };
    let json = serde_json::to_string(&partial).expect("should serialize");
    let deserialized: PartialHooksConfig = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(partial, deserialized);
}
