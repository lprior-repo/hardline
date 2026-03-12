//! Core Infrastructure Layer - External integrations and I/O
//!
//! This module contains:
//! - Database access (SQLite, etc.)
//! - File system operations
//! - External API clients
//! - JJ/Git VCS integration

pub mod chaos;
pub mod database;
pub mod operation_log;
pub mod vcs_integration;

pub use chaos::{
    ChaosConfig, ChaosDatabaseService, ChaosFs, ChaosInjector, ChaosNetworkService, NetworkService,
};
pub use database::{
    create_database_service, DatabaseConfig, DatabaseService, SqliteDatabaseService,
};
pub use operation_log::{OperationLogEntry, OperationLogError};
pub use vcs_integration::{
    create_vcs_integration_service, VcsIntegrationService, VcsIntegrationServiceImpl,
};
