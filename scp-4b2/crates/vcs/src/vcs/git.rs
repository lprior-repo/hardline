//! Git backend implementation using gix for read operations
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! This module provides:
//! - `GitBackend` - VCS backend implementation using gix (pure Rust)
//! - `GitBackendConfig` - Configuration for `GitBackend` creation
//!
//! # Design
//! - Uses gix for read operations (status, branches, commits)
//! - Uses Git CLI (2.38+) for rebase operations (--update-refs support)
//! - Caches the `gix::Repository` handle for performance
//! - Thread-safe for read operations via Mutex

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use gix::Repository;

use crate::vcs::{
    BackendType, BranchName, CommitId, RepoStatus, RepositoryPath, VcsBackend, VcsError,
};

/// Minimum required Git CLI version for rebase operations
const MIN_GIT_VERSION: (u32, u32) = (2, 38);

// ============================================================================
// GitBackend
// ============================================================================

/// Git backend implementation using gix for read operations
///
/// # Invariants
/// - Repository is always a non-bare Git repository
/// - Repository path is absolute and canonical
/// - `gix::Repository` is opened once and cached
/// - Thread-safe for read operations via Mutex
pub struct GitBackend {
    /// Absolute path to the repository root
    path: RepositoryPath,
    /// Cached gix repository handle (wrapped in Mutex for thread safety)
    repo: Mutex<Repository>,
}

/// Configuration for `GitBackend` creation
#[derive(Debug, Clone)]
pub struct GitBackendConfig {
    /// Verify Git CLI version on open (default: true)
    pub verify_cli_version: bool,
}

impl Default for GitBackendConfig {
    fn default() -> Self {
        Self {
            verify_cli_version: true,
        }
    }
}

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
        let version = parse_git_version(&version_output)?;

        if version < MIN_GIT_VERSION {
            return Err(VcsError::GitCliVersionTooOld {
                found: format!("{}.{}.0", version.0, version.1),
            });
        }

        Ok(format!("{}.{}.0", version.0, version.1))
    }
}

impl VcsBackend for GitBackend {
    /// Get the backend type
    ///
    /// # Postconditions
    /// - Q12: Always returns `BackendType::Git`
    fn backend_type(&self) -> BackendType {
        BackendType::Git
    }

    /// Get the repository path
    ///
    /// # Postconditions
    /// - I6: Returns absolute, canonical path
    fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Get the current branch name
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q2: Branch name has no `refs/heads/` prefix
    /// - Q3: Returns `None` for detached HEAD
    /// - Q3b: Returns `None` for unborn branch (empty repo)
    ///
    /// # Errors
    /// - `VcsError::GitReferenceError` if HEAD is unreadable (corrupt)
    fn current_branch(&self) -> Result<Option<BranchName>, VcsError> {
        let repo = self.repo.lock().map_err(|_| {
            VcsError::GitReferenceError("Failed to acquire repository lock".to_string())
        })?;

        let head = repo.head();

        match head {
            Ok(head) => {
                let reference = head.detach();
                if reference.is_empty() || reference.symbolic_target().is_some() {
                    return Ok(None);
                }
                let branch_name = reference.shorthand();
                branch_name
                    .map(|name| {
                        BranchName::new(name).map_err(|_| {
                            VcsError::GitReferenceError(format!("Invalid branch name: {name}"))
                        })
                    })
                    .transpose()
            }
            Err(e) => {
                if e.kind() == gix::reference::head::existing::ErrorKind::NotFound
                    || e.kind() == gix::reference::head::existing::ErrorKind::Unborn
                {
                    return Ok(None);
                }
                Err(VcsError::GitReferenceError(format!(
                    "Failed to read HEAD: {}",
                    e
                )))
            }
        }
    }

