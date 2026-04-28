//! Session command handler - Dispatches session subcommands
//!
//! This module provides the main dispatcher for `scp session <action>` commands,
//! delegating to existing command implementations or providing custom handling.

use anyhow::Result;
use clap::ArgMatches;
use scp_core::{output::Output, vcs, Error};

use super::json_format::get_format;
use crate::commands::session;

// =============================================================================
// Sync functions (from original handlers/session.rs)
// =============================================================================

/// Pause an active session.
pub fn pause(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::session(format!("session '{name}' not found")));
    }

    Err(Error::unimplemented(
        "session pause: session state persistence is not yet implemented",
    ))
}

/// Resume a paused session.
pub fn resume(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::session(format!("session '{name}' not found")));
    }

    Err(Error::unimplemented(
        "session resume: session state persistence is not yet implemented",
    ))
}

/// Result of a clone operation.
#[derive(Debug, Clone)]
pub struct CloneResult {
    pub success: bool,
    pub source: String,
    pub target: String,
    pub dry_run: bool,
}

/// Clone a session.
pub fn clone_session(source: &str, target: &str, dry_run: bool) -> Result<CloneResult> {
    if source.is_empty() {
        return Err(Error::invalid_identifier(
            "source session name cannot be empty".to_string(),
        ));
    }
    if target.is_empty() {
        return Err(Error::invalid_identifier(
            "target session name cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    // Check source exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == source) {
        return Err(Error::workspace_not_found(source.to_string()));
    }

    // Check target doesn't exist
    if workspaces.iter().any(|w| w.name == target) {
        return Err(Error::workspace_exists(target.to_string()));
    }

    if dry_run {
        Output::info(&format!(
            "[dry-run] Would clone '{}' to '{}'",
            source, target
        ));
        return Ok(CloneResult {
            success: true,
            source: source.to_string(),
            target: target.to_string(),
            dry_run: true,
        });
    }

    Output::info(&format!("Cloning '{}' to '{}'...", source, target));

    backend.fork_workspace(source, target)?;

    Output::success(&format!("Cloned '{}' to '{}'", source, target));
    Ok(CloneResult {
        success: true,
        source: source.to_string(),
        target: target.to_string(),
        dry_run: false,
    })
}

// =============================================================================
// Async handlers
// =============================================================================

/// Handle session list subcommand
async fn handle_session_list(args: &ArgMatches) -> Result<()> {
    let _format = get_format(args);
    let include_all = args.get_flag("all");
    let verbose = args.get_flag("verbose");
    let bead = args.get_one::<String>("bead").map(String::as_str);
    let _agent = args.get_one::<String>("agent").map(String::as_str);
    let _state = args.get_one::<String>("state").map(String::as_str);

    // Note: bead, agent, state filters not yet implemented in hardline
    if include_all || verbose || bead.is_some() {
        tracing::warn!("session list --all/--verbose/--bead flags not yet implemented");
    }

    session::list()?;
    Ok(())
}

/// Handle session add subcommand
async fn handle_session_add(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

    let format = get_format(args);
    let _dry_run = args.get_flag("dry-run");
    let _no_open = args.get_flag("no-open");
    let _no_hooks = args.get_flag("no-hooks");

    // Note: add options not fully implemented - hardline uses workspace add(path)
    if format.is_json() {
        serde_json::json!({
            "command": "session add",
            "name": name,
            "status": "unimplemented",
            "message": "session add not yet implemented - use workspace split instead"
        })
        .println();
    } else {
        println!("session add not yet implemented - use 'scp workspace split' instead");
    }

    Ok(())
}

/// Handle session remove subcommand
async fn handle_session_remove(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let _format = get_format(args);
    let _force = args.get_flag("force");

    session::remove(name, false, false)?;
    Ok(())
}

/// Handle session pause subcommand
async fn handle_session_pause(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);

    pause(name)?;

    if format.is_json() {
        serde_json::json!({
            "command": "session pause",
            "session": name,
            "status": "unimplemented"
        })
        .println();
    }

    Ok(())
}

/// Handle session resume subcommand
async fn handle_session_resume(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);

    resume(name)?;

    if format.is_json() {
        serde_json::json!({
            "command": "session resume",
            "session": name,
            "status": "unimplemented"
        })
        .println();
    }

    Ok(())
}

/// Handle session clone subcommand
async fn handle_session_clone(args: &ArgMatches) -> Result<()> {
    let source = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Source session name is required"))?;

    let target = args
        .get_one::<String>("new-name")
        .cloned()
        .unwrap_or_else(|| format!("{source}-copy"));

    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");

    let result = clone_session(source, &target, dry_run)?;

    if format.is_json() {
        serde_json::json!({
            "command": "session clone",
            "source": result.source,
            "target": result.target,
            "success": result.success,
            "dry_run": result.dry_run
        })
        .println();
    }

    Ok(())
}

