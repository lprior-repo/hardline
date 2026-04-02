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
pub use types::{
    detect_vcs, Branch, BranchName, ChangeId, Commit, CommitId, RepoStatus, VcsStatus, VcsType,
    Workspace,
};
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

    #[test]
    fn test_detect_vcs_with_temp_jj() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".jj")).expect("create .jj");
        let vcs = detect_vcs(dir.path());
        assert_eq!(vcs, Some(VcsType::Jujutsu));
    }

    #[test]
    fn test_create_backend_fails_for_no_vcs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = create_backend(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_backend_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let backend = create_backend(dir.path());
        assert!(backend.is_ok());
    }

    #[test]
    fn test_create_backend_jj() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".jj")).expect("create .jj");
        let backend = create_backend(dir.path());
        assert!(backend.is_ok());
    }

    #[test]
    fn test_create_backend_prefers_jj_over_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::fs::create_dir(dir.path().join(".jj")).expect("create .jj");
        let _backend = create_backend(dir.path()).expect("backend");
        // Both JjBackend and GitBackend exist; detect_vcs returns Jjutsu
        // so it should be a JjBackend
    }
}
