//! Unified VCS abstraction layer for Source Control Plane.
//!
//! Provides trait-based VCS backend supporting both JJ and Git.
//! Zero panic, zero unwrap - all operations return Result.

#[path = "trait_.rs"]
mod trait_;
#[path = "vcs_types.rs"]
mod types;
#[path = "vcs_git.rs"]
mod vcs_git;
#[path = "vcs_jj.rs"]
mod vcs_jj;

pub use trait_::VcsBackend;
pub use types::{detect_vcs, Branch, Commit, VcsStatus, VcsType, Workspace};
pub use vcs_git::GitBackend;
pub use vcs_jj::JjBackend;

use crate::error::{Error, Result};

/// Auto-detect and create appropriate VCS backend
pub fn create_backend(path: &std::path::Path) -> Result<Box<dyn VcsBackend>> {
    match detect_vcs(path) {
        Some(VcsType::Jujutsu) => Ok(Box::new(JjBackend::new(path.to_path_buf()))),
        Some(VcsType::Git) => Ok(Box::new(GitBackend::new(path.to_path_buf()))),
        None => Err(Error::vcs_not_initialized()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_detect_vcs() {
        let cwd = env::current_dir().unwrap();
        let vcs = detect_vcs(&cwd);
        println!("Detected VCS: {:?}", vcs);
    }
}
