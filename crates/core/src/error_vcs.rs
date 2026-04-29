//! VCS-related errors.
//!
//! Error codes: 5xxx (ADR-007)

use thiserror::Error;

use crate::error::Error;

/// VCS-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct VcsError {
    #[from]
    inner: VcsErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum VcsErrorKind {
    /// VCS not initialized
    #[error("VCS not initialized in this directory")]
    NotInitialized,

    /// VCS conflict detected
    #[error("VCS conflict in {0}: {1}")]
    Conflict(String, String),

    /// Push failed
    #[error("Failed to push: {0}")]
    PushFailed(String),

    /// Pull failed
    #[error("Failed to pull: {0}")]
    PullFailed(String),

    /// Rebase failed
    #[error("Failed to rebase: {0}")]
    RebaseFailed(String),

    /// Branch not found
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Branch already exists
    #[error("Branch already exists: {0}")]
    BranchExists(String),

    /// Commit not found
    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    /// Working copy is dirty
    #[error("Working copy has uncommitted changes")]
    WorkingCopyDirty,

    /// Commit failed
    #[error("Failed to commit: {0}")]
    CommitFailed(String),

    /// Checkout failed
    #[error("Failed to checkout: {0}")]
    CheckoutFailed(String),

    /// Diff failed
    #[error("Failed to get diff: {0}")]
    DiffFailed(String),

    /// Merge returned no commit ID
    #[error("Merge produced no commit ID")]
    MergeNoCommitId,

    /// VCS initialization failed
    #[error("Failed to initialize {vcs_type} in {directory}: {reason}")]
    InitFailed {
        /// VCS type being initialized (e.g. "git")
        vcs_type: String,
        /// Directory where init was attempted
        directory: String,
        /// Human-readable reason for the failure
        reason: String,
    },
}

impl From<VcsErrorKind> for Error {
    fn from(e: VcsErrorKind) -> Self {
        Error::Vcs(e.into())
    }
}

// ========================================================================
// Suggestion & Exit Code
// ========================================================================

impl VcsError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &VcsErrorKind {
        &self.inner
    }

    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self.inner {
            VcsErrorKind::NotInitialized => Some("Run 'scp init' to initialize VCS".to_string()),
            VcsErrorKind::WorkingCopyDirty => {
                Some("Commit or stash your changes before continuing".to_string())
            }
            VcsErrorKind::InitFailed {
                ref vcs_type,
                ref reason,
                ..
            } => classify_init_failure_suggestion(vcs_type, reason),
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    /// VCS errors use range 50-58 (ADR-007: 5xxx).
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            VcsErrorKind::NotInitialized => 50,
            VcsErrorKind::Conflict(_, _) => 51,
            VcsErrorKind::PushFailed(_) => 52,
            VcsErrorKind::PullFailed(_) => 53,
            VcsErrorKind::RebaseFailed(_) => 54,
            VcsErrorKind::BranchNotFound(_) => 55,
            VcsErrorKind::BranchExists(_) => 56,
            VcsErrorKind::CommitNotFound(_) => 57,
            VcsErrorKind::WorkingCopyDirty => 58,
            VcsErrorKind::CommitFailed(_) => 59,
            VcsErrorKind::CheckoutFailed(_) => 60,
            VcsErrorKind::DiffFailed(_) => 61,
            VcsErrorKind::MergeNoCommitId => 62,
            VcsErrorKind::InitFailed { .. } => 63,
        }
    }
}

