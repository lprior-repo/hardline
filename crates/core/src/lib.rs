//! Source Control Plane (SCP) - Core Library
//!
//! Unified core for workspace isolation (Isolate) and queue management (Stak).
//!
//! # Architecture (DDD)
//!
//! - `domain` - Pure domain types, entities, and business logic
//! - `application` - Use cases and service orchestration
//! - `infrastructure` - External integrations (DB, VCS, network)
//!
//! # Zero Unwrap Law
//!
//! All fallible operations return `Result<T, Error>`. No unwrap, no panic.

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

// Module declarations
pub mod agent;
pub mod application;
pub mod architecture_boundaries;
pub mod beads;
pub mod checkpoint;
#[cfg(test)]
mod checkpoint_redqueen;
pub mod cli_contracts;
pub mod config;
pub mod conflict;
pub mod contracts;
pub mod coordination;
pub mod dag;
pub mod domain;
pub mod error;
pub mod error_agent;
pub mod error_config;
pub mod error_internal;
pub mod error_io;
pub mod error_queue;
pub mod error_state;
pub mod error_task;
pub mod error_types;
pub mod error_vcs;
pub mod error_wait;
pub mod error_workspace;
pub mod events;
pub mod fix;
pub mod functional;
pub mod hints;
pub mod hooks;
pub mod infrastructure;
pub mod json;
pub mod introspection;
pub mod lifecycle;
pub mod lock;
pub mod moon_gates;
pub mod output;
pub mod output_format;
pub mod output_jsonl;
pub mod queue;
pub mod recovery;
pub mod session_state;
pub mod session_sync;
pub mod session_sync_calculations;
pub mod session_sync_data;
pub mod session_sync_errors;
pub mod shutdown;
pub mod taskregistry;
pub mod type_beads_issue;
pub mod type_branch_state;
pub mod type_file_change;
pub mod type_metadata;
pub mod type_session;
pub mod type_session_id;
pub mod type_session_name;
pub mod type_session_path;
pub mod type_session_status;
pub mod types;
pub mod validation;
pub mod vcs;
pub mod watcher;
pub mod workspace_integrity;
pub mod workspace_state;

#[cfg(test)]
mod config_property_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod json_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod session_state_tests;
#[cfg(test)]
mod types_tests;

// Re-exports
pub use agent::{
    get_agent_registry, Agent, AgentActivity, AgentId, AgentRegistry, AgentStatus, MemAgentRegistry,
};
pub use checkpoint::{
    classify_command, find_pending_restores, AutoCheckpoint, CheckpointGuard, OperationRisk,
};
pub use config::{
    config_dir, global_config, keys, Config, ConfigManager, ConfigScope, ConfigSource, ConfigValue,
    WatchConfig,
};
pub use conflict::{Conflict, ConflictManager, ConflictState};
pub use dag::{BranchDag, BranchId, DagError};
pub use error::{Error, Result};
pub use events::{EmittedEvent, Event, EventEmitter};
pub use fix::{ErrorWithFixes, Fix, FixImpact};
pub use hooks::{Hook, HookConfig, HookEnv, HookEvent, HookManager, HookResult, HookRunner};
pub use json::{
    classify_exit_code, error_with_available_sessions, map_error_to_parts, output_json_parse_error,
    output_json_success, semantic_exit_code, ErrorCode, ErrorDetail, HateoasLink, JsonError,
    JsonSerializable, JsonSuccess, RelatedResources, ResponseMeta, SchemaEnvelope,
    SchemaEnvelopeArray,
};
pub use lifecycle::LifecycleState;
pub use lock::{LockGuard, LockType, MemLockManager};
pub use moon_gates::{GateError, GateResult, GatesOutcome, GatesStatus, MoonGate};
pub use output::{Output, Verbosity};
pub use output_format::OutputFormat;
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
    create_backend, detect_vcs, Branch, Commit, GitBackend, VcsBackend, VcsStatus, VcsType,
    Workspace,
};
pub use watcher::{BeadsStatus, FileWatcher, WatchEvent};
pub use workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

pub use application::{
    create_coordination_service, create_queue_service, CoordinationService, QueueService,
    QueueServiceImpl,
};
pub use infrastructure::{
    create_database_service, create_vcs_integration_service, DatabaseConfig, DatabaseService,
    SqliteDatabaseService, VcsIntegrationService, VcsIntegrationServiceImpl,
};

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
