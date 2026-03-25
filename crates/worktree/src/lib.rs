//! Worktree - A crate for managing Git worktrees
//! 
//! This crate provides a type-safe, DDD-inspired API for managing Git worktrees.
//! It follows functional Rust patterns with zero panics in source code.

pub mod domain;
pub mod application;
pub mod infrastructure;

// Re-export domain types at crate level
pub use domain::{
    errors::WorktreeDomainError,
    worktree::Worktree,
    worktree_id::WorktreeId,
    worktree_name::WorktreeName,
    worktree_state::WorktreeState,
    worktree_type_enum::WorktreeTypeEnum,
    absolute_path::AbsolutePath,
    branch_name::BranchName,
};

// Re-export application types
pub use application::{
    commands::{
        CreateWorktreeCommand,
        InitializeWorktreeCommand,
        SuspendWorktreeCommand,
        ResumeWorktreeCommand,
        RemoveWorktreeCommand,
        ListWorktreesQuery,
    },
    services::WorktreeService,
};

// Re-export repository traits
pub use infrastructure::repositories::WorktreeRepository;

// Re-export adapters
pub use infrastructure::git::{GitWorktreeAdapter, GitError};
pub use infrastructure::sqlx::{SqliteWorktreeRepository, PostgresWorktreeRepository};
