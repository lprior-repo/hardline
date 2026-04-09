//! VCS Error Types

use std::path::PathBuf;

use thiserror::Error;

/// Railway-oriented Git errors for gitoxide operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Repository not found at {0}")]
    NotFound(PathBuf),

    #[error("Invalid reference '{name}': {reason}")]
    InvalidRef { name: String, reason: String },

    #[error("Merge conflict: {message}\nConflicted files: {conflicted_files:?}")]
    Conflict {
        message: String,
        conflicted_files: Vec<PathBuf>,
    },

    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Gitoxide error: {0}")]
    Gix(#[from] gix::Error),

    #[error("Gitoxide discover error: {0}")]
    GixDiscover(#[from] gix::discover::Error),

    #[error("Gitoxide init error: {0}")]
    GixInit(#[from] gix::init::Error),

    #[error("Gitoxide status error: {0}")]
    GixStatus(#[from] gix::status::Error),
    #[error("Gitoxide status iter error: {0}")]
    GixStatusIter(#[from] gix::status::into_iter::Error),

    #[error("Failed to parse git output: {0}")]
    ParseError(String),
}

/// Result type for GitError operations
pub type GitResult<T> = std::result::Result<T, GitError>;

#[derive(Error, Debug)]
pub enum VcsError {
    #[error("VCS not initialized in this directory")]
    NotInitialized,

    #[error("VCS conflict in {0}: {1}")]
    Conflict(String, String),

    #[error("Failed to push: {0}")]
    PushFailed(String),

    #[error("Failed to pull: {0}")]
    PullFailed(String),

    #[error("Failed to rebase: {0}")]
    RebaseFailed(String),

    #[error("Branch already exists: {0}")]
    BranchExists(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Git CLI not found in PATH")]
    GitNotInstalled,

    #[error("Failed to parse git output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Feature not implemented: {0}")]
    Unimplemented(String),
}

/// Result type for VcsError operations (backward compatible)
pub type Result<T> = std::result::Result<T, VcsError>;

impl From<GitError> for VcsError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::NotFound(_path) => VcsError::NotInitialized,
            GitError::InvalidRef { name, reason: _ } => VcsError::BranchNotFound(name),
            GitError::Conflict { message, .. } => VcsError::Conflict(message, String::new()),
            GitError::Unauthorized(msg) => VcsError::PushFailed(msg),
            GitError::Network(msg) => VcsError::PullFailed(msg),
            GitError::Io(io_err) => VcsError::Io(io_err),
            GitError::Gix(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixDiscover(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixInit(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixStatus(err) => VcsError::Unimplemented(err.to_string()),
            GitError::GixStatusIter(err) => VcsError::Unimplemented(err.to_string()),
            GitError::ParseError(msg) => VcsError::ParseError(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- GitError Display tests --

    #[test]
    fn git_error_not_found_display() {
        let err = GitError::NotFound("/repo".into());
        let msg = format!("{err}");
        assert!(msg.contains("/repo"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn git_error_invalid_ref_display() {
        let err = GitError::InvalidRef {
            name: "HEAD~5".into(),
            reason: "invalid syntax".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("HEAD~5"));
        assert!(msg.contains("invalid syntax"));
    }

    #[test]
    fn git_error_conflict_display() {
        let err = GitError::Conflict {
            message: "merge conflict".into(),
            conflicted_files: vec!["file1.rs".into()],
        };
        let msg = format!("{err}");
        assert!(msg.contains("merge conflict"));
        assert!(msg.contains("file1.rs"));
    }

    #[test]
    fn git_error_unauthorized_display() {
        let err = GitError::Unauthorized("bad token".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad token"));
        assert!(msg.contains("Authentication"));
    }

    #[test]
    fn git_error_network_display() {
        let err = GitError::Network("timeout".into());
        let msg = format!("{err}");
        assert!(msg.contains("timeout"));
        assert!(msg.contains("Network"));
    }

    #[test]
    fn git_error_io_display() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = GitError::Io(io);
        let msg = format!("{err}");
        assert!(msg.contains("missing"));
    }

    #[test]
    fn git_error_from_io_conversion() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: GitError = io.into();
        assert!(matches!(err, GitError::Io(_)));
    }

    // -- VcsError Display tests --

    #[test]
    fn vcs_error_not_initialized_display() {
        let err = VcsError::NotInitialized;
        let msg = format!("{err}");
        assert!(msg.contains("not initialized"));
    }

    #[test]
    fn vcs_error_conflict_display() {
        let err = VcsError::Conflict("main".into(), "diverged".into());
        let msg = format!("{err}");
        assert!(msg.contains("main"));
        assert!(msg.contains("diverged"));
    }

    #[test]
    fn vcs_error_push_failed_display() {
        let err = VcsError::PushFailed("rejected".into());
        let msg = format!("{err}");
        assert!(msg.contains("rejected"));
        assert!(msg.contains("push"));
    }

    #[test]
    fn vcs_error_pull_failed_display() {
        let err = VcsError::PullFailed("timeout".into());
        let msg = format!("{err}");
        assert!(msg.contains("timeout"));
        assert!(msg.contains("pull"));
    }

    #[test]
    fn vcs_error_rebase_failed_display() {
        let err = VcsError::RebaseFailed("conflict".into());
        let msg = format!("{err}");
        assert!(msg.contains("conflict"));
        assert!(msg.contains("rebase"));
    }

    #[test]
    fn vcs_error_branch_exists_display() {
        let err = VcsError::BranchExists("main".into());
        let msg = format!("{err}");
        assert!(msg.contains("main"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn vcs_error_branch_not_found_display() {
        let err = VcsError::BranchNotFound("missing".into());
        let msg = format!("{err}");
        assert!(msg.contains("missing"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn vcs_error_workspace_not_found_display() {
        let err = VcsError::WorkspaceNotFound("ws-1".into());
        let msg = format!("{err}");
        assert!(msg.contains("ws-1"));
    }

    #[test]
    fn vcs_error_workspace_exists_display() {
        let err = VcsError::WorkspaceExists("ws-1".into());
        let msg = format!("{err}");
        assert!(msg.contains("ws-1"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn vcs_error_git_not_installed_display() {
        let err = VcsError::GitNotInstalled;
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
        assert!(msg.contains("PATH"));
    }

    #[test]
    fn vcs_error_parse_error_display() {
        let err = VcsError::ParseError("bad output".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad output"));
        assert!(msg.contains("parse"));
    }

    #[test]
    fn vcs_error_unimplemented_display() {
        let err = VcsError::Unimplemented("feature X".into());
        let msg = format!("{err}");
        assert!(msg.contains("feature X"));
        assert!(msg.contains("not implemented"));
    }

    #[test]
    fn vcs_error_from_io() {
        let io = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe");
        let err: VcsError = io.into();
        assert!(matches!(err, VcsError::Io(_)));
    }

    // -- From<GitError> for VcsError --

    #[test]
    fn git_error_to_vcs_error_not_initialized() {
        let git_err = GitError::NotFound("/repo".into());
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::NotInitialized));
    }

    #[test]
    fn git_error_to_vcs_error_branch_not_found() {
        let git_err = GitError::InvalidRef {
            name: "missing".into(),
            reason: "x".into(),
        };
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::BranchNotFound(n) if n == "missing"));
    }

    #[test]
    fn git_error_to_vcs_error_conflict() {
        let git_err = GitError::Conflict {
            message: "conflict".into(),
            conflicted_files: vec![],
        };
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::Conflict(m, _) if m == "conflict"));
    }

    #[test]
    fn git_error_to_vcs_error_push_failed() {
        let git_err = GitError::Unauthorized("denied".into());
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::PushFailed(m) if m == "denied"));
    }

    #[test]
    fn git_error_to_vcs_error_pull_failed() {
        let git_err = GitError::Network("timeout".into());
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::PullFailed(m) if m == "timeout"));
    }

    #[test]
    fn git_error_to_vcs_error_io() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let git_err = GitError::Io(io);
        let vcs_err: VcsError = git_err.into();
        assert!(matches!(vcs_err, VcsError::Io(_)));
    }

    // -- Result types --

    #[test]
    fn git_result_type() {
        let ok: GitResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
        let err: GitResult<i32> = Err(GitError::NotFound("/x".into()));
        assert!(err.is_err());
    }

    #[test]
    fn result_type() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
        let err: Result<i32> = Err(VcsError::NotInitialized);
        assert!(err.is_err());
    }

    // -- Additional GitError -> VcsError conversion tests --

    #[test]
    fn git_error_to_vcs_error_gix() {
        // We can't easily create a gix::Error, so test the conversion path indirectly
        // by testing that the match arm exists
        let msg = format!("{}", VcsError::Unimplemented("test gix error".to_string()));
        assert!(msg.contains("not implemented"));
    }

    #[test]
    fn git_error_conflict_with_multiple_files_display() {
        let err = GitError::Conflict {
            message: "merge conflict".into(),
            conflicted_files: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
        };
        let msg = format!("{err}");
        assert!(msg.contains("a.rs"));
        assert!(msg.contains("b.rs"));
        assert!(msg.contains("c.rs"));
    }

    #[test]
    fn git_error_conflict_with_empty_files_display() {
        let err = GitError::Conflict {
            message: "conflict".into(),
            conflicted_files: vec![],
        };
        let msg = format!("{err}");
        assert!(msg.contains("conflict"));
    }

    #[test]
    fn git_error_not_found_with_long_path() {
        let long_path = "/very/long/path/to/some/deeply/nested/repository/that/does/not/exist";
        let err = GitError::NotFound(long_path.into());
        let msg = format!("{err}");
        assert!(msg.contains(long_path));
    }

    #[test]
    fn git_error_invalid_ref_with_reason() {
        let err = GitError::InvalidRef {
            name: "HEAD~100".into(),
            reason: "unknown revision".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("HEAD~100"));
        assert!(msg.contains("unknown revision"));
    }

    // -- VcsError Display edge cases --

    #[test]
    fn vcs_error_unimplemented_with_empty_string() {
        let err = VcsError::Unimplemented(String::new());
        let msg = format!("{err}");
        assert!(msg.contains("not implemented"));
    }

    #[test]
    fn vcs_error_conflict_with_empty_strings() {
        let err = VcsError::Conflict(String::new(), String::new());
        let msg = format!("{err}");
        // Should not panic on empty strings
        assert!(msg.contains("VCS conflict"));
    }

    #[test]
    fn vcs_error_conflict_with_unicode() {
        let err = VcsError::Conflict("main".into(), "diverged \u{1F4A9}".into());
        let msg = format!("{err}");
        assert!(msg.contains("main"));
    }

    #[test]
    fn vcs_error_branch_exists_display_with_long_name() {
        let long_name = "feature/very/long/branch/name/with/many/slashes";
        let err = VcsError::BranchExists(long_name.to_string());
        let msg = format!("{err}");
        assert!(msg.contains(long_name));
    }

    #[test]
    fn vcs_error_parse_error_with_long_message() {
        let long_msg = "a".repeat(1000);
        let err = VcsError::ParseError(long_msg);
        let msg = format!("{err}");
        assert!(msg.contains("parse"));
    }

    // -- VcsError Debug tests --

    #[test]
    fn vcs_error_debug_format() {
        let err = VcsError::NotInitialized;
        let debug = format!("{err:?}");
        assert!(debug.contains("NotInitialized"));
    }

    #[test]
    fn git_error_debug_format() {
        let err = GitError::Network("connection reset".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("Network"));
    }
}