/// Handle session status subcommand
async fn handle_session_status(args: &ArgMatches) -> Result<()> {
    let _format = get_format(args);

    session::status()?;

    Ok(())
}

/// Handle session focus subcommand
async fn handle_session_focus(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);

    session::focus(name)?;

    if format.is_json() {
        serde_json::json!({
            "command": "session focus",
            "session": name,
            "status": "focused"
        })
        .println();
    }

    Ok(())
}

/// Handle session submit subcommand
async fn handle_session_submit(args: &ArgMatches) -> Result<()> {
    let name = args.get_one::<String>("name").map(String::as_str);
    let auto_commit = args.get_flag("auto-commit");
    let message = args.get_one::<String>("message").map(String::as_str);

    session::submit(name, auto_commit, message)?;

    Ok(())
}

/// Handle session sync subcommand
async fn handle_session_sync(args: &ArgMatches) -> Result<()> {
    let name = args.get_one::<String>("name").map(String::as_str);
    let all = args.get_flag("all");

    let options = crate::commands::handlers::sync::SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: crate::commands::handlers::sync::RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
        },
    };

    let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.handle().clone()
    });

    handle.block_on(async {
        if all {
            crate::commands::handlers::sync::sync_all_sessions(options).await
        } else if let Some(n) = name {
            let session_name = scp_core::domain::SessionName::parse(n).map_err(|e| {
                crate::commands::handlers::sync::SyncError::InvalidIdentifier(e.to_string())
            })?;
            crate::commands::handlers::sync::sync_named_session(session_name, options).await
        } else {
            crate::commands::handlers::sync::sync_current_workspace(options).await
        }
    })?;

    Ok(())
}

/// Handle session init subcommand
async fn handle_session_init(args: &ArgMatches) -> Result<()> {
    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");

    if dry_run {
        if format.is_json() {
            serde_json::json!({
                "command": "session init",
                "dry_run": true,
                "status": "would_initialize"
            })
            .println();
        } else {
            println!("[dry-run] Would initialize session repository");
        }
        return Ok(());
    }

    crate::commands::init::run("git")?;

    if format.is_json() {
        serde_json::json!({
            "command": "session init",
            "vcs_type": "git",
            "status": "initialized"
        })
        .println();
    }

    Ok(())
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Main session command dispatcher
///
/// Routes `scp session <action>` commands to their handlers.
pub async fn handle_session(args: &ArgMatches) -> Result<()> {
    match args.subcommand() {
        Some(("list", sub_args)) => handle_session_list(sub_args).await,
        Some(("add", sub_args)) => handle_session_add(sub_args).await,
        Some(("remove", sub_args)) => handle_session_remove(sub_args).await,
        Some(("pause", sub_args)) => handle_session_pause(sub_args).await,
        Some(("resume", sub_args)) => handle_session_resume(sub_args).await,
        Some(("clone", sub_args)) => handle_session_clone(sub_args).await,
        Some(("status", sub_args)) => handle_session_status(sub_args).await,
        Some(("focus", sub_args)) => handle_session_focus(sub_args).await,
        Some(("submit", sub_args)) => handle_session_submit(sub_args).await,
        Some(("sync", sub_args)) => handle_session_sync(sub_args).await,
        Some(("init", sub_args)) => handle_session_init(sub_args).await,
        _ => {
            // No subcommand - show help
            let format = get_format(args);
            if format.is_json() {
                let help_json = serde_json::json!({
                    "command": "session",
                    "subcommands": [
                        {"name": "list", "description": "List all sessions"},
                        {"name": "add", "description": "Create a new session"},
                        {"name": "remove", "description": "Remove a session"},
                        {"name": "pause", "description": "Pause a session"},
                        {"name": "resume", "description": "Resume a paused session"},
                        {"name": "clone", "description": "Clone a session"},
                        {"name": "status", "description": "Show session status"},
                        {"name": "focus", "description": "Switch to a session"},
                        {"name": "submit", "description": "Submit session changes for review"},
                        {"name": "sync", "description": "Sync session with remote"},
                        {"name": "init", "description": "Initialize SCP in repository"},
                    ]
                });
                println!("{}", serde_json::to_string_pretty(&help_json)?);
            } else {
                println!("Session management commands:");
                println!();
                println!("  scp session list                      List all sessions");
                println!("  scp session add <name>                Create a new session");
                println!("  scp session remove <name>             Remove a session");
                println!("  scp session pause [name]              Pause a session");
                println!("  scp session resume [name]              Resume a paused session");
                println!("  scp session clone <name>              Clone a session");
                println!("  scp session status                    Show session status");
                println!("  scp session focus <name>               Switch to a session");
                println!("  scp session submit                    Submit session for review");
                println!("  scp session sync [name]               Sync session with remote");
                println!("  scp session init                      Initialize SCP in repository");
                println!();
                println!("Run 'scp session <command> --help' for more information.");
            }
            Ok(())
        }
    }
}
