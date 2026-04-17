//! Git CLI Backend Implementation
//!
//! Executes git CLI commands and parses output into domain types.
//! All operations return Result<T, VcsError> - no panics.

pub mod core;
pub mod vcs_impl;

pub use core::GitCliBackend;
