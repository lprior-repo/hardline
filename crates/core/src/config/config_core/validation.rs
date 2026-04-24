//! Configuration key validation and known keys registry.

use crate::error::Result;
use crate::error_config::ConfigErrorKind;

/// Configuration key names
pub mod keys {
    pub const VCS_TYPE: &str = "vcs.type";
    pub const DEFAULT_BRANCH: &str = "vcs.default_branch";
    pub const WORKSPACE_DIR: &str = "workspace.directory";
    pub const QUEUE_NAME: &str = "queue.default";
    pub const LOG_LEVEL: &str = "logging.level";
    pub const EDITOR: &str = "editor";
    pub const REMOTE_PUSH: &str = "remote.push";
    pub const REMOTE_FETCH: &str = "remote.fetch";
    pub const AUTO_REBASE: &str = "workspace.auto_rebase";
    pub const AUTO_PUSH: &str = "workspace.auto_push";
}

/// Validated list of all known configuration keys.
///
/// This is used by [`validate_key`] to reject unknown keys at runtime
/// with a helpful error message listing valid keys grouped by category.
pub const VALID_CONFIG_KEYS: &[&str] = &[
    // Top-level (section-only) keys
    "watch",
    "conflict_resolution",
    "session",
    "hooks",
    "agent",
    "vcs",
    "workspace",
    "queue",
    "logging",
    "remote",
    "editor",
    "auth",
    // Watch section
    "watch.enabled",
    "watch.debounce_ms",
    "watch.paths",
    // Conflict resolution section
    "conflict_resolution.mode",
    "conflict_resolution.autonomy",
    "conflict_resolution.security_keywords",
    "conflict_resolution.log_resolutions",
    // Session section
    "session.auto_commit",
    "session.commit_prefix",
    "session.max_sessions",
    // Hooks section
    "hooks.post_create",
    "hooks.pre_remove",
    "hooks.post_merge",
    // Agent section
    "agent.command",
    // VCS section
    "vcs.type",
    "vcs.default_branch",
    "vcs.forge",
    "vcs.branch_templates",
    // Workspace section
    "workspace.directory",
    "workspace.auto_rebase",
    "workspace.auto_push",
    // Queue section
    "queue.default",
    // Logging section
    "logging.level",
    // Remote section
    "remote.push",
    "remote.fetch",
    // Auth section
    "auth.preferred_source",
    "auth.allow_github_token_env",
    "auth.allow_stax_token_env",
    "auth.allow_credentials_file",
    "auth.allow_gh_cli",
];

/// Environment variable prefix for SCP config overrides.
/// For example, `SCP_VCS_TYPE` maps to `vcs.type`.
pub const ENV_PREFIX: &str = "SCP_";

