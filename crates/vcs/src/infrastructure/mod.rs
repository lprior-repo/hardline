//! VCS Infrastructure Layer - Backend implementations

pub mod git;
pub mod git_cli;
pub mod jj;

pub use git::GitBackend;
pub use git_cli::GitCliBackend;
pub use jj::JjBackend;
