//! Integrity command handler - workspace corruption detection, validation, and repair.
//!
//! This handler ports the integrity command from the isolate project,
//! adapted to hardline's architecture and error handling.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): IntegrityOptions, IntegritySubcommand, response types,
//!   IntegrityOutputFormat (inert, serializable)
//! - **Actions** (`actions.rs`): run_integrity, validation, repair, backup list/restore
//!   (I/O operations delegating to scp_core::workspace_integrity)
//!
//! # Delegation to Core
//!
//! This handler is a thin CLI wrapper over the existing
//! `scp_core::workspace_integrity` module which provides:
//! - `IntegrityValidator` - workspace corruption detection
//! - `RepairExecutor` - auto-repair with backup protection
//! - `BackupManager` - backup creation, listing, and restoration
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace integrity validate my-workspace    # Validate workspace
//! scp workspace integrity repair my-workspace       # Repair workspace
//! scp workspace integrity repair my-ws --force      # Force repair without prompt
//! scp workspace integrity backup-list               # List all backups
//! scp workspace integrity backup-restore <id>       # Restore from backup
//! ```

pub mod actions;
pub mod data;

// Re-export public API
pub use actions::run_integrity;
pub use data::{
    BackupListResponse, IntegrityOptions, IntegrityOutputFormat, IntegritySubcommand,
    RepairResponse, RestoreResponse, ValidationResponse,
};
