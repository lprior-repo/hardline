//! Unified VCS abstraction layer for Source Control Plane.
//!
//! Provides trait-based VCS backend supporting both JJ and Git.
//! Zero panic, zero unwrap - all operations return Result.

use std::path::Path;

use crate::error::{Error, Result};

pub mod backend;
pub mod git;
pub mod jj;
pub mod types;

// Re-exports for convenience
pub use backend::VcsBackend;
pub use git::GitBackend;
pub use jj::JjBackend;
pub use types::{Branch, Commit, VcsStatus, VcsType, Workspace};

/// Detect which VCS is in use in a directory
pub fn detect_vcs(path: &Path) -> Option<VcsType> {
    if path.join(".jj").exists() {
        Some(VcsType::Jujutsu)
    } else if path.join(".git").exists() {
        Some(VcsType::Git)
    } else {
        None
    }
}

/// Auto-detect and create appropriate VCS backend
pub fn create_backend(path: &Path) -> Result<Box<dyn VcsBackend>> {
    match detect_vcs(path) {
        Some(VcsType::Jujutsu) => Ok(Box::new(JjBackend::new(path.to_path_buf()))),
        Some(VcsType::Git) => Ok(Box::new(GitBackend::new(path.to_path_buf()))),
        None => Err(Error::VcsNotInitialized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_detect_vcs() {
        // Test in a known directory
        let cwd = env::current_dir().expect("current_dir should work");
        let vcs = detect_vcs(&cwd);
        // May or may not be initialized depending on where we run
        println!("Detected VCS: {:?}", vcs);
    }
}
