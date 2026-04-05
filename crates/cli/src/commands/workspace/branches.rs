//! Workspace branch commands
//!
//! Ported from workspace/branches.rs to branch handler
//! for proper Data->Calc->Actions architecture with validation
//! and protected branch check.
//!
//! CLI commands:
//!
//! ```text
//! scp workspace branch <name>                          # Create branch
//! scp workspace branch-delete <name> [--force]   # Delete branch
//! scp workspace branch-rename <old> <new> [--dry-run]  # Rename branch
//! ```

use scp_core::output::Output;
use scp_core::{Error, Result};

use crate::commands::handlers::branch;

/// List branches
pub fn branches() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = scp_core::vcs::create_backend(&cwd)?;
    let branch_list = backend.list_branches()?;

    if branch_list.is_empty() {
        Output::info("No branches found");
    } else {
        for b in &branch_list {
            let current = if b.is_current { " (current)" } else { "" };
            Output::info(&format!("  - {}{}", b.name, current));
        }
    }
    Ok(())
}

/// Create branch using handler
pub fn branch_create(name: &str) -> Result<()> {
    let options = branch::BranchCreateOptions {
        name: name.to_string(),
        dry_run: false,
    };
    branch::run_branch_create(&options)?;
    Ok(())
}

/// Delete branch using handler
pub fn branch_delete(name: &str) -> Result<()> {
    let options = branch::BranchDeleteOptions {
        name: name.to_string(),
        force: false,
        dry_run: false,
    };
    branch::run_branch_delete(&options)
}

/// Show current branch
pub fn branch_current() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = scp_core::vcs::create_backend(&cwd)?;
    let branch_name = backend.current_branch()?;
    Output::info(&format!("Current branch: {}", branch_name));
    Ok(())
}
