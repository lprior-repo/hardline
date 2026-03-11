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
}

pub type Result<T> = std::result::Result<T, StackError>;
