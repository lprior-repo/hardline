//! VCS (Version Control System) abstraction for Git and JJ
//!
//! This module provides:
//! - `BackendType` - Enumeration distinguishing Git vs JJ repositories
//! - `RepositoryPath` - Absolute path to a version-controlled directory
//! - `BranchName` - Named reference to a line of development
//! - `CommitId` - Unique identifier for a commit
//! - `ChangeId` - Unique identifier for a VCS change/commit (Git SHA or JJ ID)
//! - `Change` - A single atomic modification in VCS history
//! - `RepoStatus` - Current state of the working directory
//! - `VcsBackend` - Unified trait for VCS operations
//! - `detect_backend` - Detect VCS type from filesystem
//!
//! # Module Structure
//! - `errors` - Error types (VcsError, ParseError, ChangeError)
//! - `types` - Core types (BackendType, RepositoryPath, BranchName, CommitId, ChangeId)
//! - `change` - Change and status types (Change, RepoStatus)
//! - `backend` - VcsBackend trait definition
//! - `detection` - Backend detection function
//! - `git` - Git backend implementation using gix (pure Rust)
//! (JJ backend is in infrastructure module)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

pub mod backend;
pub mod change;
pub mod detection;
pub mod errors;
pub mod git;
pub mod types;

// Re-export from errors module
pub use errors::{ChangeError, ParseError, VcsError};

// Re-export from types module
pub use types::{BackendType, BranchName, ChangeId, CommitId, RepositoryPath};

// Re-export from change module
pub use change::{Change, RepoStatus};

// Re-export from backend module
pub use backend::VcsBackend;

// Re-export from detection module
pub use detection::detect_backend;

