//! Core Infrastructure Layer - External integrations and I/O
//!
//! This module contains:
//! - Database access (SQLite, etc.)
//! - File system operations
//! - External API clients
//! - JJ/Git VCS integration

pub mod chaos;
pub mod database;
pub mod database_types;
pub mod operation_log;
pub mod operation_log_repository;
pub mod operation_log_schema;
pub mod operation_log_types;
pub mod vcs_integration;

// operation_log module re-exports its submodules
pub use operation_log::{
    ensure_operation_log_schema, get_stream_version, insert_operation_log, query_all_operations,
    query_stream_events, OperationLogEntry, OperationLogError,
};

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
