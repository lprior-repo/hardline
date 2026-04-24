//! Config merging, placeholder substitution, and repo name utilities.

use std::collections::HashMap;

use crate::error::Result;

use super::types::Config;

/// Get repository name from current working directory.
///
/// # Errors
///
/// Returns error if the current directory cannot be determined or
/// the directory name cannot be extracted.
pub fn get_repo_name() -> Result<String> {
    let dir = std::env::current_dir().map_err(|e| {
        crate::error::Error::io_error(format!("Failed to get current directory: {e}"))
    })?;
    dir.file_name()
        .ok_or_else(|| {
            crate::error::Error::io_error(
                "Failed to determine repository name from current directory".to_string(),
            )
        })
        .map(|name| name.to_string_lossy().to_string())
}

/// Substitute `{repo}` placeholders in all config values with the current
/// repository name.
///
/// Only string values containing `{repo}` are modified; all other values
/// are left unchanged.
///
/// # Errors
///
/// Returns error if the repository name cannot be determined.
pub fn substitute_placeholders(config: &mut Config) -> Result<()> {
    let repo_name = get_repo_name()?;
    let updated: HashMap<String, String> = config
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.replace("{repo}", &repo_name)))
        .collect();
    config.values = updated;
    Ok(())
}

#[cfg(test)]
mod tests {
    use proptest::prop_assert;
    use proptest::prop_assert_eq;

    use super::*;
    use super::super::types::Config;

    #[test]
    fn substitute_placeholders_replaces_repo_placeholder() {
        let mut config = Config::new();
        config.set("workspace.directory", "../{repo}__workspaces");
        config.set("logging.level", "info");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        let workspace_dir = config.get("workspace.directory").expect("key should exist");
        assert!(
            !workspace_dir.contains("{repo}"),
            "Placeholder should be replaced, got: {workspace_dir}"
        );
        assert!(
            workspace_dir.contains("__workspaces"),
            "Non-placeholder content should be preserved, got: {workspace_dir}"
        );
        assert_eq!(config.get("logging.level"), Some(&"info".to_string()));
    }

    #[test]
    fn substitute_placeholders_handles_multiple_placeholders() {
        let mut config = Config::new();
        config.set("remote.push", "https://github.com/{repo}/{repo}.git");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        let push = config.get("remote.push").expect("key should exist");
        assert!(
            !push.contains("{repo}"),
            "All placeholders should be replaced, got: {push}"
        );
    }

    #[test]
    fn substitute_placeholders_no_error_when_no_values() {
        let mut config = Config::new();
        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn substitute_placeholders_preserves_non_placeholder_values() {
        let mut config = Config::new();
        config.set("vcs.type", "git");
        config.set("logging.level", "debug");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        assert_eq!(config.get("vcs.type"), Some(&"git".to_string()));
        assert_eq!(config.get("logging.level"), Some(&"debug".to_string()));
    }

    #[test]
    fn get_repo_name_returns_current_dir_name() {
        let result = get_repo_name();
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(!name.is_empty());
    }

    proptest::proptest! {
        #[test]
        fn proptest_substitute_placeholders(
            before in ".{0,64}",
            after in ".{0,64}",
            unknown_tag in "[a-z_]{1,20}"
        ) {
            let mut config = Config::new();
            let raw = format!("{before}{{repo}}{after}{{{unknown_tag}}}");
            config.set("workspace.directory", &raw);
            config.set("logging.level", "info");

            let result = substitute_placeholders(&mut config);
            prop_assert!(result.is_ok());

            let _repo_name = get_repo_name().expect("get_repo_name should succeed");
            let replaced = config.get("workspace.directory").expect("key should exist");

            prop_assert!(!replaced.contains("{repo}"),
                "Expected {{repo}} to be substituted, got: {replaced}");

            let expected_unknown = format!("{{{unknown_tag}}}");
            prop_assert!(replaced.contains(&expected_unknown),
                "Expected unknown placeholder '{expected_unknown}' to be preserved, got: {replaced}");

            prop_assert!(replaced.starts_with(&before),
                "Value should start with '{before}', got: {replaced}");
            prop_assert!(replaced.ends_with(&expected_unknown),
                "Value should end with '{expected_unknown}', got: {replaced}");

            prop_assert_eq!(config.get("logging.level"), Some(&"info".to_string()));
        }
    }
}
