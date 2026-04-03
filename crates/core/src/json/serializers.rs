//! Command output structures for JSON serialization.
//!
//! Each struct represents the JSON output of a specific CLI command.
//! These types are pure data carriers with no business logic.

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// Init command output
// ═══════════════════════════════════════════════════════════════════════════

/// Output from the `scp init` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InitOutput {
    /// Human-readable status message.
    pub message: String,
    /// Absolute path to the initialized repository root.
    pub repo_dir: String,
    /// Path to the VCS metadata directory (`.git`)
    pub vcs_dir: String,
    /// Path to the SCP config file.
    pub config_file: String,
    /// Path to the SCP state database.
    pub state_db: String,
    /// Path to the layouts directory.
    pub layouts_dir: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Diff command output
// ═══════════════════════════════════════════════════════════════════════════

/// Output from the `scp diff` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    /// Session or workspace name the diff belongs to.
    pub name: String,
    /// Base revision for the diff.
    pub base: String,
    /// Head revision for the diff.
    pub head: String,
    /// Optional diff statistics summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStatOutput>,
    /// Optional raw diff content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_content: Option<String>,
}

/// Diff statistics for a session/workspace comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DiffStatOutput {
    /// Number of files changed.
    pub files_changed: usize,
    /// Number of lines inserted.
    pub insertions: usize,
    /// Number of lines deleted.
    pub deletions: usize,
    /// Per-file breakdown.
    pub files: Vec<FileDiffStatOutput>,
}

/// Per-file diff statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FileDiffStatOutput {
    /// File path relative to the repository root.
    pub path: String,
    /// Number of lines inserted.
    pub insertions: usize,
    /// Number of lines deleted.
    pub deletions: usize,
    /// Change status (added, modified, deleted, renamed).
    pub status: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Template command outputs
// ═══════════════════════════════════════════════════════════════════════════

/// Output from the `scp template show` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TemplateShowOutput {
    /// Template name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unix timestamp of creation.
    pub created_at: i64,
    /// Unix timestamp of last update.
    pub updated_at: i64,
    /// Template layout content.
    pub layout: String,
}

/// Output from the `scp template delete` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TemplateDeleteOutput {
    /// Name of the deleted template.
    pub name: String,
    /// Human-readable status message.
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Config command outputs
// ═══════════════════════════════════════════════════════════════════════════

/// Output from `scp config get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ConfigValueOutput {
    /// Configuration key.
    pub key: String,
    /// Configuration value (JSON to support nested structures).
    pub value: serde_json::Value,
}

