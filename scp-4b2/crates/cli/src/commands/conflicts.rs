//! Conflict resolution commands

use scp_core::{Error, Result};

pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get current directory: {e}"),
        ))
    })?;

    let backend = scp_core::vcs::create_backend(&cwd)?;
    let status = backend.status()?;

    match status {
        scp_core::vcs::VcsStatus::Conflicted => {
            println!("Working copy has conflicts. Run 'jj log' to see conflict details.");
        }
        scp_core::vcs::VcsStatus::Dirty => {
            println!("Working copy has uncommitted changes.");
        }
        scp_core::vcs::VcsStatus::Clean => {
            println!("No conflicts found");
        }
        scp_core::vcs::VcsStatus::Detached => {
            println!("Working copy is in detached HEAD state");
        }
    }

    Ok(())
}

pub fn resolve(_files: Option<Vec<String>>) -> Result<()> {
    println!("Conflict resolution not yet implemented via API");
    println!("Use 'jj resolve' directly to resolve conflicts");
    Ok(())
}
