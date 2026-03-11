//! Source Control Plane (SCP) - Core Library
//!
//! Unified core for workspace isolation (Isolate) and queue management (Stak).
//!
//! # Architecture
//!
//! - `error` - Unified error types with suggestions and exit codes
//! - `lock` - Lock management for workspaces, sessions, and queues
//! - `queue` - Queue management with priority support
//! - `vcs` - VCS abstraction (JJ/Git backends)
//! - `events` - Unified event system
//!
//! # Zero Unwrap Law
//!
//! All fallible operations return `Result<T, Error>`. No unwrap, no panic.
//!
//! # Example
//!
//! ```ignore
//! use scp_core::{Result, Error};
//!
//! fn example() -> Result<()> {
//!     // All operations return Result
//!     let item = QueueItem::direct("feature branch");
//!     queue.enqueue(item)?;
//!     Ok(())
//! }
//! ```

// Module declarations
pub mod agent;
pub mod architecture_boundaries;
pub mod checkpoint;
pub mod config;
pub mod conflict;
pub mod contracts;
pub mod dag;
pub mod error;
pub mod fix;
pub mod functional;
pub mod events;
pub mod hooks;
pub mod hints;
pub mod introspection;
pub mod jj;
pub mod jj_operation_sync;
pub mod lifecycle;
pub mod lock;
pub mod moon_gates;
pub mod queue;
pub mod recovery;
pub mod session_state;
pub mod session_sync;
pub mod shutdown;
pub mod taskregistry;
pub mod types;
pub mod validation;
pub mod vcs;
pub mod output_format;
pub mod watcher;
pub mod workspace_state;

// Re-exports
pub use agent::{
    get_agent_registry, Agent, AgentActivity, AgentId, AgentRegistry, AgentStatus, MemAgentRegistry,
};
pub use checkpoint::{AutoCheckpoint, CheckpointGuard, OperationRisk, classify_command, find_pending_restores};
pub use config::{
    config_dir, global_config, keys, Config, ConfigManager, ConfigScope, ConfigSource, ConfigValue,
    WatchConfig,
};
pub use conflict::{Conflict, ConflictManager, ConflictState};
pub use dag::{BranchDag, BranchId, DagError};
pub use error::{Error, Result};
pub use fix::{ErrorWithFixes, Fix, FixImpact};
pub use events::{EmittedEvent, Event, EventEmitter};
pub use hooks::{Hook, HookConfig, HookEnv, HookEvent, HookManager, HookResult, HookRunner};
pub use jj::{
    create_workspace, get_jj_command, get_jj_command_sync, is_jj_installed, is_jj_repo,
    parse_diff_stat, parse_status, workspace_create, workspace_diff, workspace_forget,
    workspace_list, workspace_status, Status, WorkspaceGuard, WorkspaceInfo,
};
pub use jj_operation_sync::{
    create_workspace_synced, get_current_operation, RepoOperationInfo,
};
pub use lifecycle::LifecycleState;
pub use lock::{LockGuard, LockInfo, LockManager, LockType, MemLockManager};
pub use moon_gates::{GateError, GateResult, GatesOutcome, GatesStatus, MoonGate};
pub use queue::{
    MemQueue, Priority, ProcessResult, QueueItem, QueueManager, QueueSource, QueueStatus,
};
pub use recovery::{RecoveryConfig, RecoveryPolicy};
pub use session_state::{SessionState, SessionStateManager, StateTransition};
pub use shutdown::{signal_channels, ShutdownCoordinator, ShutdownSignal};
pub use taskregistry::TaskRegistry;
pub use types::{
    AbsolutePath, BeadsIssue, BeadsSummary, BranchState, ChangesSummary, DiffSummary, FileChange,
    FileDiffStat, FileStatus, IssueStatus, Operation, Session, SessionId, SessionName,
    SessionStatus, ValidatedMetadata,
};
pub use vcs::{
    create_backend, detect_vcs, Branch, Commit, GitBackend, JjBackend, VcsBackend, VcsStatus,
    VcsType, Workspace,
};
pub use output_format::OutputFormat;
pub use watcher::{BeadsStatus, FileWatcher, WatchEvent};
pub use workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

/// SCP version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
