//! Worktree - A crate for managing Git worktrees
//!
//! This crate provides a type-safe, DDD-inspired API for managing Git worktrees.
//! It follows functional Rust patterns with zero panics in source code.

pub mod application;
pub mod domain;
pub mod infrastructure;

// Re-export domain types at crate level
// Re-export application types
pub use application::{
    commands::{
        CreateWorktreeCommand, InitializeWorktreeCommand, ListWorktreesQuery,
        RemoveWorktreeCommand, ResumeWorktreeCommand, SuspendWorktreeCommand,
    },
    services::WorktreeService,
};
pub use domain::{
    absolute_path::AbsolutePath, branch_name::BranchName, errors::WorktreeDomainError,
    worktree::Worktree, worktree_id::WorktreeId, worktree_name::WorktreeName,
    worktree_state::WorktreeState, worktree_type_enum::WorktreeTypeEnum,
};
// Re-export adapters
pub use infrastructure::git::{GitError, GitWorktreeAdapter};
// Re-export repository traits
pub use infrastructure::repositories::WorktreeRepository;
pub use infrastructure::sqlx::{PostgresWorktreeRepository, SqliteWorktreeRepository};