/// Output from `scp config set`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ConfigSetOutput {
    /// Configuration key that was set.
    pub key: String,
    /// Value that was set (string representation).
    pub value: String,
    /// Scope where the value was written (global, local, etc.).
    pub scope: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── InitOutput tests ───────────────────────────────────────────────

    #[test]
    fn test_init_output_serialization() {
        let output = InitOutput {
            message: "Initialized successfully".to_string(),
            repo_dir: "/home/user/project".to_string(),
            vcs_dir: "/home/user/project/.git".to_string(),
            config_file: "/home/user/project/.scp/config.toml".to_string(),
            state_db: "/home/user/project/.scp/state.db".to_string(),
            layouts_dir: "/home/user/project/.scp/layouts".to_string(),
        };

        let json = serde_json::to_value(&output).expect("serialize InitOutput");

        assert_eq!(
            json.get("message").and_then(|v| v.as_str()),
            Some("Initialized successfully")
        );
        assert_eq!(
            json.get("repo_dir").and_then(|v| v.as_str()),
            Some("/home/user/project")
        );
        assert_eq!(
            json.get("vcs_dir").and_then(|v| v.as_str()),
            Some("/home/user/project/.git")
        );
        assert_eq!(
            json.get("config_file").and_then(|v| v.as_str()),
            Some("/home/user/project/.scp/config.toml")
        );
        assert_eq!(
            json.get("state_db").and_then(|v| v.as_str()),
            Some("/home/user/project/.scp/state.db")
        );
        assert_eq!(
            json.get("layouts_dir").and_then(|v| v.as_str()),
            Some("/home/user/project/.scp/layouts")
        );
    }

    #[test]
    fn test_init_output_all_fields_present() {
        let output = InitOutput {
            message: "ok".to_string(),
            repo_dir: "/r".to_string(),
            vcs_dir: "/r/.git".to_string(),
            config_file: "/r/.scp/config.toml".to_string(),
            state_db: "/r/.scp/state.db".to_string(),
            layouts_dir: "/r/.scp/layouts".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let expected_fields = [
            "\"message\"",
            "\"repo_dir\"",
            "\"vcs_dir\"",
            "\"config_file\"",
            "\"state_db\"",
            "\"layouts_dir\"",
        ];
        for field in &expected_fields {
            assert!(
                json_str.contains(field),
                "InitOutput JSON should contain {field}: {json_str}"
            );
        }
    }

    #[test]
    fn test_init_output_round_trip() {
        let output = InitOutput {
            message: "done".to_string(),
            repo_dir: "/p".to_string(),
            vcs_dir: "/p/.git".to_string(),
            config_file: "/p/.scp/config.toml".to_string(),
            state_db: "/p/.scp/state.db".to_string(),
            layouts_dir: "/p/.scp/layouts".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: InitOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output, deserialized);
    }

    // ── DiffOutput tests ──────────────────────────────────────────────

    #[test]
    fn test_diff_output_serialization() {
        let output = DiffOutput {
            name: "feature-branch".to_string(),
            base: "main".to_string(),
            head: "feature-branch".to_string(),
            diff_stat: Some(DiffStatOutput {
                files_changed: 3,
                insertions: 42,
                deletions: 7,
                files: vec![
                    FileDiffStatOutput {
                        path: "src/main.rs".to_string(),
                        insertions: 30,
                        deletions: 2,
                        status: "modified".to_string(),
                    },
                    FileDiffStatOutput {
                        path: "src/new_file.rs".to_string(),
                        insertions: 12,
                        deletions: 0,
                        status: "added".to_string(),
                    },
                ],
            }),
            diff_content: Some("diff --git a/src/main.rs b/src/main.rs\n".to_string()),
        };

        let json = serde_json::to_value(&output).expect("serialize DiffOutput");

        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("feature-branch")
        );
        assert_eq!(
            json.get("base").and_then(|v| v.as_str()),
            Some("main")
        );
        assert_eq!(
            json.get("head").and_then(|v| v.as_str()),
            Some("feature-branch")
        );
        assert!(json.get("diff_stat").is_some());
        assert!(json.get("diff_content").is_some());
    }

    #[test]
    fn test_diff_output_skips_none_fields() {
        let output = DiffOutput {
            name: "empty".to_string(),
            base: "main".to_string(),
            head: "empty".to_string(),
            diff_stat: None,
            diff_content: None,
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        assert!(
            !json_str.contains("diff_stat"),
            "Should skip diff_stat when None: {json_str}"
        );
        assert!(
            !json_str.contains("diff_content"),
            "Should skip diff_content when None: {json_str}"
        );
    }

    #[test]
    fn test_diff_stat_output_serialization() {
        let stat = DiffStatOutput {
            files_changed: 2,
            insertions: 10,
            deletions: 5,
            files: vec![
                FileDiffStatOutput {
                    path: "a.txt".to_string(),
                    insertions: 10,
                    deletions: 5,
                    status: "modified".to_string(),
                },
            ],
        };

        let json = serde_json::to_value(&stat).expect("serialize DiffStatOutput");
        assert_eq!(json.get("files_changed").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(json.get("insertions").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(json.get("deletions").and_then(|v| v.as_u64()), Some(5));
    }

    #[test]
    fn test_diff_output_round_trip() {
        let output = DiffOutput {
            name: "test".to_string(),
            base: "main".to_string(),
            head: "test".to_string(),
            diff_stat: Some(DiffStatOutput {
                files_changed: 1,
                insertions: 1,
                deletions: 0,
                files: vec![FileDiffStatOutput {
                    path: "f.txt".to_string(),
                    insertions: 1,
                    deletions: 0,
                    status: "added".to_string(),
                }],
            }),
            diff_content: None,
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: DiffOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output, deserialized);
    }

    // ── TemplateShowOutput tests ───────────────────────────────────────

    #[test]
    fn test_template_show_output_serialization() {
        let output = TemplateShowOutput {
            name: "feature-branch".to_string(),
            description: Some("A feature branch template".to_string()),
            created_at: 1_700_000_000,
            updated_at: 1_710_000_000,
            layout: "layouts/feature.json".to_string(),
        };

        let json = serde_json::to_value(&output).expect("serialize TemplateShowOutput");

        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("feature-branch")
        );
        assert_eq!(
            json.get("description").and_then(|v| v.as_str()),
            Some("A feature branch template")
        );
        assert_eq!(
            json.get("created_at").and_then(|v| v.as_i64()),
            Some(1_700_000_000)
        );
        assert_eq!(
            json.get("updated_at").and_then(|v| v.as_i64()),
            Some(1_710_000_000)
        );
        assert_eq!(
            json.get("layout").and_then(|v| v.as_str()),
            Some("layouts/feature.json")
        );
    }

    #[test]
    fn test_template_show_output_skips_none_description() {
        let output = TemplateShowOutput {
            name: "simple".to_string(),
            description: None,
            created_at: 0,
            updated_at: 0,
            layout: "layouts/simple.json".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        assert!(
            !json_str.contains("description"),
            "Should skip description when None: {json_str}"
        );
    }

    #[test]
    fn test_template_show_output_round_trip() {
        let output = TemplateShowOutput {
            name: "t1".to_string(),
            description: Some("desc".to_string()),
            created_at: 100,
            updated_at: 200,
            layout: "l.json".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: TemplateShowOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output, deserialized);
    }

    // ── TemplateDeleteOutput tests ─────────────────────────────────────

    #[test]
    fn test_template_delete_output_serialization() {
        let output = TemplateDeleteOutput {
            name: "old-template".to_string(),
            message: "Template deleted successfully".to_string(),
        };

        let json = serde_json::to_value(&output).expect("serialize TemplateDeleteOutput");

        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("old-template")
        );
        assert_eq!(
            json.get("message").and_then(|v| v.as_str()),
            Some("Template deleted successfully")
        );
    }

    #[test]
    fn test_template_delete_output_round_trip() {
        let output = TemplateDeleteOutput {
            name: "x".to_string(),
            message: "deleted".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: TemplateDeleteOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output, deserialized);
    }

    // ── ConfigValueOutput tests ────────────────────────────────────────

    #[test]
    fn test_config_value_output_serialization() {
        let output = ConfigValueOutput {
            key: "editor".to_string(),
            value: serde_json::json!("vim"),
        };

        let json = serde_json::to_value(&output).expect("serialize ConfigValueOutput");

        assert_eq!(
            json.get("key").and_then(|v| v.as_str()),
            Some("editor")
        );
        assert_eq!(json.get("value").and_then(|v| v.as_str()), Some("vim"));
    }

    #[test]
    fn test_config_value_output_with_complex_value() {
        let output = ConfigValueOutput {
            key: "aliases".to_string(),
            value: serde_json::json!({"s": "session", "d": "done"}),
        };

        let json = serde_json::to_value(&output).expect("serialize");

        let value = json.get("value").expect("value field");
        assert_eq!(value.get("s").and_then(|v| v.as_str()), Some("session"));
        assert_eq!(value.get("d").and_then(|v| v.as_str()), Some("done"));
    }

    #[test]
    fn test_config_value_output_round_trip() {
        let output = ConfigValueOutput {
            key: "k".to_string(),
            value: serde_json::json!("v"),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: ConfigValueOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output.key, deserialized.key);
        assert_eq!(output.value, deserialized.value);
    }

    // ── ConfigSetOutput tests ──────────────────────────────────────────

    #[test]
    fn test_config_set_output_serialization() {
        let output = ConfigSetOutput {
            key: "editor".to_string(),
            value: "nvim".to_string(),
            scope: "local".to_string(),
        };

        let json = serde_json::to_value(&output).expect("serialize ConfigSetOutput");

        assert_eq!(
            json.get("key").and_then(|v| v.as_str()),
            Some("editor")
        );
        assert_eq!(
            json.get("value").and_then(|v| v.as_str()),
            Some("nvim")
        );
        assert_eq!(
            json.get("scope").and_then(|v| v.as_str()),
            Some("local")
        );
    }

    #[test]
    fn test_config_set_output_round_trip() {
        let output = ConfigSetOutput {
            key: "auto_sync".to_string(),
            value: "true".to_string(),
            scope: "global".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");
        let deserialized: ConfigSetOutput =
            serde_json::from_str(&json_str).expect("deserialize round-trip");

        assert_eq!(output, deserialized);
    }
}
