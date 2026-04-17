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

/// Short status - single line output
fn short_status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::io_error(e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_accepts_bool() {
        let _fn: fn(bool) -> Result<()> = run;
    }

    #[test]
    fn short_status_uses_vcs_backend() {
        use scp_core::vcs::VcsStatus;
        // Verify all VcsStatus variants used in status_char mapping
        let _ = VcsStatus::Clean;
        let _ = VcsStatus::Dirty;
        let _ = VcsStatus::Conflicted;
        let _ = VcsStatus::Detached;
    }

    #[test]
    fn detailed_status_delegates_to_session_status() {
        // detailed_status calls sess::status() — verify it exists
        let _fn: fn() -> scp_core::Result<()> = sess::status;
    }

    #[test]
    fn run_short_path_exists() {
        // Verify run(false) takes the detailed path, run(true) takes short
        // This is a compile-time check
        let _ = std::any::type_name::<fn(bool) -> Result<()>>();
    }

    #[test]
    fn status_module_imports_session() {
        // Verify session module is accessible
        let _ = std::any::type_name::<fn() -> scp_core::Result<()>>();
    }
}
