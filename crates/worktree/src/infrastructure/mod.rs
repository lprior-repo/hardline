//! Infrastructure layer - External system adapters and persistence

pub mod git;
pub mod sqlx;
pub mod repositories;

pub use git::{GitError, GitWorktreeAdapter};
pub use repositories::WorktreeRepository;
pub use sqlx::{SqliteWorktreeRepository, PostgresWorktreeRepository};