/// Classify an init failure reason and return an actionable suggestion.
fn classify_init_failure_suggestion(vcs_type: &str, reason: &str) -> Option<String> {
    let reason_lower = reason.to_lowercase();

    if reason_lower.contains("another init process is in progress")
        || reason_lower.contains("lock")
        || reason_lower.contains("already locked")
    {
        return Some(format!(
            "Another {vcs_type} initialization is in progress. \
             Wait for it to finish, or remove the lock file if the process has crashed."
        ));
    }

    if reason_lower.contains("unrecognized subcommand")
        || reason_lower.contains("not found")
        || reason_lower.contains("no such file")
    {
        return Some(format!(
            "The '{vcs_type}' command is not installed or is not in your PATH. \
             Install it and try again."
        ));
    }

    if reason_lower.contains("already exists")
        || reason_lower.contains("already initialized")
        || reason_lower.contains("destination path")
    {
        return Some(format!(
            "The directory is already initialized with {vcs_type}. \
             If this is unexpected, check for a partial '.{vcs_type}' directory."
        ));
    }

    if reason_lower.contains("permission denied") || reason_lower.contains("access denied") {
        return Some("Check file permissions for the target directory.".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcs_error_kind_display() {
        assert_eq!(
            VcsErrorKind::NotInitialized.to_string(),
            "VCS not initialized in this directory"
        );
        assert_eq!(
            VcsErrorKind::PushFailed("network error".to_string()).to_string(),
            "Failed to push: network error"
        );
        assert_eq!(
            VcsErrorKind::InitFailed {
                vcs_type: "git".to_string(),
                directory: "/tmp".to_string(),
                reason: "not found".to_string(),
            }
            .to_string(),
            "Failed to initialize git in /tmp: not found"
        );
    }

    #[test]
    fn test_vcs_error_suggestion() {
        assert!(VcsError::from(VcsErrorKind::NotInitialized)
            .suggestion()
            .is_some());
        assert!(VcsError::from(VcsErrorKind::WorkingCopyDirty)
            .suggestion()
            .is_some());
        assert!(VcsError::from(VcsErrorKind::InitFailed {
            vcs_type: "git".to_string(),
            directory: "/tmp".to_string(),
            reason: "lock".to_string(),
        })
        .suggestion()
        .is_some());
        // No suggestion for random errors
        assert!(VcsError::from(VcsErrorKind::PushFailed("fail".to_string()))
            .suggestion()
            .is_none());
    }

    #[test]
    fn test_vcs_error_exit_codes() {
        assert_eq!(VcsError::from(VcsErrorKind::NotInitialized).exit_code(), 50);
        assert_eq!(
            VcsError::from(VcsErrorKind::Conflict("x".into(), "y".into())).exit_code(),
            51
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::PushFailed("x".into())).exit_code(),
            52
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::PullFailed("x".into())).exit_code(),
            53
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::RebaseFailed("x".into())).exit_code(),
            54
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::BranchNotFound("x".into())).exit_code(),
            55
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::BranchExists("x".into())).exit_code(),
            56
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::CommitNotFound("x".into())).exit_code(),
            57
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::WorkingCopyDirty).exit_code(),
            58
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::CommitFailed("x".into())).exit_code(),
            59
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::CheckoutFailed("x".into())).exit_code(),
            60
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::DiffFailed("x".into())).exit_code(),
            61
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::MergeNoCommitId).exit_code(),
            62
        );
        assert_eq!(
            VcsError::from(VcsErrorKind::InitFailed {
                vcs_type: "git".into(),
                directory: "/tmp".into(),
                reason: "x".into(),
            })
            .exit_code(),
            63
        );
    }

    #[test]
    fn test_classify_init_failure_suggestion_lock() {
        let suggestion = classify_init_failure_suggestion("git", "lock file exists");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("lock"));
    }

    #[test]
    fn test_classify_init_failure_suggestion_not_found() {
        let suggestion = classify_init_failure_suggestion("git", "command not found");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("not installed"));
    }

    #[test]
    fn test_classify_init_failure_suggestion_already_exists() {
        let suggestion = classify_init_failure_suggestion("git", "already initialized");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("already initialized"));
    }

    #[test]
    fn test_classify_init_failure_suggestion_permission() {
        let suggestion = classify_init_failure_suggestion("git", "permission denied");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("permissions"));
    }

    #[test]
    fn test_classify_init_failure_suggestion_unknown() {
        let suggestion = classify_init_failure_suggestion("git", "some random error");
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_from_vcs_error_kind_to_error() {
        let err: Error = VcsErrorKind::NotInitialized.into();
        assert!(matches!(err, Error::Vcs(_)));
    }
}
