use crate::domain::{
    AbsolutePath, BranchName, WorktreeDomainError, WorktreeId, WorktreeName, WorktreeTypeEnum,
};

/// Command to create a new worktree
#[derive(Debug, Clone)]
pub struct CreateWorktreeCommand {
    pub name: WorktreeName,
    pub path: AbsolutePath,
    pub parent_path: AbsolutePath,
    pub worktree_type: WorktreeTypeEnum,
    pub branch: Option<BranchName>,
}

impl CreateWorktreeCommand {
    #[must_use]
    pub fn new(
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
    ) -> Self {
        Self {
            name,
            path,
            parent_path,
            worktree_type,
            branch,
        }
    }
}

/// Command to initialize an existing worktree
#[derive(Debug, Clone)]
pub struct InitializeWorktreeCommand {
    pub worktree_id: WorktreeId,
}

impl InitializeWorktreeCommand {
    #[must_use]
    pub fn new(worktree_id: WorktreeId) -> Self {
        Self { worktree_id }
    }
}

/// Command to suspend a worktree
#[derive(Debug, Clone)]
pub struct SuspendWorktreeCommand {
    pub worktree_id: WorktreeId,
}

impl SuspendWorktreeCommand {
    #[must_use]
    pub fn new(worktree_id: WorktreeId) -> Self {
        Self { worktree_id }
    }
}

/// Command to resume a suspended worktree
#[derive(Debug, Clone)]
pub struct ResumeWorktreeCommand {
    pub worktree_id: WorktreeId,
}

impl ResumeWorktreeCommand {
    #[must_use]
    pub fn new(worktree_id: WorktreeId) -> Self {
        Self { worktree_id }
    }
}

/// Command to remove a worktree
#[derive(Debug, Clone)]
pub struct RemoveWorktreeCommand {
    pub worktree_id: WorktreeId,
}

impl RemoveWorktreeCommand {
    #[must_use]
    pub fn new(worktree_id: WorktreeId) -> Self {
        Self { worktree_id }
    }
}

/// Query to list worktrees with optional filters
#[derive(Debug, Clone, Default)]
pub struct ListWorktreesQuery {
    pub include_removed: bool,
    pub state_filter: Option<crate::domain::WorktreeState>,
    pub worktree_type_filter: Option<WorktreeTypeEnum>,
    pub name_prefix: Option<String>,
}

impl ListWorktreesQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_include_removed(mut self, include_removed: bool) -> Self {
        self.include_removed = include_removed;
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: crate::domain::WorktreeState) -> Self {
        self.state_filter = Some(state);
        self
    }

    #[must_use]
    pub fn with_worktree_type(mut self, worktree_type: WorktreeTypeEnum) -> Self {
        self.worktree_type_filter = Some(worktree_type);
        self
    }

    #[must_use]
    pub fn with_name_prefix(mut self, prefix: &str) -> Self {
        self.name_prefix = Some(prefix.to_string());
        self
    }
}

/// Result type for command execution
pub type CommandResult<T> = Result<T, WorktreeDomainError>;
