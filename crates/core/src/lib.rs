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

// TODO: progressively fix these and remove the allows
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

// Temporarily allowed lints - progressively fix and remove
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_inception)]
#![allow(clippy::to_string_trait_impl)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::similar_names)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::type_complexity)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::manual_strip)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::single_match_else)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_option_as_deref)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::map_identity)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::unnecessary_fallible_conversions)]
#![allow(clippy::filetype_is_file)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::redundant_else)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::useless_asref)]
#![allow(clippy::clone_on_copy)]
#![allow(dropping_references)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::unnested_or_patterns)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::suspicious_open_options)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::inherent_to_string)]
#![allow(clippy::result_large_err)]

// Module declarations
pub mod agent;
pub mod application;
pub mod architecture_boundaries;
pub mod beads;
pub mod checkpoint;
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
pub mod error_jj;
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
pub mod introspection;
pub mod jj;
pub mod jj_operation_sync;
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
pub use jj::{
    create_workspace, get_jj_command, get_jj_command_sync, is_jj_installed, is_jj_repo,
    parse_diff_stat, parse_status, workspace_create, workspace_diff, workspace_forget,
    workspace_list, workspace_status, Status, WorkspaceGuard, WorkspaceInfo,
};
pub use jj_operation_sync::{create_workspace_synced, get_current_operation, RepoOperationInfo};
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
    create_backend, detect_vcs, Branch, Commit, GitBackend, JjBackend, VcsBackend, VcsStatus,
    VcsType, Workspace,
};
pub use watcher::{BeadsStatus, FileWatcher, WatchEvent};
pub use workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

pub use application::{
    create_coordination_service, create_queue_service, CoordinationService,
    CoordinationServiceImpl, QueueService, QueueServiceImpl,
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
