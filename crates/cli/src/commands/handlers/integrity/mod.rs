//! Integrity command handler - workspace corruption detection, validation, and repair.
//!
//! This handler ports the integrity command from the isolate project,
//! adapted to hardline's architecture and error handling.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): IntegrityOptions, IntegritySubcommand, response types,
//!   IntegrityOutputFormat (inert, serializable)
//! - **Actions** (`actions.rs`): run_integrity, validation, repair, backup list/restore (I/O
//!   operations delegating to scp_core::workspace_integrity)
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
use anyhow::Result;
use clap::ArgMatches;
pub use data::{
    BackupListResponse, IntegrityOptions, IntegrityOutputFormat, IntegritySubcommand,
    RepairResponse, RestoreResponse, ValidationResponse,
};

use crate::commands::{
    doctor,
    handlers::{
        clean,
        clean::data::CleanOptions,
        json_format::get_format,
        prune,
        prune::data::{PruneMode, PruneOptions},
    },
};

// ============================================================================
// CLI Handlers
// ============================================================================

/// Main handler for the integrity command - routes to subcommands.
pub async fn handle_integrity(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("validate", validate_m)) => {
            let workspace = validate_m
                .get_one::<String>("workspace")
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("Workspace argument is required for validate command")
                })?;
            let subcommand = IntegritySubcommand::Validate { workspace };
            Ok(run_integrity(&subcommand)?)
        }
        Some(("repair", repair_m)) => {
            let workspace = repair_m
                .get_one::<String>("workspace")
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("Workspace argument is required for repair command")
                })?;
            let force = repair_m.get_flag("force");
            let _rebind = repair_m.get_flag("rebind"); // Not supported in hardline
            let _ = _rebind;
            let subcommand = IntegritySubcommand::Repair { workspace, force };
            Ok(run_integrity(&subcommand)?)
        }
        Some(("backup", backup_m)) => match backup_m.subcommand() {
            Some(("list", _list_m)) => {
                let subcommand = IntegritySubcommand::BackupList;
                Ok(run_integrity(&subcommand)?)
            }
            Some(("restore", restore_m)) => {
                let backup_id = restore_m
                    .get_one::<String>("backup_id")
                    .ok_or_else(|| anyhow::anyhow!("Backup ID is required"))?
                    .clone();
                let force = restore_m.get_flag("force");
                let subcommand = IntegritySubcommand::BackupRestore { backup_id, force };
                Ok(run_integrity(&subcommand)?)
            }
            _ => Err(anyhow::anyhow!("Unknown backup subcommand")),
        },
        _ => Err(anyhow::anyhow!("Unknown integrity subcommand")),
    }
}

/// Main doctor command dispatcher - routes to subcommands.
pub async fn handle_doctor(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        // scp doctor check - run all health checks
        Some(("check", check_m)) => {
            let _format = get_format(check_m);
            Ok(doctor::run(false)?)
        }
        // scp doctor fix - auto-fix issues (hardline doesn't support fix mode)
        Some(("fix", fix_m)) => {
            let _format = get_format(fix_m);
            let _dry_run = fix_m.get_flag("dry-run");
            let _verbose = fix_m.get_flag("verbose");
            // hardline's doctor doesn't have fix/dry_run/verbose options
            Ok(doctor::run(false)?)
        }
        // scp doctor integrity - run database integrity check (hardline doesn't have this)
        Some(("integrity", _integrity_m)) => Err(anyhow::anyhow!(
            "Doctor integrity subcommand is not yet implemented in hardline"
        )),
        // scp doctor clean - remove stale sessions
        Some(("clean", clean_m)) => {
            let force = clean_m.get_flag("force");
            let dry_run = clean_m.get_flag("dry-run");
            let options = CleanOptions {
                force,
                dry_run,
                verbose: false,
            };
            Ok(clean::run_clean(&options).map(|_output| ())?)
        }
        // No subcommand - legacy mode (doctor with flags)
        None => {
            let _format = get_format(sub_m);
            let _fix = sub_m.get_flag("fix");
            let _dry_run = sub_m.get_flag("dry-run");
            let _verbose = sub_m.get_flag("verbose");

            // hardline's doctor doesn't support fix/dry_run/verbose flags
            Ok(doctor::run(false)?)
        }
        // Unknown subcommand
        _ => {
            let available = ["check", "fix", "clean"];
            Err(anyhow::anyhow!(
                "Unknown doctor subcommand. Available: {}",
                available.join(", ")
            ))
        }
    }
}

/// Run database integrity check only (PRAGMA integrity_check).
///
/// Note: hardline doesn't have a database-level integrity check like isolate.
/// This function is kept for interface compatibility but returns an error
/// indicating it's not implemented.
async fn run_db_integrity_check(_json_output: bool) -> Result<()> {
    Err(anyhow::anyhow!(
        "Database integrity check is not yet implemented in hardline. \
         hardline uses file-based workspace storage rather than a database."
    ))
}
