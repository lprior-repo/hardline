//! Infrastructure layer - External system adapters and persistence

pub mod git;
pub mod repositories;
pub mod sqlx;

pub use git::{GitError, GitWorktreeAdapter};
pub use repositories::WorktreeRepository;
pub use sqlx::{PostgresWorktreeRepository, SqliteWorktreeRepository};
