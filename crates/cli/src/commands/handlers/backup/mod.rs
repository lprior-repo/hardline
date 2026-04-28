//! Backup command handler - Manage database backups.
//!
//! Provides subcommands for creating, listing, restoring, and managing
//! retention of database backups (state.db, beads.db).
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): BackupCommand, BackupConfig, BackupMetadata, BackupInfo,
//!   BackupCreateOutput, BackupListOutput, BackupRestoreOutput, BackupRetentionOutput,
//!   RetentionStatusOutput, BackupStatusOutput (inert, serializable)
//! - **Calculations** (`calculations.rs`): validate_backup_command, parse_backup_filename,
//!   generate_backup_filename, get_database_backup_dir, resolve_database_target, backups_to_remove,
//!   format_size, build_retention_status (pure functions, no I/O)
//! - **Actions** (`actions.rs`): execute_backup_command, execute_create, execute_list,
//!   execute_restore, execute_retention, execute_status, create_backup, restore_backup,
//!   compute_checksum (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp backup create                      Create backups of all databases
//! scp backup list                        List all available backups
//! scp backup restore state.db            Restore latest backup of state.db
//! scp backup restore beads.db --timestamp 20250101-010101  Restore specific backup
//! scp backup retention                   Apply retention policy
//! scp backup status                      Show backup status and retention info
//! ```

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![forbid(unsafe_code)]

pub mod actions;
pub mod calculations;
pub mod data;

#[cfg(test)]
mod tests;

// Re-export public API
pub use actions::execute_backup_command;
pub use data::BackupCommand;
