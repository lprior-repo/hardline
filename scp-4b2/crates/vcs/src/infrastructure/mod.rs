//! VCS Infrastructure Layer - Backend implementations

pub mod git;
pub mod jj;

pub use git::GitBackend;
pub use jj::JjBackend;
