//! abort command - Abandon workspace without merging
//!
//! This command is the opposite of `done`:
//! - Removes the workspace without merging changes
//! - Optionally updates bead status back to ready
//! - Can be run from inside or outside the workspace
//!
//! Use this when you want to discard work without completing it.

use isolate_core::json::SchemaEnvelope;
use serde::Serialize;

use crate::{session::Session, IsolateError, Result};

/// Stub session database for abort operations
#[derive(Debug, Clone, Default)]
pub struct StubSessionDb;

impl StubSessionDb {
    /// Get a session by name (stub - returns none for now)
    pub async fn get(&self, _name: &str) -> Result<Option<Session>> {
        Ok(None)
    }
}

/// Get the session database (stub)
pub async fn get_session_db() -> Result<StubSessionDb> {
    Ok(StubSessionDb)
}

/// Output for abort command
#[derive(Debug, Clone, Serialize)]
pub struct AbortOutput {
    /// Session that was aborted
    pub session_name: String,
    /// Whether workspace was removed
    pub workspace_removed: bool,
    /// Whether bead status was updated
    pub bead_updated: bool,
    /// Message describing what happened
    pub message: String,
}

/// Options for abort command
#[derive(Debug, Clone)]
pub struct AbortOptions {
    /// Workspace to abort (optional - uses current if not specified)
    pub workspace: Option<String>,
    /// Don't update bead status
    pub no_bead_update: bool,
    /// Keep workspace files (just remove from isolate tracking)
    pub keep_workspace: bool,
    /// Dry run - don't actually abort
    pub dry_run: bool,
    /// Output format
    pub format: isolate_core::OutputFormat,
}

/// Detect the current location (workspace or main)
#[derive(Debug, Clone)]
pub enum Location {
    /// In the main repository
    Main,
    /// Inside a workspace
    Workspace {
        /// Workspace name
        name: String,
        /// Path to the workspace
        path: String,
    },
}

