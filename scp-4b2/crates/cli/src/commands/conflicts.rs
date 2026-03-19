//! Conflict resolution commands

use scp_core::vcs::{VcsBackend, VcsStatus};
use scp_core::{Error, Result};

/// Pure calculation: formats the human-readable status message
fn format_status_message(status: VcsStatus) -> &'static str {
    match status {
        VcsStatus::Conflicted => {
            "Working copy has conflicts. Run 'jj log' to see conflict details."
        }
        VcsStatus::Dirty => "Working copy has uncommitted changes.",
        VcsStatus::Clean => "No conflicts found",
        VcsStatus::Detached => "Working copy is in detached HEAD state",
    }
}

/// Boundary: retrieves the current working directory
fn get_current_dir() -> Result<std::path::PathBuf> {
    std::env::current_dir().map_err(|e| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get current directory: {e}"),
        ))
    })
}

/// Action: list conflicts in the working copy
pub fn list() -> Result<()> {
    let cwd = get_current_dir()?;
    let backend = scp_core::vcs::create_backend(&cwd)?;
    let status = backend.status()?;
    println!("{}", format_status_message(status));
    Ok(())
}

/// Action: resolve conflicts in the working copy
pub fn resolve(_files: Option<Vec<String>>) -> Result<()> {
    println!("Conflict resolution not yet implemented via API");
    println!("Use 'jj resolve' directly to resolve conflicts");
    Ok(())
}
