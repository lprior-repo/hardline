//! Tests for GitError type

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use scp_vcs::error::{GitError, GitResult};

    #[test]
    fn test_git_error_not_found() {
        let err = GitError::NotFound(PathBuf::from("/nonexistent"));
        let msg = err.to_string();
        assert!(msg.contains("/nonexistent"));
    }

    #[test]
    fn test_result_alias() {
        fn some_fn() -> GitResult<i32> {
            Ok(42)
        }
        assert!(some_fn().is_ok());
    }
}
