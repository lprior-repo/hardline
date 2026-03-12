//! Status command - shows current workspace/session status

use crate::commands::session as sess;
use crate::commands::workspace as ws;
use scp_core::Result;

/// Show status (default: detailed)
pub fn run(short: bool) -> Result<()> {
    if short {
        short_status()
    } else {
        detailed_status()
    }
}

/// Short status - single line output
fn short_status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::Io(e))?;

    let backend = scp_core::vcs::create_backend(&cwd)?;

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    let status_char = match vcs_status {
        scp_core::vcs::VcsStatus::Clean => "✓",
        scp_core::vcs::VcsStatus::Dirty => "◐",
        scp_core::vcs::VcsStatus::Conflicted => "✗",
        scp_core::vcs::VcsStatus::Detached => "⚙",
    };

    println!("{} {} {}", status_char, branch, cwd.display());

    Ok(())
}

/// Detailed status - full output
fn detailed_status() -> Result<()> {
    sess::status()?;
    Ok(())
}