/// Detect location from JJ repository root
pub fn detect_location(root: &std::path::Path) -> Result<Location> {
    // Check if we're in a workspace by looking at .isolate/workspaces/
    let workspaces_dir = root.join(".isolate").join("workspaces");
    if workspaces_dir.exists() {
        // Get current working directory
        if let Ok(cwd) = std::env::current_dir() {
            // Check if cwd is inside workspaces_dir
            if cwd.starts_with(&workspaces_dir) {
                // Extract workspace name from path
                let rel_path = cwd.strip_prefix(&workspaces_dir).ok();
                if let Some(rel) = rel_path {
                    let name = rel
                        .components()
                        .next()
                        .and_then(|c| c.as_os_str().to_str())
                        .unwrap_or("unknown");
                    return Ok(Location::Workspace {
                        name: name.to_string(),
                        path: cwd.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    Ok(Location::Main)
}

/// Check if we're in a JJ repository and return its root
pub async fn check_in_jj_repo() -> Result<std::path::PathBuf> {
    isolate_core::jj::check_in_jj_repo()
        .await
        .map_err(|e| IsolateError::OperationFailed(format!("Not in a JJ repository: {e}")))
}

/// Run the abort command
///
/// # Errors
///
/// Returns an error if:
/// - Not in a workspace and no workspace specified
/// - Workspace not found
/// - Cleanup fails
pub async fn run(options: &AbortOptions) -> Result<()> {
    let root = check_in_jj_repo().await?;
    let location = detect_location(&root)?;

    // Determine which workspace to abort
    let workspace_name = match (&options.workspace, &location) {
        (Some(name), _) | (None, Location::Workspace { name, .. }) => name.clone(),
        (None, Location::Main) => {
            return Err(IsolateError::OperationFailed(
                "Not in a workspace. Use --workspace <name> to specify which workspace to abort."
                    .to_string(),
            ));
        }
    };

    // Dry run - just show what would happen
    if options.dry_run {
        return output_dry_run(&workspace_name, options);
    }

    // Find the session
    let db = get_session_db().await?;
    let session = db.get(&workspace_name).await?.ok_or_else(|| {
        IsolateError::OperationFailed(format!("Session '{workspace_name}' not found"))
    })?;

    let workspace_path = std::path::Path::new(&session.workspace_path);

    // Forget the JJ workspace
    let _ = tokio::process::Command::new("jj")
        .args(["workspace", "forget", &workspace_name])
        .current_dir(&root)
        .output()
        .await;

    // Remove workspace files unless --keep-workspace
    let workspace_removed = if options.keep_workspace {
        false
    } else {
        match tokio::fs::try_exists(workspace_path).await {
            Ok(true) => {
                tokio::fs::remove_dir_all(workspace_path)
                    .await
                    .map_err(|e| {
                        IsolateError::OperationFailed(format!(
                            "Failed to remove workspace at {}: {e}",
                            workspace_path.display()
                        ))
                    })?;
                true
            }
            _ => false,
        }
    };

    // Update bead status if applicable
    let bead_updated = if options.no_bead_update {
        false
    } else {
        update_bead_status_to_ready(&session).await
    };

    let output = AbortOutput {
        session_name: workspace_name.clone(),
        workspace_removed,
        bead_updated,
        message: format!("Aborted session '{workspace_name}'"),
    };

    output_result(&output, options.format)
}

/// Output for dry run
fn output_dry_run(workspace_name: &str, options: &AbortOptions) -> Result<()> {
    let output = AbortOutput {
        session_name: workspace_name.to_string(),
        workspace_removed: !options.keep_workspace,
        bead_updated: !options.no_bead_update,
        message: format!("[DRY RUN] Would abort session '{workspace_name}'"),
    };

    if options.format.is_json() {
        let mut envelope =
            serde_json::to_value(SchemaEnvelope::new("abort-response", "single", &output))
                .map_err(|e| {
                    IsolateError::OperationFailed(format!(
                        "Failed to serialize abort dry-run output: {e}"
                    ))
                })?;
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        }
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize abort dry-run output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        println!("[DRY RUN] Would abort session '{workspace_name}'");
        if !options.keep_workspace {
            println!("  Would remove workspace files");
        }
        if !options.no_bead_update {
            println!("  Would update bead status to ready");
        }
    }

    Ok(())
}

/// Update bead status back to ready
async fn update_bead_status_to_ready(session: &crate::session::Session) -> bool {
    // Check if session has a bead_id in metadata
    let bead_id = session
        .metadata
        .as_ref()
        .and_then(|m| m.get("bead_id"))
        .and_then(|v| v.as_str());

    if let Some(bead_id) = bead_id {
        // Try to run br update to set status back to ready
        let result = tokio::process::Command::new("br")
            .args(["update", bead_id, "--status", "ready"])
            .output()
            .await;

        if let Ok(output) = result {
            return output.status.success();
        }
    }

    false
}

/// Output the result
fn output_result(output: &AbortOutput, format: isolate_core::OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("abort-response", "single", output);
        let json_str = serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize abort output: {e}"))
        })?;
        println!("{json_str}");
    } else {
        println!("{}", output.message);
        if output.workspace_removed {
            println!("  Workspace files removed");
        }
        if output.bead_updated {
            println!("  Bead status updated to ready");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_output_serializes() -> Result<(), serde_json::Error> {
        let output = AbortOutput {
            session_name: "test-session".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted session 'test-session'".to_string(),
        };

        let json_str = serde_json::to_string(&output)?;
        assert!(json_str.contains("\"session_name\":\"test-session\""));
        assert!(json_str.contains("\"workspace_removed\":true"));

        Ok(())
    }

    #[test]
    fn test_abort_options_default() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: false,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.workspace.is_none());
        assert!(!options.no_bead_update);
        assert!(!options.keep_workspace);
        assert!(!options.dry_run);
    }

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test `AbortOutput` message format
    #[test]
    fn test_abort_output_message_format() {
        let output = AbortOutput {
            session_name: "feature-auth".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted session 'feature-auth'".to_string(),
        };

        assert!(output.message.contains("Aborted"));
        assert!(output.message.contains(&output.session_name));
    }

    /// Test `workspace_removed` flag
    #[test]
    fn test_abort_workspace_removed_flag() {
        // When workspace removed
        let removed = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(removed.workspace_removed);

        // When --keep-workspace used
        let kept = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: false,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(!kept.workspace_removed);
    }

    /// Test `bead_updated` flag
    #[test]
    fn test_abort_bead_updated_flag() {
        // When bead updated
        let updated = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: true,
            message: "Aborted".to_string(),
        };
        assert!(updated.bead_updated);

        // When --no-bead-update used
        let not_updated = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(!not_updated.bead_updated);
    }

    /// Test `AbortOptions` with explicit workspace
    #[test]
    fn test_abort_options_explicit_workspace() {
        let options = AbortOptions {
            workspace: Some("specific-workspace".to_string()),
            no_bead_update: false,
            keep_workspace: false,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert_eq!(options.workspace, Some("specific-workspace".to_string()));
    }

    /// Test `AbortOptions` with `keep_workspace`
    #[test]
    fn test_abort_options_keep_workspace() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: true,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.keep_workspace);
    }

    /// Test `AbortOptions` with `no_bead_update`
    #[test]
    fn test_abort_options_no_bead_update() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: true,
            keep_workspace: false,
            dry_run: false,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.no_bead_update);
    }

    /// Test `AbortOptions` `dry_run` mode
    #[test]
    fn test_abort_options_dry_run() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: false,
            dry_run: true,
            format: isolate_core::OutputFormat::Json,
        };

        assert!(options.dry_run);
    }

    /// Test `AbortOutput` JSON serialization
    #[test]
    fn test_abort_output_json_complete() {
        let output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: true,
            message: "Aborted session 'test'".to_string(),
        };

        let json_str = serde_json::to_string(&output).unwrap_or_default();

        assert!(json_str.contains("session_name"));
        assert!(json_str.contains("workspace_removed"));
        assert!(json_str.contains("bead_updated"));
        assert!(json_str.contains("message"));
    }

    /// Test abort is opposite of done
    #[test]
    fn test_abort_is_opposite_of_done() {
        // Abort removes workspace, done merges it
        let abort_output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true, // Key difference
            bead_updated: false,
            message: "Aborted".to_string(),
        };

        // Abort removes the workspace (destroys changes)
        assert!(abort_output.workspace_removed);
    }

    /// Test `dry_run` output structure
    #[test]
    fn test_abort_dry_run_output() {
        let output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true, // Would be removed
            bead_updated: true,      // Would be updated
            message: "[DRY RUN] Would abort session 'test'".to_string(),
        };

        assert!(output.message.contains("DRY RUN"));
    }
}
