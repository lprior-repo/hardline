//! Checkpoint, undo, revert, recover, and rollback handlers
//!
//! Adapted from the isolate project to hardline's architecture.
//! Hardline uses Git exclusively (no JJ), so recovery operations
//! use `git reset`, `git worktree`, etc.

use anyhow::Result;
use clap::ArgMatches;

use crate::commands::handlers::{checkpoint, json_format::get_format, recover, revert, undo};

/// Handle the checkpoint command and its subcommands (create, restore, list).
pub async fn handle_checkpoint(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let action = match sub_m.subcommand() {
        Some(("create", create_m)) => checkpoint::CheckpointAction::Create {
            description: create_m.get_one::<String>("description").cloned(),
        },
        Some(("restore", restore_m)) => checkpoint::CheckpointAction::Restore {
            checkpoint_id: restore_m
                .get_one::<String>("checkpoint_id")
                .ok_or_else(|| anyhow::anyhow!("Checkpoint ID is required"))?
                .clone(),
        },
        Some(("list", _)) => checkpoint::CheckpointAction::List,
        _ => anyhow::bail!("Unknown checkpoint subcommand"),
    };
    let options = checkpoint::CheckpointOptions { action, format };
    checkpoint::run_checkpoint(&options).map_err(anyhow::Error::new)
}

/// Handle the undo command (revert the most recent session merge).
pub async fn handle_undo(sub_m: &ArgMatches) -> Result<()> {
    use undo::{UndoMode, UndoOptions};

    let _format = get_format(sub_m);
    let mode = if sub_m.get_flag("list") {
        UndoMode::ListHistory
    } else if sub_m.get_flag("dry-run") {
        UndoMode::DryRun
    } else {
        UndoMode::Execute
    };
    let options = UndoOptions { mode };
    undo::run_undo(&options)
        .map(|_output| ())
        .map_err(anyhow::Error::new)
}

/// Handle the revert command (revert a specific session merge).
pub async fn handle_revert(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let _format = get_format(sub_m);
    let options = revert::RevertOptions {
        session_name: name.clone(),
        dry_run: sub_m.get_flag("dry-run"),
    };
    revert::run_revert(&options)
        .map(|_output| ())
        .map_err(anyhow::Error::new)
}

/// Handle the recover command (auto-detect and fix common broken states).
pub async fn handle_recover(sub_m: &ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);

    let diagnose_only = sub_m.get_flag("diagnose");
    let target = sub_m.get_one::<String>("session").cloned();

    let options = recover::RecoverOptions {
        diagnose_only,
        target,
        dry_run: false,
        verbose: false,
    };
    recover::run_recover(&options)
        .map(|_output| ())
        .map_err(anyhow::Error::new)
}

/// Handle the rollback command (rollback a workspace to a specific commit).
pub async fn handle_rollback(sub_m: &ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("session")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
    let commit = sub_m
        .get_one::<String>("to")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Checkpoint/commit name is required"))?;
    let dry_run = sub_m.get_flag("dry-run");
    let options = recover::RollbackOptions {
        session,
        commit,
        dry_run,
    };
    let output = recover::run_rollback(&options).map_err(anyhow::Error::new)?;
    if !output.succeeded && !dry_run {
        std::process::exit(1);
    }
    Ok(())
}