    /// List all local branches
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q4: Returns only local branches (refs/heads/*)
    /// - Q5: Branch names have no `refs/heads/` prefix
    ///
    /// # Errors
    /// - `VcsError::GitReferenceError` if references unreadable
    fn list_branches(&self) -> Result<Vec<BranchName>, VcsError> {
        let repo = self.repo.lock().map_err(|_| {
            VcsError::GitReferenceError("Failed to acquire repository lock".to_string())
        })?;

        let references = repo
            .references()
            .map_err(|e| VcsError::GitReferenceError(format!("Failed to list branches: {}", e)))?;

        let mut result = references
            .local_branches()
            .filter_map(|branch_result| {
                branch_result.ok().and_then(|branch| {
                    let name = branch.name().ok().flatten()?;
                    BranchName::new(name).ok()
                })
            })
            .collect::<Vec<_>>();

        result.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        Ok(result)
    }

    /// Get repository status
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    ///
    /// # Postconditions
    /// - Q6: Accurately reflects working directory state
    /// - Q7: `has_changes` is false when clean
    ///
    /// # Errors
    /// - `VcsError::GitOpenFailed` if status check fails
    fn status(&self) -> Result<RepoStatus, VcsError> {
        let (added, modified, deleted) = {
            let repo = self.repo.lock().map_err(|_| {
                VcsError::GitReferenceError("Failed to acquire repository lock".to_string())
            })?;

            let mut opts = gix::status::Options::default();
            opts.include_untracked(false)
                .include_ignored(false)
                .include_unmodified(false);

            let statuses = repo
                .status_files_with_index(opts, std::iter::empty::<&std::path::Path>())
                .map_err(|e| VcsError::GitOpenFailed {
                    path: self.path.as_path().to_path_buf(),
                    message: format!("Failed to get status: {}", e),
                    source: None,
                })?;

            let mut added = 0u32;
            let mut modified = 0u32;
            let mut deleted = 0u32;

            for entry in statuses {
                let entry = entry.map_err(|e| VcsError::GitOpenFailed {
                    path: self.path.as_path().to_path_buf(),
                    message: format!("Failed to read status entry: {}", e),
                    source: None,
                })?;
                let change = entry.index_to_worktree_entry();
                if let Some(change) = change {
                    match change.kind() {
                        gix::status::change::Kind::New => added += 1,
                        gix::status::change::Kind::Modified => modified += 1,
                        gix::status::change::Kind::Deleted => deleted += 1,
                        _ => {}
                    }
                }
            }

            (added, modified, deleted)
        };

        let has_changes = added > 0 || modified > 0 || deleted > 0;

        let current_branch = self.current_branch()?;

        Ok(RepoStatus {
            has_changes,
            added,
            modified,
            deleted,
            current_branch,
        })
    }

    /// Check if a commit exists
    ///
    /// # Preconditions
    /// - P5: Repository is open and valid
    /// - P8: Commit ID is not empty (validated by `CommitId`)
    ///
    /// # Postconditions
    /// - Q8: Returns `true` for valid commit
    /// - Q9: Returns `false` for non-existent commit
    /// - Q9b: Returns `false` for malformed/invalid revision specifiers
    ///
    /// # Errors
    /// - `VcsError::GitOpenFailed` if lookup fails due to repository corruption
    fn commit_exists(&self, id: &CommitId) -> Result<bool, VcsError> {
        let repo = self.repo.lock().map_err(|_| {
            VcsError::GitReferenceError("Failed to acquire repository lock".to_string())
        })?;

        let revision = id.as_str();

        match repo.rev_parse(revision) {
            Ok(obj) => {
                let is_commit = obj.kind == gix::object::Kind::Commit;
                Ok(is_commit)
            }
            Err(e) => {
                let is_not_found = e.to_string().contains("not found")
                    || e.to_string().contains("ambiguous")
                    || e.to_string().contains("invalid");
                if is_not_found {
                    Ok(false)
                } else {
                    Err(VcsError::GitOpenFailed {
                        path: self.path.as_path().to_path_buf(),
                        message: format!("Failed to lookup commit: {}", e),
                        source: None,
                    })
                }
            }
        }
    }

    /// Rebase the given branch onto its parent branch
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Errors
    /// Returns `VcsError` if the rebase fails.
    fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError> {
        use std::process::Command;

        self.is_clean().and_then(|clean| {
            if clean {
                Ok(clean)
            } else {
                Err(VcsError::DirtyWorkingDirectory)
            }
        })?;

        let branches = self.list_branches()?;
        let current = self.current_branch()?;

        let is_current_branch = current
            .as_ref()
            .map(|b| b.as_str() == branch.as_str())
            .unwrap_or(false);
        let branch_exists =
            is_current_branch || branches.iter().any(|b| b.as_str() == branch.as_str());

        branch_exists
            .then_some(())
            .ok_or_else(|| VcsError::NotFound {
                entity: "Branch",
                id: branch.as_str().to_string(),
            })?;

        let parent_exists =
            parent.as_str() == "trunk" || branches.iter().any(|b| b.as_str() == parent.as_str());

        parent_exists
            .then_some(())
            .ok_or_else(|| VcsError::NotFound {
                entity: "Parent branch",
                id: parent.as_str().to_string(),
            })?;

        let original_branch = current;

        let _checkout_result = Command::new("git")
            .args(["checkout", branch.as_str()])
            .current_dir(self.path.as_path())
            .output()
            .map_err(|e| VcsError::CommandFailed {
                message: format!("Failed to checkout branch '{}'", branch.as_str()),
                source: Some(e),
            })
            .and_then(|output| {
                output
                    .status
                    .success()
                    .then_some(())
                    .ok_or_else(|| VcsError::GitCliFailed {
                        command: format!("git checkout {}", branch.as_str()),
                        source: None,
                    })
            })?;

        let _rebase_result = Command::new("git")
            .args(["rebase", "--update-refs", parent.as_str()])
            .current_dir(self.path.as_path())
            .output()
            .map_err(|e| VcsError::CommandFailed {
                message: format!("Failed to rebase onto '{}'", parent.as_str()),
                source: Some(e),
            })
            .and_then(|output| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let is_up_to_date =
                    stderr.contains("Current branch") && stderr.contains("is up to date");
                (output.status.success() || is_up_to_date)
                    .then_some(())
                    .ok_or_else(|| VcsError::GitCliFailed {
                        command: format!("git rebase --update-refs {}", parent.as_str()),
                        source: None,
                    })
            })?;

        let _ = original_branch
            .filter(|orig| orig.as_str() != branch.as_str())
            .and_then(|orig| {
                Command::new("git")
                    .args(["checkout", orig.as_str()])
                    .current_dir(self.path.as_path())
                    .output()
                    .ok()
            });

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse Git version from output like "git version 2.43.0"
fn parse_git_version(output: &str) -> Result<(u32, u32), VcsError> {
    let output = output.trim();

    let parts: Vec<&str> = output.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(VcsError::GitParseError(format!(
            "Unexpected git version format: {output}"
        )));
    }

    let version_str = parts[2];

    let version_parts: Vec<&str> = version_str.split('.').collect();

    if version_parts.len() < 2 {
        return Err(VcsError::GitParseError(format!(
            "Invalid version number: {version_str}"
        )));
    }

    let major = version_parts[0].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid major version: {}", version_parts[0]))
    })?;

    let minor = version_parts[1].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid minor version: {}", version_parts[1]))
    })?;

    Ok((major, minor))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    fn create_test_repo() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().to_path_buf();

        let output = Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .expect("Failed to run git init");

        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .expect("Failed to configure git");

        (temp, path)
    }

    fn create_bare_repo() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().join("repo.git");

        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&path)
            .output()
            .expect("Failed to run git init --bare");

        assert!(
            output.status.success(),
            "git init --bare failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        (temp, path)
    }

    fn create_initial_commit(repo_path: &std::path::Path) -> String {
        let file = repo_path.join("README.md");
        fs::write(&file, "# Test Repository\n").expect("Failed to write file");

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["", "Initial commitcommit", "-m"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to get HEAD");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn test_open_git_repo_returns_ok() {
        let (_temp, path) = create_test_repo();

        let result = GitBackend::open(&path);

        assert!(result.is_ok());
    }

    #[test]
    fn test_open_returns_gitbackend_with_correct_path() {
        let (_temp, path) = create_test_repo();

        let backend = GitBackend::open(&path).expect("Should open");

        let backend_path = backend.path().as_path();
        assert!(backend_path.is_absolute());
    }

    #[test]
    fn test_backend_type_returns_git() {
        let (_temp, path) = create_test_repo();
        let backend = GitBackend::open(&path).expect("Should open");

        let backend_type = backend.backend_type();

        assert_eq!(backend_type, BackendType::Git);
    }

    #[test]
    fn test_path_returns_absolute_canonical_path() {
        let (_temp, path) = create_test_repo();
        let backend = GitBackend::open(&path).expect("Should open");

        let repo_path = backend.path();

        assert!(repo_path.as_path().is_absolute());
        let path_str = repo_path.as_path().to_string_lossy();
        assert!(!path_str.contains("/./"));
        assert!(!path_str.contains("/../"));
    }

    #[test]
    fn test_open_from_subdirectory_finds_repo_root() {
        let (_temp, path) = create_test_repo();
        let subdir = path.join("src").join("lib");
        fs::create_dir_all(&subdir).expect("Failed to create subdir");

        let result = GitBackend::open(&subdir);

        assert!(result.is_ok());
    }

    #[test]
    fn test_current_branch_on_main_returns_main() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.current_branch();

        assert!(result.is_ok());
        let branch = result.expect("Should have branch");
        assert!(branch.is_some());
    }

    #[test]
    fn test_current_branch_name_has_no_refs_prefix() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let branch = backend.current_branch().expect("Should work");

        if let Some(name) = branch {
            assert!(!name.as_str().starts_with("refs/heads/"));
        }
    }

    #[test]
    fn test_current_branch_on_branch_with_slash_works() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        Command::new("git")
            .args(["checkout", "-b", "feature/test-branch"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        let backend = GitBackend::open(&path).expect("Should open");

        let branch = backend.current_branch().expect("Should work");

        assert!(branch.is_some());
        let name = branch.expect("Should have branch");
        assert_eq!(name.as_str(), "feature/test-branch");
    }

    #[test]
    fn test_current_branch_detached_head_returns_none() {
        let (_temp, path) = create_test_repo();
        let sha = create_initial_commit(&path);

        Command::new("git")
            .args(["checkout", &sha])
            .current_dir(&path)
            .output()
            .expect("Failed to checkout commit");

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.current_branch();

        assert!(result.is_ok());
        assert!(result.expect("Should have result").is_none());
    }

    #[test]
    fn test_list_branches_returns_all_local_branches() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        Command::new("git")
            .args(["branch", "develop"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        Command::new("git")
            .args(["branch", "feature/a"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        let backend = GitBackend::open(&path).expect("Should open");

        let branches = backend.list_branches().expect("Should work");

        assert!(branches.len() >= 3);
    }

    #[test]
    fn test_list_branches_names_have_no_refs_prefix() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let branches = backend.list_branches().expect("Should work");

        for branch in &branches {
            assert!(!branch.as_str().starts_with("refs/heads/"));
        }
    }

    #[test]
    fn test_status_clean_repo_returns_has_changes_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let status = backend.status().expect("Should work");

        assert!(!status.has_changes);
    }

    #[test]
    fn test_status_modified_file_has_changes_true() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let file = path.join("README.md");
        fs::write(&file, "# Modified content\n").expect("Failed to modify file");

        let backend = GitBackend::open(&path).expect("Should open");

        let status = backend.status().expect("Should work");

        assert!(status.has_changes);
        assert!(status.modified > 0);
    }

    #[test]
    fn test_commit_exists_valid_sha_returns_true() {
        let (_temp, path) = create_test_repo();
        let sha = create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new(&sha).expect("Valid commit ID");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(result.expect("Should have result"));
    }

    #[test]
    fn test_commit_exists_nonexistent_sha_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new("deadbeef12345678901234567890123456789012")
            .expect("Valid commit ID format");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_commit_exists_invalid_sha_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new("not-a-valid-ref").expect("Valid string");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_is_clean_clean_repo_returns_true() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.is_clean();

        assert!(result.is_ok());
        assert!(result.expect("Should be clean"));
    }

    #[test]
    fn test_is_clean_with_modified_file_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let file = path.join("README.md");
        fs::write(&file, "# Modified\n").expect("Failed to modify");

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.is_clean();

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_verify_cli_version_returns_version_string() {
        let (_temp, path) = create_test_repo();

        let config = GitBackendConfig {
            verify_cli_version: false,
        };
        let backend = GitBackend::open_with_config(&path, &config).expect("Should open");

        let result = backend.verify_cli_version();

        assert!(result.is_ok());
        let version = result.expect("Should have version");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_open_nonexistent_path_returns_path_not_found() {
        let nonexistent = "/nonexistent/path/xyz/test";

        let result = GitBackend::open(nonexistent);

        assert!(matches!(result, Err(VcsError::PathNotFound(_))));
    }

    #[test]
    fn test_open_file_path_returns_path_not_directory() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").expect("Failed to write file");

        let result = GitBackend::open(&file_path);

        assert!(matches!(result, Err(VcsError::PathNotDirectory(_))));
    }

    #[test]
    fn test_open_non_git_directory_returns_git_open_failed() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        let result = GitBackend::open(temp.path());

        assert!(matches!(result, Err(VcsError::GitOpenFailed { .. })));
    }

    #[test]
    fn test_open_bare_repo_returns_bare_repository_not_supported() {
        let (_temp, path) = create_bare_repo();

        let result = GitBackend::open(&path);

        match result {
            Err(VcsError::BareRepositoryNotSupported(p)) => {
                assert_eq!(p, path);
            }
            Err(e) => panic!("Wrong error type: {e:?}"),
            Ok(_) => panic!("Should have returned error"),
        }
    }

    #[test]
    fn test_parse_git_version_standard() {
        let output = "git version 2.43.0";
        let result = parse_git_version(output);
        assert!(result.is_ok());
        assert_eq!(result.expect("Should parse"), (2, 43));
    }

    #[test]
    fn test_parse_git_version_with_windows_suffix() {
        let output = "git version 2.43.0.windows.1";
        let result = parse_git_version(output);
        assert!(result.is_ok());
        assert_eq!(result.expect("Should parse"), (2, 43));
    }

    #[test]
    fn test_parse_git_version_invalid_format() {
        let output = "invalid output";
        let result = parse_git_version(output);
        assert!(matches!(result, Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn test_gitbackend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GitBackend>();
    }

    #[test]
    fn test_git_backend_config_default() {
        let config = GitBackendConfig::default();
        assert!(config.verify_cli_version);
    }

    #[test]
    fn test_open_with_config_skip_version_check() {
        let (_temp, path) = create_test_repo();

        let config = GitBackendConfig {
            verify_cli_version: false,
        };

        let result = GitBackend::open_with_config(&path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_snapshot_commit_message() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Add initial file"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["log", "--format=%B", "-1"])
            .current_dir(&path)
            .output()
            .expect("Failed to git log");

        let message = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("commit_message_single", message);
    }

    #[test]
    fn test_snapshot_commit_message_multiline() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args([
                "commit",
                "-m",
                "Add feature\n\nThis is the body of the commit.\nWith multiple lines.",
            ])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["log", "--format=%B", "-1"])
            .current_dir(&path)
            .output()
            .expect("Failed to git log");

        let message = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("commit_message_multiline", message);
    }

    #[test]
    fn test_snapshot_diff_single_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        fs::write(path.join("file.txt"), "modified content\n").expect("Failed to modify file");

        let output = Command::new("git")
            .args(["diff"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_single_file", diff);
    }

    #[test]
    fn test_snapshot_diff_binary_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        fs::write(path.join("binary.bin"), b"\x00\x01\x02\x03").expect("Failed to write binary");

        let output = Command::new("git")
            .args(["diff", "--binary"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_binary_file", diff);
    }

    #[test]
    fn test_snapshot_tree_output() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        fs::create_dir(path.join("subdir")).expect("Failed to create dir");
        fs::write(path.join("subdir", "nested.txt"), "nested\n").expect("Failed to write nested");

        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Add files and dirs"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["ls-tree", "-R", "HEAD"])
            .current_dir(&path)
            .output()
            .expect("Failed to git ls-tree");

        let tree = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("tree_output", tree);
    }

    #[test]
    fn test_snapshot_diff_with_renamed_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("old.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        Command::new("git")
            .args(["mv", "old.txt", "new.txt"])
            .current_dir(&path)
            .output()
            .expect("Failed to git mv");

        let output = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_renamed_file", diff);
    }
}
