//! Core Infrastructure Layer - External integrations and I/O
//!
//! This module contains:
//! - Database access (SQLite, etc.)
//! - File system operations
//! - External API clients
//! - Git VCS integration
//! - Event store lock management

pub mod chaos;
pub mod database;
pub mod database_types;
pub mod event_store_lock_repository;
pub mod event_store_lock_schema;
pub mod event_store_lock_types;
pub mod operation_log;
pub mod operation_log_repository;
pub mod operation_log_schema;
pub mod operation_log_types;
pub mod restate;
pub mod vcs_integration;

// operation_log module re-exports its submodules
pub use operation_log::{
    ensure_operation_log_schema, get_stream_version, insert_operation_log, query_all_operations,
    query_stream_events, OperationLogEntry, OperationLogError,
};

// event_store_locks re-exports
pub use event_store_lock_repository::{
    acquire_stream_lock, cleanup_expired_stream_locks, ensure_event_store_locks, get_next_sequence,
    get_stream_locks, is_stream_locked, locks_by_holder, release_stream_lock,
};
pub use event_store_lock_schema::ensure_event_store_lock_schema;
pub use event_store_lock_types::{parse_event_store_lock_row, EventStoreLock, EventStoreLockError};

pub use chaos::{
    ChaosConfig, ChaosDatabaseService, ChaosFs, ChaosInjector, ChaosNetworkService, NetworkService,
};
pub use database::{
    create_database_service, DatabaseConfig, DatabaseService, SqliteDatabaseService,
};
pub use database_types::{DatabasePath, MaxConnections};
pub use vcs_integration::{
    create_vcs_integration_service, VcsIntegrationService, VcsIntegrationServiceImpl,
};
