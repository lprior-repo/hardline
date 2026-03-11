//! VCS (Version Control System) abstraction for Git and JJ
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

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
//! - `git` - Git backend implementation using git2
//! - `jj` - JJ backend implementation using jj-lib

pub mod git;
pub mod jj;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export GitBackend and GitBackendConfig
pub use git::{GitBackend, GitBackendConfig};

// Re-export JjBackend and JjBackendConfig
pub use jj::{JjBackend, JjBackendConfig, RebaseStats};

// ============================================================================
// Error Types
// ============================================================================

/// VCS-specific errors
#[derive(Debug, Error)]
pub enum VcsError {
    /// Path does not exist on filesystem
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    /// Path exists but is not a directory
    #[error("Path is not a directory: {0}")]
    PathNotDirectory(PathBuf),

    /// No VCS backend detected (neither .git nor .jj found)
    #[error("No VCS backend found at path: {0}")]
    NoVcsFound(PathBuf),

    /// Invalid branch name (empty or contains illegal characters)
    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),

    /// Invalid commit ID (empty or malformed)
    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    /// Requested backend type is not supported
    #[error("Backend type not supported: {0:?}")]
    BackendNotSupported(BackendType),

    /// VCS command execution failed
    #[error("VCS command failed: {message}")]
    CommandFailed {
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<std::io::Error>,
    },

    /// Repository is in an invalid state
    #[error("Repository is in invalid state: {0}")]
    InvalidState(String),

    /// Operation requires a clean working directory
    #[error("Working directory has uncommitted changes")]
    DirtyWorkingDirectory,

    /// Branch or commit not found
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity type (e.g., "Branch", "Commit")
        entity: &'static str,
        /// Entity identifier
        id: String,
    },

    /// Failed to open Git repository
    #[error("Failed to open Git repository at {path}: {message}")]
    GitOpenFailed {
        /// Path to the repository
        path: PathBuf,
        /// Error message
        message: String,
        /// Source error from git2
        #[source]
        source: Option<git2::Error>,
    },

    /// Repository is bare (no working tree) - stacking requires working tree
    #[error("Bare repository not supported: {0}")]
    BareRepositoryNotSupported(PathBuf),

    /// Git reference operation failed
    #[error("Git reference error: {0}")]
    GitReferenceError(String),

    /// Git CLI command failed (for rebase operations)
    #[error("Git CLI command failed: {command}")]
    GitCliFailed {
        /// The command that failed
        command: String,
        /// Source error
        #[source]
        source: Option<std::io::Error>,
    },

    /// Git CLI version too old
    #[error("Git CLI version too old: {found}, requires 2.38+")]
    GitCliVersionTooOld {
        /// The version that was found
        found: String,
    },

    /// Failed to parse Git CLI output
    #[error("Failed to parse Git output: {0}")]
    GitParseError(String),

    /// Failed to open JJ workspace
    #[error("Failed to open JJ workspace at {path}: {message}")]
    JjOpenFailed {
        /// Path to the workspace
        path: PathBuf,
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Path is not a JJ workspace
    #[error("Not a JJ workspace: {0}")]
    NotAWorkspace(PathBuf),

    /// Invalid change ID format
    #[error("Invalid JJ change ID: {0}")]
    InvalidChangeId(String),

    /// Change not found in JJ workspace
    #[error("JJ change not found: {id}")]
    ChangeNotFound {
        /// Change ID
        id: String,
    },

    /// Ambiguous change ID
    #[error("Ambiguous JJ change ID: {id}")]
    AmbiguousChangeId {
        /// Change ID
        id: String,
    },

    /// Bookmark not found
    #[error("JJ bookmark not found: {name}")]
    BookmarkNotFound {
        /// Bookmark name
        name: String,
    },

    /// Bookmark already exists
    #[error("JJ bookmark already exists: {name}")]
    BookmarkAlreadyExists {
        /// Bookmark name
        name: String,
    },

    /// Failed to acquire workspace lock
    #[error("Failed to acquire JJ workspace lock: {0}")]
    LockAcquisitionFailed(String),

    /// Rebase operation failed
    #[error("JJ rebase operation failed: {message}")]
    RebaseFailed {
        /// Error message
        message: String,
        /// Source error
        #[source]
        source: Option<anyhow::Error>,
    },

    /// JJ internal error
    #[error("JJ internal error: {0}")]
    JjInternalError(#[source] anyhow::Error),
}

impl PartialEq for VcsError {
    #[allow(clippy::match_same_arms)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PathNotFound(a), Self::PathNotFound(b))
            | (Self::PathNotDirectory(a), Self::PathNotDirectory(b))
            | (Self::NoVcsFound(a), Self::NoVcsFound(b))
            | (Self::BareRepositoryNotSupported(a), Self::BareRepositoryNotSupported(b)) => a == b,
            (Self::InvalidBranchName(a), Self::InvalidBranchName(b))
            | (Self::InvalidCommitId(a), Self::InvalidCommitId(b))
            | (Self::InvalidState(a), Self::InvalidState(b))
            | (Self::GitReferenceError(a), Self::GitReferenceError(b))
            | (Self::GitParseError(a), Self::GitParseError(b)) => a == b,
            (Self::BackendNotSupported(a), Self::BackendNotSupported(b)) => a == b,
            (Self::CommandFailed { message: a, .. }, Self::CommandFailed { message: b, .. }) => {
                a == b
            }
            (
                Self::GitOpenFailed {
                    path: p1,
                    message: m1,
                    ..
                },
                Self::GitOpenFailed {
                    path: p2,
                    message: m2,
                    ..
                },
            ) => p1 == p2 && m1 == m2,
            (Self::GitCliFailed { command: a, .. }, Self::GitCliFailed { command: b, .. }) => {
                a == b
            }
            (Self::GitCliVersionTooOld { found: a }, Self::GitCliVersionTooOld { found: b }) => {
                a == b
            }
            (Self::DirtyWorkingDirectory, Self::DirtyWorkingDirectory) => true,
            (Self::NotFound { entity: a1, id: a2 }, Self::NotFound { entity: b1, id: b2 }) => {
                a1 == b1 && a2 == b2
            }
            (Self::NotAWorkspace(a), Self::NotAWorkspace(b)) => a == b,
            (Self::InvalidChangeId(a), Self::InvalidChangeId(b))
            | (Self::LockAcquisitionFailed(a), Self::LockAcquisitionFailed(b)) => a == b,
            (
                Self::JjOpenFailed {
                    path: p1,
                    message: m1,
                    ..
                },
                Self::JjOpenFailed {
                    path: p2,
                    message: m2,
                    ..
                },
            ) => p1 == p2 && m1 == m2,
            (Self::ChangeNotFound { id: a }, Self::ChangeNotFound { id: b }) => a == b,
            (Self::AmbiguousChangeId { id: a }, Self::AmbiguousChangeId { id: b }) => a == b,
            (Self::BookmarkNotFound { name: a }, Self::BookmarkNotFound { name: b }) => a == b,
            (Self::BookmarkAlreadyExists { name: a }, Self::BookmarkAlreadyExists { name: b }) => {
                a == b
            }
            (Self::RebaseFailed { message: a, .. }, Self::RebaseFailed { message: b, .. }) => {
                a == b
            }
            (Self::JjInternalError(a), Self::JjInternalError(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

/// Errors when parsing `ChangeId` from string
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Input string is empty or whitespace-only
    #[error("ChangeId cannot be empty")]
    Empty,

    /// Input contains invalid characters for the format
    #[error("Invalid characters in ChangeId: {0}")]
    InvalidCharacters(String),

    /// Git SHA has invalid length (expected 7-40 characters)
    #[error("Invalid Git SHA length: {0} characters")]
    InvalidGitShaLength(usize),

    /// JJ change ID has invalid length (expected >= 1)
    #[error("Invalid JJ change ID length: {0} characters")]
    InvalidJjLength(usize),
}

/// Errors when creating or manipulating Change
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChangeError {
    /// Message cannot be empty
    #[error("Change message cannot be empty")]
    EmptyMessage,

    /// Author cannot be empty
    #[error("Change author cannot be empty")]
    EmptyAuthor,
}

// ============================================================================
// Type Definitions
// ============================================================================

/// Version control system backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    /// Git repository (contains .git directory)
    Git,
    /// Jujutsu repository (contains .jj directory)
    Jj,
}

/// Absolute path to a VCS repository
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryPath(PathBuf);

impl RepositoryPath {
    /// Create from any path (converts to absolute)
    ///
    /// # Errors
    /// - `VcsError::PathNotFound` if path does not exist
    /// - `VcsError::PathNotDirectory` if path is not a directory
    pub fn new(path: impl AsRef<Path>) -> Result<Self, VcsError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(VcsError::PathNotFound(path.to_path_buf()));
        }

        if !path.is_dir() {
            return Err(VcsError::PathNotDirectory(path.to_path_buf()));
        }

        let canonical = path.canonicalize().map_err(|e| VcsError::CommandFailed {
            message: format!("Failed to canonicalize path: {}", path.display()),
            source: Some(e),
        })?;

        Ok(Self(canonical))
    }

    /// Create without validation (for testing only)
    #[must_use]
    pub const fn new_unchecked(path: PathBuf) -> Self {
        Self(path)
    }

    /// Get the path as a reference
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Name of a branch in the VCS
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

fn is_invisible_char(c: char) -> bool {
    matches!(
        c,
        '\u{FEFF}'
            | '\u{200B}'
            | '\u{200C}'
            | '\u{200D}'
            | '\u{2060}'
            | '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{115F}'
            | '\u{1160}'
    ) || is_in_range(c, '\u{2061}', '\u{2064}')
        || is_in_range(c, '\u{206A}', '\u{206F}')
        || is_in_range(c, '\u{17B4}', '\u{17B5}')
        || is_in_range(c, '\u{202A}', '\u{202E}')
        || is_in_range(c, '\u{2066}', '\u{2069}')
}

fn is_in_range(c: char, start: char, end: char) -> bool {
    c >= start && c <= end
}

fn is_effectively_empty(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.trim().is_empty() {
        return true;
    }

    s.chars().all(|c| c.is_whitespace() || is_invisible_char(c))
}

fn has_invalid_branch_syntax(name: &str) -> bool {
    if name == "@" {
        return true;
    }

    if name.starts_with('/') || name.ends_with('/') || name.ends_with('.') {
        return true;
    }

    if name.contains("..")
        || name.contains("@{")
        || std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
    {
        return true;
    }

    if name.chars().any(|char| {
        char.is_control() || matches!(char, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return true;
    }

    name.split('/').any(str::is_empty)
}

impl BranchName {
    /// Create a new branch name with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidBranchName` if name is empty, whitespace-only, or contains only invisible characters
    pub fn new(name: impl Into<String>) -> Result<Self, VcsError> {
        let name = name.into();

        if is_effectively_empty(&name) || has_invalid_branch_syntax(&name) {
            return Err(VcsError::InvalidBranchName(name));
        }

        Ok(Self(name))
    }

    /// Get the branch name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for a commit
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new commit ID with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidCommitId` if ID is empty, whitespace-only, or contains only invisible characters
    pub fn new(id: impl Into<String>) -> Result<Self, VcsError> {
        let id = id.into();

        if is_effectively_empty(&id) {
            return Err(VcsError::InvalidCommitId(id));
        }

        Ok(Self(id))
    }

    /// Get the commit ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for a VCS change/commit
///
/// # Invariants
/// - Always contains a non-empty, trimmed ID string
/// - Git SHAs are lowercase hex
/// - JJ change IDs are lowercase base36
/// - Backend type is encoded to prevent cross-backend comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId {
    inner: ChangeIdInner,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ChangeIdInner {
    /// Git commit SHA (7-40 lowercase hex chars)
    Git { sha: String },
    /// JJ change ID (lowercase base36)
    Jj { id: String },
}

impl ChangeId {
    /// Create a Git `ChangeId` from a SHA string
    ///
    /// # Preconditions
    /// - P1: `sha` is not empty
    /// - P2: `sha` contains only hex characters (0-9, a-f, A-F)
    /// - P4: `sha` length is 7-40 characters
    ///
    /// # Postconditions
    /// - Q4: SHA is normalized to lowercase
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-hex chars present
    /// - `ParseError::InvalidGitShaLength` if length invalid
    pub fn from_git_sha(sha: impl AsRef<str>) -> Result<Self, ParseError> {
        let sha = sha.as_ref().trim();

        if is_effectively_empty(sha) {
            return Err(ParseError::Empty);
        }

        let len = sha.len();
        if !(7..=40).contains(&len) {
            return Err(ParseError::InvalidGitShaLength(len));
        }

        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseError::InvalidCharacters(sha.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Git {
                sha: sha.to_lowercase(),
            },
        })
    }

    /// Create a JJ `ChangeId` from a change ID string
    ///
    /// # Preconditions
    /// - P1: `id` is not empty
    /// - P3: `id` contains only base36 characters (0-9, a-z)
    /// - P5: `id` length is >= 1
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-base36 chars present
    /// - `ParseError::InvalidJjLength` if length is 0
    pub fn from_jj_id(id: impl AsRef<str>) -> Result<Self, ParseError> {
        let id = id.as_ref().trim();

        if is_effectively_empty(id) {
            return Err(ParseError::Empty);
        }

        let len = id.len();
        if len == 0 {
            return Err(ParseError::InvalidJjLength(len));
        }

        let normalized = id.to_lowercase();
        if !normalized
            .chars()
            .all(|c: char| c.is_ascii_digit() || c.is_ascii_lowercase())
        {
            return Err(ParseError::InvalidCharacters(id.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Jj { id: normalized },
        })
    }

    /// Get the backend type for this `ChangeId`
    ///
    /// # Postconditions
    /// - Q3: Returns correct `BackendType`
    #[must_use]
    pub fn backend_type(&self) -> BackendType {
        match &self.inner {
            ChangeIdInner::Git { .. } => BackendType::Git,
            ChangeIdInner::Jj { .. } => BackendType::Jj,
        }
    }

    /// Get the ID as a string slice (without backend prefix)
    ///
    /// # Postconditions
    /// - Q2: Returns inner ID only
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            ChangeIdInner::Git { sha } => sha,
            ChangeIdInner::Jj { id } => id,
        }
    }
}

impl std::str::FromStr for ChangeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();

        if is_effectively_empty(trimmed) {
            return Err(ParseError::Empty);
        }

        let is_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());

        if is_hex {
            Self::from_git_sha(trimmed)
        } else {
            Self::from_jj_id(trimmed)
        }
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ChangeIdInner::Git { sha } => write!(f, "git:{sha}"),
            ChangeIdInner::Jj { id } => write!(f, "jj:{id}"),
        }
    }
}

/// A single atomic change in VCS history
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// Unique identifier for this change
    id: ChangeId,
    /// Commit message (first line / summary)
    message: String,
    /// Author of the change (e.g., "Alice <alice@example.com>")
    author: String,
    /// Timestamp when the change was created
    timestamp: DateTime<Utc>,
}

impl Change {
    /// Create a new Change with validation
    ///
    /// # Preconditions
    /// - P6: `message` is not empty (after trimming)
    /// - P7: `author` is not empty (after trimming)
    ///
    /// # Postconditions
    /// - Q5: All fields populated
    /// - I5: Message is trimmed
    ///
    /// # Errors
    /// - `ChangeError::EmptyMessage` if message is empty
    /// - `ChangeError::EmptyAuthor` if author is empty
    pub fn new(
        id: ChangeId,
        message: impl Into<String>,
        author: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, ChangeError> {
        let message = message.into();
        let author = author.into();

        if is_effectively_empty(&message) {
            return Err(ChangeError::EmptyMessage);
        }

        if is_effectively_empty(&author) {
            return Err(ChangeError::EmptyAuthor);
        }

        Ok(Self {
            id,
            message: message.trim().to_string(),
            author: author.trim().to_string(),
            timestamp,
        })
    }

    /// Get a reference to the change ID
    #[must_use]
    pub fn id(&self) -> &ChangeId {
        &self.id
    }

    /// Get the commit message
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the author string
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Get the timestamp
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// Status of a repository working directory
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStatus {
    /// Whether the working directory has uncommitted changes
    pub has_changes: bool,
    /// Number of added files
    pub added: u32,
    /// Number of modified files
    pub modified: u32,
    /// Number of deleted files
    pub deleted: u32,
    /// Current branch name (if any)
    pub current_branch: Option<BranchName>,
}

// ============================================================================
// VcsBackend Trait
// ============================================================================

/// Unified VCS backend trait for Git and JJ operations
///
/// # Type Invariants
/// - All methods return `Result<T, VcsError>` - never panic
/// - Implementations must be thread-safe (Send + Sync)
/// - Operations are idempotent where semantically meaningful
pub trait VcsBackend: Send + Sync {
    /// Get the backend type for this implementation
    fn backend_type(&self) -> BackendType;

    /// Get the repository path
    fn path(&self) -> &RepositoryPath;

    /// Detect the current branch
    ///
    /// # Returns
    /// - `Ok(Some(BranchName))` if on a branch
    /// - `Ok(None)` if in detached HEAD state (Git) or equivalent
    ///
    /// # Errors
    /// Returns `VcsError` if the branch cannot be determined.
    fn current_branch(&self) -> Result<Option<BranchName>, VcsError>;

    /// List all branches in the repository
    ///
    /// # Errors
    /// Returns `VcsError` if branches cannot be listed.
    fn list_branches(&self) -> Result<Vec<BranchName>, VcsError>;

    /// Get the repository status
    ///
    /// # Errors
    /// Returns `VcsError` if status cannot be determined.
    fn status(&self) -> Result<RepoStatus, VcsError>;

    /// Check if a commit exists in the repository
    ///
    /// # Errors
    /// Returns `VcsError` if the commit check fails.
    fn commit_exists(&self, id: &CommitId) -> Result<bool, VcsError>;

    /// Check if the working directory is clean (no uncommitted changes)
    ///
    /// # Default Implementation
    /// Uses `status()` to determine if there are changes
    ///
    /// # Errors
    /// Returns `VcsError` if status cannot be determined.
    fn is_clean(&self) -> Result<bool, VcsError> {
        self.status().map(|s| !s.has_changes)
    }

    /// Rebase the given branch onto its parent branch
    ///
    /// # Preconditions
    /// - Branch must exist in the repository
    /// - Working directory must be clean
    ///
    /// # Errors
    /// Returns `VcsError` if the rebase fails.
    fn sync(&self, branch: &BranchName, parent: &BranchName) -> Result<(), VcsError>;
}

// ============================================================================
// Detection Function
// ============================================================================

/// Detect the VCS backend type at a given path
///
/// # Preconditions
/// - Path must exist
/// - Path must be a directory
/// - Either .git or .jj must exist in path hierarchy
///
/// # Detection Order
/// - Checks for .jj first (JJ can wrap Git repositories)
/// - Then checks for .git
/// - Returns `NoVcsFound` if neither exists
///
/// # Errors
/// - `VcsError::PathNotFound` if path does not exist
/// - `VcsError::PathNotDirectory` if path is not a directory
/// - `VcsError::NoVcsFound` if no VCS detected in path hierarchy
pub fn detect_backend(path: impl AsRef<Path>) -> Result<BackendType, VcsError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(VcsError::PathNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(VcsError::PathNotDirectory(path.to_path_buf()));
    }

    path.ancestors()
        .find_map(|current| {
            if current.join(".jj").exists() {
                Some(BackendType::Jj)
            } else if current.join(".git").exists() {
                Some(BackendType::Git)
            } else {
                None
            }
        })
        .ok_or_else(|| VcsError::NoVcsFound(path.to_path_buf()))
}

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
        let backend = StubBackend { path: repo_path };

        let backend_type = backend.backend_type();
        let _path = backend.path();

        assert_eq!(backend_type, BackendType::Git);
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
