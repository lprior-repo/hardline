//! Data types for the work command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

use scp_core::OutputFormat;

/// Options for the work command (parsed from CLI).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct WorkOptions {
    /// Session name to create/use.
    pub name: String,
    /// Bead ID to associate (optional).
    pub bead_id: Option<String>,
    /// Agent ID to register (optional, auto-generated if not provided).
    pub agent_id: Option<String>,
    /// Don't register as agent.
    pub no_agent: bool,
    /// Idempotent mode - succeed if session already exists.
    pub idempotent: bool,
    /// Dry run - don't actually create.
    pub dry_run: bool,
    /// Output format.
    pub format: OutputFormat,
}

/// Output for the work command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOutput {
    /// Session name.
    pub name: String,
    /// Workspace path.
    pub workspace_path: String,
    /// Whether this was a new session or existing.
    pub created: bool,
    /// Agent ID if registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Bead ID if specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_id: Option<String>,
    /// Environment variables set.
    pub env_vars: Vec<EnvVar>,
    /// Shell command to enter workspace (for non-interactive use).
    pub enter_command: String,
}

/// Environment variable set by work command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    /// Variable name.
    pub name: String,
    /// Variable value.
    pub value: String,
}

/// Build environment variables for the workspace.
///
/// Pure function: given session metadata, produces the list of env vars
/// that the work command would set.
#[must_use]
pub fn build_env_vars(
    name: &str,
    workspace_path: &str,
    agent_id: Option<&str>,
    bead_id: Option<&str>,
) -> Vec<EnvVar> {
    let base_vars = vec![
        EnvVar {
            name: "SCP_SESSION".to_string(),
            value: name.to_string(),
        },
        EnvVar {
            name: "SCP_WORKSPACE".to_string(),
            value: workspace_path.to_string(),
        },
        EnvVar {
            name: "SCP_ACTIVE".to_string(),
            value: "1".to_string(),
        },
    ];

    let agent_vars = agent_id
        .map(|agent| {
            vec![EnvVar {
                name: "SCP_AGENT_ID".to_string(),
                value: agent.to_string(),
            }]
        })
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let bead_vars = bead_id
        .map(|bead| {
            vec![EnvVar {
                name: "SCP_BEAD_ID".to_string(),
                value: bead.to_string(),
            }]
        })
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    [base_vars, agent_vars, bead_vars]
        .into_iter()
        .flatten()
        .collect()
}

/// Generate a short random ID from the current timestamp.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    // Use last 8 hex chars of timestamp (truncation intentional).
    format!("{:08x}", timestamp as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_output_serializes() {
        let output = WorkOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/.scp/workspaces/test-session".to_string(),
            created: true,
            agent_id: Some("agent-12345".to_string()),
            bead_id: Some("scp-abc12".to_string()),
            env_vars: vec![EnvVar {
                name: "SCP_SESSION".to_string(),
                value: "test-session".to_string(),
            }],
            enter_command: "cd /path/to/.scp/workspaces/test-session".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.expect("just checked is_ok");
        assert!(json_str.contains("\"name\":\"test-session\""));
        assert!(json_str.contains("\"created\":true"));
    }

    #[test]
    fn work_output_roundtrip() {
        let output = WorkOutput {
            name: "roundtrip".to_string(),
            workspace_path: "/tmp/ws".to_string(),
            created: false,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /tmp/ws".to_string(),
        };

        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: WorkOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "roundtrip");
        assert!(!deserialized.created);
    }

    #[test]
    fn build_env_vars_with_all() {
        let vars = build_env_vars("test", "/ws/test", Some("agent-1"), Some("bead-1"));

        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(names.contains(&"SCP_SESSION"));
        assert!(names.contains(&"SCP_WORKSPACE"));
        assert!(names.contains(&"SCP_ACTIVE"));
        assert!(names.contains(&"SCP_AGENT_ID"));
        assert!(names.contains(&"SCP_BEAD_ID"));
    }

    #[test]
    fn build_env_vars_without_agent_or_bead() {
        let vars = build_env_vars("test", "/ws/test", None, None);

        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(names.contains(&"SCP_SESSION"));
        assert!(names.contains(&"SCP_WORKSPACE"));
        assert!(names.contains(&"SCP_ACTIVE"));
        assert!(!names.contains(&"SCP_AGENT_ID"));
        assert!(!names.contains(&"SCP_BEAD_ID"));
    }

    #[test]
    fn build_env_vars_agent_only() {
        let vars = build_env_vars("s", "/p", Some("agent-x"), None);

        assert!(vars.iter().any(|v| v.name == "SCP_AGENT_ID"));
        assert!(!vars.iter().any(|v| v.name == "SCP_BEAD_ID"));
    }

    #[test]
    fn build_env_vars_bead_only() {
        let vars = build_env_vars("s", "/p", None, Some("bead-y"));

        assert!(!vars.iter().any(|v| v.name == "SCP_AGENT_ID"));
        assert!(vars.iter().any(|v| v.name == "SCP_BEAD_ID"));
    }

    #[test]
    fn generate_short_id_format() {
        let id = generate_short_id();

        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn work_options_defaults() {
        let options = WorkOptions {
            name: "test-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        };

        assert!(!options.no_agent);
        assert!(!options.idempotent);
        assert!(!options.dry_run);
    }

    #[test]
    fn work_output_created_flag_true() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(output.created);
    }

    #[test]
    fn work_output_created_flag_false() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: false,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(!output.created);
    }

    #[test]
    fn work_output_json_includes_all_fields() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: true,
            agent_id: Some("agent-1".to_string()),
            bead_id: Some("bead-1".to_string()),
            env_vars: vec![EnvVar {
                name: "SCP_SESSION".to_string(),
                value: "test".to_string(),
            }],
            enter_command: "cd /path".to_string(),
        };

        let json_str = serde_json::to_string(&output).expect("serialize");

        assert!(json_str.contains("name"));
        assert!(json_str.contains("workspace_path"));
        assert!(json_str.contains("created"));
        assert!(json_str.contains("agent_id"));
        assert!(json_str.contains("bead_id"));
        assert!(json_str.contains("env_vars"));
        assert!(json_str.contains("enter_command"));
    }

    #[test]
    fn env_var_serialization() {
        let env_var = EnvVar {
            name: "SCP_SESSION".to_string(),
            value: "my-session".to_string(),
        };
        let json = serde_json::to_string(&env_var).expect("serialize");
        assert!(json.contains("SCP_SESSION"));
        assert!(json.contains("my-session"));
    }

    #[test]
    fn enter_command_format() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/home/user/.scp/workspaces/test".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /home/user/.scp/workspaces/test".to_string(),
        };

        assert!(output.enter_command.starts_with("cd "));
        assert!(output.enter_command.contains(&output.workspace_path));
    }
}
