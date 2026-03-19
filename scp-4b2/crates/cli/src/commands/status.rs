//! Status command - shows current workspace/session status

use crate::commands::session as sess;
use scp_core::Result;

/// Show status (default: detailed)
pub fn run(short: bool) -> Result<()> {
    if short {
        short_status()
    } else {
        detailed_status()
    }
}

/// Map VCS status to Unicode symbol character
const fn vcs_status_to_symbol(status: scp_core::vcs::VcsStatus) -> &'static str {
    match status {
        scp_core::vcs::VcsStatus::Clean => "✓",
        scp_core::vcs::VcsStatus::Dirty => "◐",
        scp_core::vcs::VcsStatus::Conflicted => "✗",
        scp_core::vcs::VcsStatus::Detached => "⚙",
    }
}

/// Format short status line as "symbol branch cwd"
fn format_short_status(symbol: &str, branch: &str, cwd_display: impl std::fmt::Display) -> String {
    format!("{} {} {}", symbol, branch, cwd_display)
}

/// Short status - single line output
fn short_status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;
    let backend = scp_core::vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    println!(
        "{}",
        format_short_status(vcs_status_to_symbol(vcs_status), &branch, cwd.display())
    );
    Ok(())
}

/// Detailed status - full output
fn detailed_status() -> Result<()> {
    sess::status()?;
    Ok(())
}
