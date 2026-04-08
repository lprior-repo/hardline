//! Lock command handler - Manage session locks.
//!
//! Provides subcommands for acquiring, releasing, heartbeat, status, list,
//! force-unlock, and metadata operations on session locks.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): LockCommand, LockOutput, LockStatus, LockMetadata, etc.
//!   (inert, serializable)
//! - **Calculations** (`calculations.rs`): validate_session_name, validate_agent_id,
//!   format_lock_output (pure functions, no I/O)
//! - **Actions** (`actions.rs`): execute_lock_command, run_lock_command (I/O operations)
//!
//! # Subcommands
//!
//! - `acquire` - Acquire a lock on a session
//! - `release` - Release a lock on a session
//! - `heartbeat` - Send heartbeat to extend lock TTL
//! - `status` - Show lock status for a session
//! - `list` - List all active locks
//! - `force-unlock` - Force release a lock (admin)
//! - `metadata` - Show detailed lock metadata

pub mod actions;
pub mod calculations;
pub mod data;

// Re-export public API at module level for convenience
pub use actions::run_lock_command;
pub use calculations::validate_session_name;
pub use data::{AgentId, LockCommand};

#[cfg(test)]
pub(crate) mod tests;
