//! Workspace consistency verification
//!
//! Verifies that created workspaces have consistent operation graphs.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(unused)]

use std::path::Path;

use crate::error::{Error, Result};

use super::jj_path::get_jj_command;

/// Verify workspace has consistent operation graph
pub(super) async fn verify_workspace_consistency(name: &str, path: &Path) -> Result<()> {
    let output = get_jj_command()
        .args(["status"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| crate::error_jj::JjErrorKind::CommandError {
            operation: "verify workspace operation".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        let error_str = stderr.to_string();

        if error_str.contains("sibling of the working copy's operation")
            || error_str.contains("working copy")
            || error_str.contains("operation")
        {
            return Err(crate::error_jj::JjErrorKind::WorkspaceConflict {
                conflict_type: crate::error::JjConflictType::Stale,
                workspace_name: name.to_string(),
                msg: format!("Operation graph mismatch: {error_str}"),
                recovery_hint: format!(
                    "The workspace '{name}' was created but has an inconsistent operation graph.\n\n\
                     Recovery: Run 'jj workspace forget {name}' and retry creation.\n\n\
                     This error indicates concurrent workspace creation or repo state change."
                ),
            }
            .into());
        }

        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "verify workspace operation".to_string(),
            msg: error_str,
            is_not_found: false,
        }
        .into());
    }

    Ok(())
}