/// Validate a configuration key.
///
/// Checks if the given key is either:
/// - An exact match in [`VALID_CONFIG_KEYS`], or
/// - A parent prefix of a valid key (e.g. `"watch"` is valid because
///   `"watch.enabled"` etc. start with `"watch."`)
///
/// # Errors
///
/// Returns `ConfigErrorKind::ConfigParseError` if the key is not recognized.
/// The error message includes a list of valid keys grouped by category.
pub fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(ConfigErrorKind::ConfigParseError("empty config key".to_string()).into());
    }

    let is_valid = VALID_CONFIG_KEYS
        .iter()
        .any(|valid_key| key == *valid_key || valid_key.starts_with(&format!("{key}.")));

    if is_valid {
        Ok(())
    } else {
        let mut msg = format!("Unknown configuration key: '{key}'\n\n");
        msg.push_str("  watch.enabled, watch.debounce_ms, watch.paths\n");
        msg.push_str("  conflict_resolution.mode, conflict_resolution.autonomy,\n");
        msg.push_str(
            "    conflict_resolution.security_keywords, conflict_resolution.log_resolutions\n",
        );
        msg.push_str("  session.auto_commit, session.commit_prefix, session.max_sessions\n");
        msg.push_str("  hooks.post_create, hooks.pre_remove, hooks.post_merge\n");
        msg.push_str("  agent.command\n");
        msg.push_str("  vcs.type, vcs.default_branch, vcs.forge, vcs.branch_templates\n");
        msg.push_str("  workspace.directory, workspace.auto_rebase, workspace.auto_push\n");
        msg.push_str("  queue.default\n");
        msg.push_str("  logging.level\n");
        msg.push_str("  remote.push, remote.fetch\n");
        msg.push_str("  editor\n");
        msg.push_str("  auth.preferred_source, auth.allow_github_token_env,\n");
        msg.push_str(
            "  auth.allow_stax_token_env, auth.allow_credentials_file, auth.allow_gh_cli\n",
        );
        msg.push_str("\nUse 'scp config list' to see current configuration.");
        Err(ConfigErrorKind::ConfigParseError(msg).into())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prop_assert;

    use super::*;

    #[test]
    fn validate_key_accepts_exact_leaf_keys() {
        let leaf_keys = [
            "watch.enabled",
            "watch.debounce_ms",
            "watch.paths",
            "conflict_resolution.mode",
            "conflict_resolution.autonomy",
            "conflict_resolution.security_keywords",
            "conflict_resolution.log_resolutions",
            "session.auto_commit",
            "session.commit_prefix",
            "session.max_sessions",
            "hooks.post_create",
            "hooks.pre_remove",
            "hooks.post_merge",
            "agent.command",
            "vcs.type",
            "vcs.default_branch",
            "workspace.directory",
            "workspace.auto_rebase",
            "workspace.auto_push",
            "queue.default",
            "logging.level",
            "remote.push",
            "remote.fetch",
            "editor",
        ];
        for key in &leaf_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    #[test]
    fn validate_key_accepts_section_prefixes() {
        let section_keys = [
            "watch",
            "conflict_resolution",
            "session",
            "hooks",
            "agent",
            "vcs",
            "workspace",
            "queue",
            "logging",
            "remote",
        ];
        for key in &section_keys {
            assert!(validate_key(key).is_ok(), "Section '{key}' should be valid");
        }
    }

    #[test]
    fn validate_key_rejects_unknown_keys() {
        let invalid_keys = [
            "foo.bar",
            "unknown_key",
            "watch.nonexistent",
            "session.invalid_field",
            "nope",
            "vcs.svn",
        ];
        for key in &invalid_keys {
            let result = validate_key(key);
            assert!(result.is_err(), "Key '{key}' should be rejected");
            let err_msg = format!("{result:?}");
            assert!(
                err_msg.contains("Unknown configuration key"),
                "Error for '{key}' should mention unknown key, got: {err_msg}"
            );
        }
    }

    #[test]
    fn validate_key_rejects_empty_string() {
        let result = validate_key("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_key_error_lists_valid_keys() {
        let result = validate_key("bad_key");
        assert!(result.is_err());
        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.contains("watch.enabled"),
            "Should list watch.enabled"
        );
        assert!(
            err_str.contains("conflict_resolution.mode"),
            "Should list conflict_resolution.mode"
        );
        assert!(
            err_str.contains("session.auto_commit"),
            "Should list session.auto_commit"
        );
        assert!(
            err_str.contains("hooks.post_create"),
            "Should list hooks.post_create"
        );
        assert!(err_str.contains("vcs.type"), "Should list vcs.type");
        assert!(
            err_str.contains("workspace.directory"),
            "Should list workspace.directory"
        );
    }

    #[test]
    fn env_prefix_is_scp() {
        assert_eq!(ENV_PREFIX, "SCP_");
    }

    #[test]
    fn valid_config_keys_is_nonempty() {
        assert!(!VALID_CONFIG_KEYS.is_empty());
    }

    #[test]
    fn valid_config_keys_covers_all_known_keys() {
        let known_from_command = crate::config::command_types::KNOWN_CONFIG_KEYS;
        for key in known_from_command {
            assert!(
                VALID_CONFIG_KEYS.contains(key),
                "KNOWN_CONFIG_KEYS entry '{key}' should be in VALID_CONFIG_KEYS"
            );
        }
    }

    proptest::proptest! {
        #[test]
        fn proptest_validate_key_accepts_all_valid_keys(idx in 0..VALID_CONFIG_KEYS.len()) {
            let key = VALID_CONFIG_KEYS[idx];
            prop_assert!(validate_key(key).is_ok(), "VALID_CONFIG_KEYS entry '{key}' was rejected");
        }
    }

    proptest::proptest! {
        #[test]
        fn proptest_validate_key_rejects_empty_segments(
            prefix in "[a-z]{1,10}",
            suffix in "[a-z]{1,10}"
        ) {
            let double_dot = format!("{prefix}..{suffix}");
            prop_assert!(validate_key(&double_dot).is_err(),
                "Key with empty segment (double dot) should be rejected: {double_dot}");

            let leading = format!(".{suffix}");
            prop_assert!(validate_key(&leading).is_err(),
                "Key with leading dot should be rejected: {leading}");

            let trailing = format!("{prefix}.");
            prop_assert!(validate_key(&trailing).is_err(),
                "Key with trailing dot should be rejected: {trailing}");
        }
    }
}