// Re-export GitBackend and GitBackendConfig
pub use git::{GitBackend, GitBackendConfig};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_detect_backend_returns_git_for_git_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).expect("Failed to create .git dir");

        let result = detect_backend(temp_dir.path());

        assert_eq!(result, Ok(BackendType::Git));
    }

    #[test]
    fn test_detect_backend_returns_jj_for_jj_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let jj_dir = temp_dir.path().join(".jj");
        fs::create_dir(&jj_dir).expect("Failed to create .jj dir");

        let result = detect_backend(temp_dir.path());

        assert_eq!(result, Ok(BackendType::Jj));
    }

    #[test]
    fn test_detect_backend_prioritizes_jj_over_git() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let git_dir = temp_dir.path().join(".git");
        let jj_dir = temp_dir.path().join(".jj");
        fs::create_dir(&git_dir).expect("Failed to create .git dir");
        fs::create_dir(&jj_dir).expect("Failed to create .jj dir");

        let result = detect_backend(temp_dir.path());

        assert_eq!(result, Ok(BackendType::Jj));
    }

    #[test]
    fn test_repository_path_normalizes_relative_paths() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = RepositoryPath::new(temp_dir.path());

        assert!(result.is_ok());
        let repo_path = result.expect("Should have repo path");
        assert!(repo_path.as_path().is_absolute());
    }

    #[test]
    fn test_branch_name_accepts_valid_names() {
        let valid_names = ["main", "feature/my-feature", "fix-123", "release_v1.0"];

        for name in valid_names {
            let result = BranchName::new(name);

            assert!(result.is_ok(), "Expected '{name}' to be valid");
            let branch = result.expect("Should have branch");
            assert_eq!(branch.as_str(), name);
        }
    }

    #[test]
    fn test_commit_id_accepts_valid_ids() {
        let valid_ids = ["abc123", "a1b2c3d4e5f6", "deadbeef", "0123456789abcdef"];

        for id in valid_ids {
            let result = CommitId::new(id);

            assert!(result.is_ok(), "Expected '{id}' to be valid");
            let commit = result.expect("Should have commit id");
            assert_eq!(commit.as_str(), id);
        }
    }

    #[test]
    fn test_vcs_backend_trait_compiles_with_stub() {
        struct StubBackend {
            path: RepositoryPath,
        }

        impl VcsBackend for StubBackend {
            fn backend_type(&self) -> BackendType {
                BackendType::Git
            }

            fn path(&self) -> &RepositoryPath {
                &self.path
            }

            fn current_branch(&self) -> Result<Option<BranchName>, VcsError> {
                Ok(Some(BranchName::new("main").expect("valid branch")))
            }

            fn list_branches(&self) -> Result<Vec<BranchName>, VcsError> {
                Ok(vec![BranchName::new("main").expect("valid branch")])
            }

            fn status(&self) -> Result<RepoStatus, VcsError> {
                Ok(RepoStatus::default())
            }

            fn commit_exists(&self, _id: &CommitId) -> Result<bool, VcsError> {
                Ok(true)
            }

            fn sync(&self, _branch: &BranchName, _parent: &BranchName) -> Result<(), VcsError> {
                Ok(())
            }
        }

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = RepositoryPath::new(temp_dir.path()).expect("Valid path");
        let _backend = StubBackend { path: repo_path };
    }

    #[test]
    fn test_status_returns_repo_status() {
        let status = RepoStatus::default();

        assert!(!status.has_changes);
        assert_eq!(status.added, 0);
        assert_eq!(status.modified, 0);
        assert_eq!(status.deleted, 0);
        assert!(status.current_branch.is_none());
    }

    #[test]
    fn test_detect_backend_returns_no_vcs_found_outside_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = detect_backend(temp_dir.path());

        assert!(matches!(result, Err(VcsError::NoVcsFound(_))));
    }

    #[test]
    fn test_detect_backend_returns_path_not_found_for_nonexistent() {
        let nonexistent_path = "/nonexistent/path/xyz/12345";

        let result = detect_backend(nonexistent_path);

        assert!(matches!(result, Err(VcsError::PathNotFound(_))));
    }

    #[test]
    fn test_repository_path_rejects_non_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").expect("Failed to write file");

        let result = RepositoryPath::new(&file_path);

        assert!(matches!(result, Err(VcsError::PathNotDirectory(_))));
    }

    #[test]
    fn test_branch_name_rejects_empty_string() {
        let result = BranchName::new("");

        assert!(matches!(result, Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn test_branch_name_rejects_whitespace_only() {
        let result = BranchName::new("   ");

        assert!(matches!(result, Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn test_branch_name_rejects_double_dot_sequence() {
        let result = BranchName::new("feature/..");
        assert!(matches!(result, Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn test_branch_name_rejects_git_reserved_characters() {
        let result = BranchName::new("feature bad");
        assert!(matches!(result, Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn test_branch_name_rejects_single_at_symbol() {
        let result = BranchName::new("@");
        assert!(matches!(result, Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn test_commit_id_rejects_empty_string() {
        let result = CommitId::new("");

        assert!(matches!(result, Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn test_commit_id_rejects_whitespace_only() {
        let result = CommitId::new("   ");

        assert!(matches!(result, Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn test_detect_backend_works_with_bare_git_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let bare_git = temp_dir.path().join("repo.git");
        fs::create_dir(&bare_git).expect("Failed to create bare git dir");

        fs::write(bare_git.join("HEAD"), "ref: refs/heads/main\n").expect("Failed to write HEAD");

        let result = detect_backend(&bare_git);

        assert!(matches!(result, Err(VcsError::NoVcsFound(_))));
    }

    #[test]
    fn test_detect_backend_searches_parent_directories() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).expect("Failed to create .git dir");

        let subdir = temp_dir.path().join("src").join("lib");
        fs::create_dir_all(&subdir).expect("Failed to create subdir");

        let result = detect_backend(&subdir);

        assert_eq!(result, Ok(BackendType::Git));
    }

    #[test]
    fn test_repo_status_default_is_clean() {
        let status = RepoStatus::default();

        assert!(!status.has_changes);
    }

    #[test]
    fn test_backend_type_equality() {
        assert_eq!(BackendType::Git, BackendType::Git);
        assert_eq!(BackendType::Jj, BackendType::Jj);
        assert_ne!(BackendType::Git, BackendType::Jj);
    }

    #[test]
    fn test_branch_name_clone() {
        let branch = BranchName::new("main").expect("valid");
        let cloned = branch.clone();
        assert_eq!(branch, cloned);
    }

    #[test]
    fn test_commit_id_clone() {
        let commit = CommitId::new("abc123").expect("valid");
        let cloned = commit.clone();
        assert_eq!(commit, cloned);
    }

    #[test]
    fn test_repository_path_clone() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = RepositoryPath::new(temp_dir.path()).expect("valid");
        let cloned = path.clone();
        assert_eq!(path, cloned);
    }

    #[test]
    fn test_repository_path_new_unchecked() {
        let path = PathBuf::from("/some/path");
        let repo_path = RepositoryPath::new_unchecked(path.clone());
        assert_eq!(repo_path.as_path(), path);
    }

    #[test]
    fn test_repo_status_with_branch() {
        let branch = BranchName::new("develop").expect("valid");
        let status = RepoStatus {
            has_changes: true,
            added: 2,
            modified: 3,
            deleted: 1,
            current_branch: Some(branch),
        };

        assert!(status.has_changes);
        assert_eq!(status.added, 2);
        assert_eq!(status.modified, 3);
        assert_eq!(status.deleted, 1);
        assert!(status.current_branch.is_some());
        assert_eq!(
            status.current_branch.as_ref().map(BranchName::as_str),
            Some("develop")
        );
    }

    #[test]
    fn test_vcs_error_display() {
        let err = VcsError::PathNotFound(PathBuf::from("/test/path"));
        let msg = format!("{err}");
        assert!(msg.contains("/test/path"));

        let err = VcsError::InvalidBranchName("bad".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad"));
    }

    #[test]
    fn test_git_open_failed_error_display() {
        let err = VcsError::GitOpenFailed {
            path: PathBuf::from("/repo/path"),
            message: "something went wrong".to_string(),
            source: None,
        };
        let msg = format!("{err}");
        assert!(msg.contains("/repo/path"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn test_bare_repository_not_supported_error_display() {
        let err = VcsError::BareRepositoryNotSupported(PathBuf::from("/bare/repo.git"));
        let msg = format!("{err}");
        assert!(msg.contains("/bare/repo.git"));
        assert!(msg.contains("Bare repository"));
    }

    #[test]
    fn test_change_id_display_git() {
        let change_id = ChangeId::from_git_sha("abc123def").expect("valid");
        let msg = format!("{change_id}");
        assert!(msg.starts_with("git:"));
        assert!(msg.contains("abc123def"));
    }

    #[test]
    fn test_change_id_display_jj() {
        let change_id = ChangeId::from_jj_id("abc123").expect("valid");
        let msg = format!("{change_id}");
        assert!(msg.starts_with("jj:"));
        assert!(msg.contains("abc123"));
    }
}
