use thiserror::Error;

#[derive(Error, Debug)]
pub enum StackError {
    #[error("Stack not found: {0}")]
    NotFound(String),

    #[error("Stack orphaned branch: {0}")]
    OrphanedBranch(String),

    #[error("Stack cyclic dependency")]
    CyclicDependency,

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),

    #[error("Git error: {0}")]
    GitError(String),

    #[error("GitHub error: {0}")]
    GitHubError(String),

    #[error("Forge error: {0}")]
    ForgeError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),
}

impl From<scp_vcs::VcsError> for StackError {
    fn from(err: scp_vcs::VcsError) -> Self {
        StackError::TransactionError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StackError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_error_display() {
        let err = StackError::NotFound("my-stack".to_string());
        assert_eq!(format!("{err}"), "Stack not found: my-stack");

        let err = StackError::OrphanedBranch("feature-x".to_string());
        assert_eq!(format!("{err}"), "Stack orphaned branch: feature-x");

        let err = StackError::CyclicDependency;
        assert_eq!(format!("{err}"), "Stack cyclic dependency");

        let err = StackError::BranchNotFound("missing".to_string());
        assert_eq!(format!("{err}"), "Branch not found: missing");

        let err = StackError::InvalidBranchName("bad name!".to_string());
        assert_eq!(format!("{err}"), "Invalid branch name: bad name!");

        let err = StackError::GitError("merge failed".to_string());
        assert_eq!(format!("{err}"), "Git error: merge failed");

        let err = StackError::GitHubError("API rate limit".to_string());
        assert_eq!(format!("{err}"), "GitHub error: API rate limit");

        let err = StackError::ForgeError("connection refused".to_string());
        assert_eq!(format!("{err}"), "Forge error: connection refused");
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(0), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(StackError::NotFound("x".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_error_debug_format() {
        let err = StackError::NotFound("debug-test".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("NotFound"));
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn test_stack_error_clone_via_debug() {
        // StackError doesn't derive Clone, but we can verify the Debug output
        let err = StackError::GitHubError("debug me".to_string());
        let debug1 = format!("{err:?}");
        let debug2 = format!("{err:?}");
        assert_eq!(debug1, debug2);
    }

    #[test]
    fn test_stack_error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StackError>();
    }

    #[test]
    fn test_stack_error_all_variants_distinct_display() {
        let variants = vec![
            StackError::NotFound("a".to_string()),
            StackError::OrphanedBranch("a".to_string()),
            StackError::CyclicDependency,
            StackError::BranchNotFound("a".to_string()),
            StackError::InvalidBranchName("a".to_string()),
            StackError::GitError("a".to_string()),
            StackError::GitHubError("a".to_string()),
            StackError::ForgeError("a".to_string()),
        ];
        let mut display_strings: Vec<String> = variants.iter().map(|v| format!("{v}")).collect();
        display_strings.dedup();
        assert_eq!(
            display_strings.len(),
            8,
            "All error variants should have distinct display messages"
        );
    }

    #[test]
    fn test_result_map_ok() {
        let result: Result<i32> = Ok(5);
        let mapped = result.map(|v| v * 2);
        assert_eq!(mapped.unwrap_or(0), 10);
    }

    #[test]
    fn test_result_map_err() {
        let result: Result<i32> = Err(StackError::NotFound("x".to_string()));
        let mapped = result.map(|v| v * 2);
        assert!(mapped.is_err());
    }

    #[test]
    fn test_result_and_then_ok() {
        let result: Result<i32> = Ok(5);
        let chained = result.and_then(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(StackError::GitError("neg".to_string()))
            }
        });
        assert_eq!(chained.unwrap_or(0), 5);
    }

    #[test]
    fn test_result_and_then_err() {
        let result: Result<i32> = Ok(0);
        let chained = result.and_then(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(StackError::GitError("neg".to_string()))
            }
        });
        assert!(chained.is_err());
    }
}
