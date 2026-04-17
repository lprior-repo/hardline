//! Source Control Plane — Unified CLI
//!
//! `scp-cli` provides a single command-line interface for workspace isolation,
//! queue management, Git operations, agent coordination, and task tracking.
//!
//! # Quick Start
//!
//! ```bash
//! scp init                           # Initialize in a Git repo
//! scp workspace spawn feature-auth   # Create an isolated workspace
//! scp workspace commit "Add OAuth2"  # Commit your work
//! scp workspace done -m "OAuth2"     # Merge back to main
//! ```
//!
//! # Command Families
//!
//! - **workspace**: Isolated git worktrees, branches, commits, merge, recovery
//! - **lock**: Distributed locking for multi-agent coordination
//! - **queue**: Ordered merge queue
//! - **agent**: Agent registration and lifecycle
//! - **session**: Session tracking and submission
//! - **task**: Task (bead) management
//! - **config**: Configuration get/set/list
//! - **stash/tag**: Git stash and tag operations
//! - **batch**: Atomic multi-command execution
//! - **fetch/pull/push**: Git remote sync
//! - **doctor/status/context**: Diagnostics and inspection

#![allow(dead_code, unused_imports, unexpected_cfgs, unknown_lints)]
#![allow(
    clippy::must_use_attr,
    clippy::module_inception,
    clippy::from_str_instead_of_fromstr,
    clippy::single_call_fn,
    clippy::double_must_use
)]

use std::process::ExitCode;

mod cli;
mod commands;

fn main() -> ExitCode {
    cli::main::main()
}
