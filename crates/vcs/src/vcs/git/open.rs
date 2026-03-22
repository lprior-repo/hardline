//! Git repository opening and initialization
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use gix::Repository;

use crate::vcs::{BackendType, BranchName, RepositoryPath, VcsError};

use super::types::{GitBackend, GitBackendConfig, MIN_GIT_VERSION};

impl GitBackend {
    /// Open a Git repository at the given path
    ///
    /// # Preconditions
    /// - P1: Path exists on filesystem
    /// - P2: Path is a directory
    /// - P3: Path is inside a Git repository
    /// - P4: Repository is NOT bare
    /// - P5: gix can open the repository
    ///
    /// # Postconditions
    /// - Q1: Returns `Ok(GitBackend)` with valid repo handle
    /// - Q12: `backend_type()` returns `BackendType::Git`
    /// - I1: Repository is non-bare
    /// - I6: Path is absolute and canonical
    ///
    /// # Errors
    /// - `VcsError::PathNotFound` if path doesn't exist
    /// - `VcsError::PathNotDirectory` if path is a file
    /// - `VcsError::NoVcsFound` if not a git repository
    /// - `VcsError::BareRepositoryNotSupported` if bare repo
    /// - `VcsError::GitOpenFailed` if gix fails to open
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VcsError> {
        Self::open_with_config(path, &GitBackendConfig::default())
    }

    /// Open with explicit configuration
    ///
    /// # Errors
    /// Same as [`open`](Self::open), plus:
    /// - `VcsError::GitCliVersionTooOld` if `verify_cli_version` is true and Git < 2.38
    pub fn open_with_config(
        path: impl AsRef<Path>,
        config: &GitBackendConfig,
    ) -> Result<Self, VcsError> {
        let path = path.as_ref();

        let repo_path = RepositoryPath::new(path)?;

        let repo = gix::discover(repo_path.as_path()).map_err(|e| VcsError::GitOpenFailed {
            path: repo_path.as_path().to_path_buf(),
            message: e.to_string(),
            source: None,
        })?;

        if repo.is_bare() {
            return Err(VcsError::BareRepositoryNotSupported(
                repo_path.as_path().to_path_buf(),
            ));
        }

        let workdir = repo.work_dir().ok_or_else(|| {
            VcsError::BareRepositoryNotSupported(repo_path.as_path().to_path_buf())
        })?;

        let canonical_path = RepositoryPath::new(workdir)?;

        let backend = Self {
            path: canonical_path,
            repo: Mutex::new(repo),
        };

        if config.verify_cli_version {
            backend.verify_cli_version()?;
        }

        Ok(backend)
    }

    /// Verify Git CLI version is 2.38+
    ///
    /// # Errors
    /// - `VcsError::CommandFailed` if git not found
    /// - `VcsError::GitCliVersionTooOld` if version < 2.38
    /// - `VcsError::GitParseError` if version parse fails
    pub fn verify_cli_version(&self) -> Result<String, VcsError> {
        let output =
            Command::new("git")
                .arg("--version")
                .output()
                .map_err(|e| VcsError::CommandFailed {
                    message: "Failed to execute git --version".to_string(),
                    source: Some(e),
                })?;

        if !output.status.success() {
            return Err(VcsError::CommandFailed {
                message: "git --version failed".to_string(),
                source: None,
            });
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version = super::helpers::parse_git_version(&version_output)?;

        if version < MIN_GIT_VERSION {
            return Err(VcsError::GitCliVersionTooOld {
                found: format!("{}.{}.0", version.0, version.1),
            });
        }

        Ok(format!("{}.{}.0", version.0, version.1))
    }
}
