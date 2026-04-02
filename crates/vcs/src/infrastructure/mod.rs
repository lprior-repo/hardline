//! VCS Infrastructure Layer - Backend implementations

pub mod git;
pub mod git_cli;

pub use git::GitBackend;
pub use git_cli::GitCliBackend;
