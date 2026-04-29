//! Dispatch for workspace subcommands.
//!
//! Wires ADR-015 security: scope checks and audit logging for workspace
//! mutations. Scope violations are advisory (warnings only) in
//! local-only / anonymous mode.

use tracing::warn;

use scp_core::{
    output::Output, AuditEntry, AuditLogger, AuditOutcome, AuthContext, Error, OutputFormat,
    Result, Scope,
};

use crate::{cli::workspace_args::WorkspaceCommands, commands};

// ========================================================================
// Security helpers (shared with dispatch.rs pattern)
// ========================================================================

fn workspace_auth(scopes: Vec<Scope>) -> AuthContext {
    let agent_id = std::env::var("HD_AGENT_ID")
        .or_else(|_| std::env::var("SCP_AGENT_ID"))
        .unwrap_or_else(|_| "anonymous".to_string());
    AuthContext::anonymous_with_scopes(agent_id, scopes)
}

fn workspace_warn_scope(ctx: &AuthContext, required: &Scope, action: &str) {
    if ctx.is_anonymous() && !ctx.has_scope(required) {
        warn!(
            agent = %ctx.agent_id,
            action = action,
            required = required.as_str(),
            "Scope check advisory: anonymous context lacks required scope (local-only mode)"
        );
    }
}

fn workspace_audit(action: &str, resource: &str, agent_id: &scp_core::AgentId) {
    let entry = AuditEntry {
        timestamp: chrono::Utc::now(),
        agent_id: agent_id.clone(),
        action: action.to_string(),
        resource: resource.to_string(),
        outcome: AuditOutcome::Success,
    };
    let log_path = std::path::PathBuf::from(".hd/audit.jsonl");
    let logger = AuditLogger::new(log_path);
    if let Err(e) = logger.log(&entry) {
        warn!(error = %e, "Failed to write audit log entry");
    }
}

// ========================================================================
// Command handler extraction functions
// ========================================================================

fn handle_spawn(name: String, sync: bool) -> Result<()> {
    let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
    workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.spawn");
    workspace_audit("workspace.spawn", &name, &ctx.agent_id);
    commands::workspace::spawn(&name, commands::workspace::SyncOption::from_bool(sync))
}

struct HandleDoneArgs {
    name: Option<String>,
    message: Option<String>,
    keep_workspace: bool,
    squash: bool,
    dry_run: bool,
    detect_conflicts: bool,
    no_bead_update: bool,
}

fn handle_done(args: HandleDoneArgs) -> Result<()> {
    let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
    workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.done");
    let ws_name = args.name.as_deref().unwrap_or("unknown");
    workspace_audit("workspace.done", ws_name, &ctx.agent_id);
    let options = commands::handlers::done::DoneOptions {
        workspace: args.name,
        message: args.message,
        keep_workspace: args.keep_workspace,
        squash: args.squash,
        dry_run: args.dry_run,
        detect_conflicts: args.detect_conflicts,
        no_bead_update: args.no_bead_update,
    };
    commands::handlers::done::run_done(&options)?;
    Ok(())
}

fn handle_abort(name: Option<String>) -> Result<()> {
    let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
    workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.abort");
    let ws_name = name.as_deref().unwrap_or("unknown");
    workspace_audit("workspace.abort", ws_name, &ctx.agent_id);
    commands::workspace::abort(name.as_deref())
}

fn handle_branch_delete(name: String) -> Result<()> {
    let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
    workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.branch_delete");
    workspace_audit("workspace.branch_delete", &name, &ctx.agent_id);
    commands::workspace::branch_delete(&name)
}

fn handle_integrity(subcommand: commands::handlers::integrity::IntegritySubcommand) -> Result<()> {
    commands::handlers::integrity::run_integrity(&subcommand)
}

fn handle_recover(
    target: Option<String>,
    diagnose: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let options = commands::handlers::recover::RecoverOptions {
        diagnose_only: diagnose,
        target,
        dry_run,
        verbose,
    };
    let output = commands::handlers::recover::run_recover(&options)?;
    if verbose {
        Output::info(&format!("Status: {}", output.status));
        Output::info(&format!(
            "Fixed: {}, Remaining: {}",
            output.fixed_count, output.remaining_count
        ));
    }
    Ok(())
}

fn handle_rollback(session: String, commit: String, dry_run: bool) -> Result<()> {
    let options = commands::handlers::recover::RollbackOptions {
        session,
        commit,
        dry_run,
    };
    let output = commands::handlers::recover::run_rollback(&options)?;
    if output.succeeded {
        Output::success(&output.message);
    } else {
        Output::info(&output.message);
    }
    Ok(())
}

fn handle_query(
    query_type: String,
    argument: Option<String>,
    status: Option<String>,
    agent: Option<String>,
) -> Result<()> {
    let qt = commands::handlers::query::data::QueryType::from_str(&query_type).ok_or_else(|| {
        Error::validation_error(format!("Unknown query type: {query_type}"))
    })?;
    let options = commands::handlers::query::QueryOptions {
        query_type: qt,
        argument,
        status_filter: status,
        agent_filter: agent,
    };
    commands::handlers::query::run_query(&options)
}

fn handle_clean(dry_run: bool, force: bool, verbose: bool) -> Result<()> {
    let options = commands::handlers::clean::CleanOptions {
        dry_run,
        force,
        verbose,
    };
    let output = commands::handlers::clean::run_clean(&options)?;
    if verbose {
        Output::info(&format!(
            "Removed {} of {} stale items",
            output.removed_count, output.stale_count
        ));
    }
    Ok(())
}

fn handle_bookmark(command: &crate::cli::workspace_args::BookmarkCommands) -> Result<()> {
    use commands::handlers::bookmark::BookmarkSubcommand;
    let subcmd = match command {
        crate::cli::workspace_args::BookmarkCommands::Create { name } => {
            BookmarkSubcommand::Create { name: name.clone(), push: false }
        }
        crate::cli::workspace_args::BookmarkCommands::List => {
            BookmarkSubcommand::List { show_all: false }
        }
        crate::cli::workspace_args::BookmarkCommands::Delete { name } => {
            BookmarkSubcommand::Delete { name: name.clone() }
        }
        crate::cli::workspace_args::BookmarkCommands::Track { name } => {
            BookmarkSubcommand::Track { name: name.clone(), remote: None }
        }
    };
    let options = commands::handlers::bookmark::BookmarkOptions { subcommand: subcmd };
    commands::handlers::bookmark::run_bookmark(&options)?;
    Ok(())
}

struct HandleWorkArgs {
    name: Option<String>,
    bead: Option<String>,
    agent: Option<String>,
    no_agent: bool,
    idempotent: bool,
    dry_run: bool,
}

fn handle_work(args: HandleWorkArgs) -> Result<()> {
    let mode = if args.dry_run {
        commands::handlers::work::WorkMode::DryRun
    } else if args.idempotent {
        commands::handlers::work::WorkMode::Idempotent
    } else {
        commands::handlers::work::WorkMode::Normal
    };
    let options = commands::handlers::work::WorkOptions {
        name: args.name.unwrap_or_default(),
        bead_id: args.bead,
        agent_id: args.agent,
        no_agent: args.no_agent,
        mode,
        format: OutputFormat::Json,
    };
    commands::handlers::work::run_work(&options)
}

fn handle_wait(condition: String, timeout: u64, poll_interval: u64) -> Result<()> {
    let options = commands::handlers::wait::WaitOptions {
        condition: commands::handlers::wait::WaitCondition::SessionExists(condition),
        timeout: std::time::Duration::from_secs(timeout),
        poll_interval: std::time::Duration::from_secs(poll_interval),
    };
    let output = commands::handlers::wait::run_wait(&options)?;
    if output.timed_out {
        Output::warn(&format!("Timed out waiting for: {}", output.condition));
    }
    Ok(())
}

fn handle_undo(dry_run: bool, list: bool) -> Result<()> {
    let mode = if list {
        commands::handlers::undo::UndoMode::ListHistory
    } else if dry_run {
        commands::handlers::undo::UndoMode::DryRun
    } else {
        commands::handlers::undo::UndoMode::Execute
    };
    let options = commands::handlers::undo::UndoOptions { mode };
    let output = commands::handlers::undo::run_undo(&options)?;
    if output.pushed_to_remote {
        Output::info("Changes pushed to remote");
    }
    Ok(())
}

fn handle_checkpoint(command: &crate::cli::workspace_args::CheckpointCommands) -> Result<()> {
    use commands::handlers::checkpoint::CheckpointAction;
    let action = match command {
        crate::cli::workspace_args::CheckpointCommands::Create { message } => {
            CheckpointAction::Create {
                description: message.clone(),
            }
        }
        crate::cli::workspace_args::CheckpointCommands::Restore { id } => {
            CheckpointAction::Restore { checkpoint_id: id.clone() }
        }
        crate::cli::workspace_args::CheckpointCommands::List => CheckpointAction::List,
    };
    let options = commands::handlers::checkpoint::CheckpointOptions {
        action,
        format: OutputFormat::Json,
    };
    commands::handlers::checkpoint::run_checkpoint(&options)
}

fn handle_prune(yes: bool, dry_run: bool) -> Result<()> {
    let options = commands::handlers::prune::PruneOptions::from_cli(yes, dry_run);
    let output = commands::handlers::prune::run_prune(&options)?;
    if dry_run {
        Output::info(&format!(
            "Would prune {} invalid items",
            output.invalid_count
        ));
    }
    Ok(())
}

fn handle_schema(name: Option<String>, list: bool, all: bool) -> Result<()> {
    let mode = if list {
        commands::handlers::schema::SchemaMode::List
    } else if all {
        commands::handlers::schema::SchemaMode::All
    } else if let Some(schema_name) = name {
        commands::handlers::schema::SchemaMode::Single(schema_name)
    } else {
        commands::handlers::schema::SchemaMode::List
    };
    let options = commands::handlers::schema::SchemaOptions {
        mode,
        format: OutputFormat::Json,
    };
    commands::handlers::schema::run_schema(&options)
}

// ========================================================================
// Sub-dispatch functions (each handles a category of WorkspaceCommands)
// ========================================================================

/// Core workspace lifecycle: spawn, switch, list, status, sync, done, abort,
/// log, diff, uncommitted, commit.
fn dispatch_core(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Spawn { name, sync } => {
            Some(handle_spawn(name.clone(), *sync))
        }
        WorkspaceCommands::Switch { name } => Some(commands::workspace::switch(name)),
        WorkspaceCommands::List => Some(commands::workspace::list()),
        WorkspaceCommands::Status => Some(commands::workspace::status()),
        WorkspaceCommands::Sync { name, all } => {
            Some(commands::workspace::sync(name.as_deref(), *all))
        }
        WorkspaceCommands::Done {
            name,
            message,
            keep_workspace,
            squash,
            dry_run,
            detect_conflicts,
            no_bead_update,
        } => Some(handle_done(HandleDoneArgs {
            name: name.clone(),
            message: message.clone(),
            keep_workspace: *keep_workspace,
            squash: *squash,
            dry_run: *dry_run,
            detect_conflicts: *detect_conflicts,
            no_bead_update: *no_bead_update,
        })),
        WorkspaceCommands::Abort { name } => Some(handle_abort(name.clone())),
        WorkspaceCommands::Log { limit } => Some(commands::workspace::log(*limit)),
        WorkspaceCommands::Diff { path } => Some(commands::workspace::diff(path.as_deref())),
        WorkspaceCommands::Uncommitted => Some(commands::workspace::uncommitted()),
        WorkspaceCommands::Commit { message } => Some(commands::workspace::commit(message)),
        _ => None,
    }
}

/// Branch operations: branches, branch create, branch delete, branch current,
/// branch rename.
fn dispatch_branch(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Branches => Some(commands::workspace::branches()),
        WorkspaceCommands::Branch { name } => Some(commands::workspace::branch_create(name)),
        WorkspaceCommands::BranchDelete { name } => Some(handle_branch_delete(name.clone())),
        WorkspaceCommands::BranchCurrent => Some(commands::workspace::branch_current()),
        WorkspaceCommands::BranchRename {
            old_name,
            new_name,
            dry_run,
        } => {
            let options = commands::handlers::branch::BranchRenameOptions {
                old_name: old_name.clone(),
                new_name: new_name.clone(),
                dry_run: *dry_run,
            };
            Some(commands::handlers::branch::run_branch_rename(&options))
        }
        _ => None,
    }
}

/// Workspace mutation operations: add, fork, merge, revert, rename.
fn dispatch_workspace_ops(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Add { path } => Some(commands::workspace::add(path)),
        WorkspaceCommands::Fork { name, from } => {
            Some(commands::workspace::fork(name, from.as_deref()))
        }
        WorkspaceCommands::Merge { name } => Some(commands::workspace::merge(name)),
        WorkspaceCommands::Revert { name, dry_run } => {
            let options = commands::handlers::revert::RevertOptions {
                session_name: name.clone(),
                dry_run: *dry_run,
            };
            Some(commands::handlers::revert::run_revert(&options).map(|_| ()))
        }
        WorkspaceCommands::Rename {
            old_name,
            new_name,
            dry_run,
        } => {
            let options = commands::handlers::rename::RenameOptions {
                old_name: old_name.clone(),
                new_name: new_name.clone(),
                dry_run: *dry_run,
            };
            Some(commands::handlers::rename::run_rename(&options).map(|_| ()))
        }
        _ => None,
    }
}

/// Integrity operations: validate, repair, backup list, backup restore.
fn dispatch_integrity(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    let subcommand = match cmd {
        WorkspaceCommands::IntegrityValidate { workspace } => {
            Some(commands::handlers::integrity::IntegritySubcommand::Validate {
                workspace: workspace.clone(),
            })
        }
        WorkspaceCommands::IntegrityRepair { workspace, force } => {
            Some(commands::handlers::integrity::IntegritySubcommand::Repair {
                workspace: workspace.clone(),
                force: *force,
            })
        }
        WorkspaceCommands::IntegrityBackupList => {
            Some(commands::handlers::integrity::IntegritySubcommand::BackupList)
        }
        WorkspaceCommands::IntegrityBackupRestore { backup_id, force } => {
            Some(commands::handlers::integrity::IntegritySubcommand::BackupRestore {
                backup_id: backup_id.clone(),
                force: *force,
            })
        }
        _ => return None,
    };
    Some(handle_integrity(subcommand.unwrap()))
}

/// Recovery operations: recover, rollback.
fn dispatch_recovery(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Recover {
            target,
            diagnose,
            dry_run,
            verbose,
        } => Some(handle_recover(
            target.clone(),
            *diagnose,
            *dry_run,
            *verbose,
        )),
        WorkspaceCommands::Rollback {
            session,
            commit,
            dry_run,
        } => Some(handle_rollback(session.clone(), commit.clone(), *dry_run)),
        _ => None,
    }
}

/// Data I/O operations: export, import, contract, validate.
fn dispatch_data_io(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Export { session, output } => {
            let options = commands::handlers::export_import::ExportOptions {
                session: session.clone(),
                output: output.clone(),
            };
            Some(commands::handlers::export_import::run_export(&options))
        }
        WorkspaceCommands::Import {
            input,
            force,
            skip_existing,
            dry_run,
        } => {
            let options = commands::handlers::export_import::ImportOptions {
                input: input.clone(),
                force: *force,
                skip_existing: *skip_existing,
                dry_run: *dry_run,
            };
            Some(commands::handlers::export_import::run_import(&options))
        }
        WorkspaceCommands::Contract { command } => {
            let options = commands::handlers::contract::ContractOptions {
                command: command.clone(),
            };
            Some(commands::handlers::contract::run_contract(&options))
        }
        WorkspaceCommands::Validate {
            command,
            args,
            dry_run,
        } => {
            let options = commands::handlers::validate::ValidateOptions {
                command: command.clone(),
                args: args.clone(),
                dry_run: *dry_run,
            };
            Some(commands::handlers::validate::run_validate(&options))
        }
        _ => None,
    }
}

/// Query and events operations: query, can_i, events.
fn dispatch_query_events(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Query {
            query_type,
            argument,
            status,
            agent,
        } => Some(handle_query(
            query_type.clone(),
            argument.clone(),
            status.clone(),
            agent.clone(),
        )),
        WorkspaceCommands::CanI { action, resource } => {
            let options = commands::handlers::can_i::CanIOptions {
                action: action.clone(),
                resource: resource.clone(),
            };
            Some(commands::handlers::can_i::run_can_i(&options).map(|_| ()))
        }
        WorkspaceCommands::Events {
            session,
            event_type,
            follow,
            limit,
        } => {
            let options = commands::handlers::events::EventsOptions {
                session: session.clone(),
                event_type: event_type.clone(),
                follow: *follow,
                limit: *limit,
                since: None,
            };
            Some(commands::handlers::events::run_events(&options))
        }
        _ => None,
    }
}

/// Session operations: clean, bookmark, work, whoami, wait, undo, checkpoint.
fn dispatch_session_ops(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Clean {
            dry_run,
            force,
            verbose,
            ..
        } => Some(handle_clean(*dry_run, *force, *verbose)),
        WorkspaceCommands::Bookmark { command } => Some(handle_bookmark(command)),
        WorkspaceCommands::Work {
            name,
            bead,
            agent,
            no_agent,
            idempotent,
            dry_run,
        } => Some(handle_work(HandleWorkArgs {
            name: name.clone(),
            bead: bead.clone(),
            agent: agent.clone(),
            no_agent: *no_agent,
            idempotent: *idempotent,
            dry_run: *dry_run,
        })),
        WorkspaceCommands::Whoami { json } => {
            let options = commands::handlers::whoami::WhoamiOptions { json: *json };
            Some(commands::handlers::whoami::run_whoami(&options))
        }
        WorkspaceCommands::Wait {
            condition,
            timeout,
            poll_interval,
        } => Some(handle_wait(condition.clone(), *timeout, *poll_interval)),
        WorkspaceCommands::Undo { dry_run, list } => Some(handle_undo(*dry_run, *list)),
        WorkspaceCommands::Checkpoint { command } => Some(handle_checkpoint(command)),
        _ => None,
    }
}

/// Tooling operations: introspect, completions, prune, schema.
fn dispatch_tooling(cmd: &WorkspaceCommands) -> Option<Result<()>> {
    match cmd {
        WorkspaceCommands::Introspect { target } => {
            let options =
                commands::handlers::introspect::IntrospectOptions::from_cli(target.clone());
            Some(commands::handlers::introspect::run_introspect(&options))
        }
        WorkspaceCommands::Completions { shell } => {
            let shell_type = shell
                .parse::<commands::handlers::completions::Shell>()
                .map_err(|e| Error::validation_error(e.to_string()));
            match shell_type {
                Ok(s) => {
                    let options = commands::handlers::completions::CompletionsOptions { shell: s };
                    Some(commands::handlers::completions::run_completions(&options))
                }
                Err(e) => Some(Err(e)),
            }
        }
        WorkspaceCommands::Prune { yes, dry_run } => Some(handle_prune(*yes, *dry_run)),
        WorkspaceCommands::Schema { name, list, all } => {
            Some(handle_schema(name.clone(), *list, *all))
        }
        _ => None,
    }
}

// ========================================================================
// Main dispatch
// ========================================================================

pub fn run(cmd: WorkspaceCommands) -> Result<()> {
    dispatch_core(&cmd)
        .or_else(|| dispatch_branch(&cmd))
        .or_else(|| dispatch_workspace_ops(&cmd))
        .or_else(|| dispatch_integrity(&cmd))
        .or_else(|| dispatch_recovery(&cmd))
        .or_else(|| dispatch_data_io(&cmd))
        .or_else(|| dispatch_query_events(&cmd))
        .or_else(|| dispatch_session_ops(&cmd))
        .or_else(|| dispatch_tooling(&cmd))
        .ok_or_else(|| {
            Error::from(scp_core::error_internal::InternalErrorKind::Internal(
                "unhandled WorkspaceCommands variant".to_string(),
            ))
        })?
}
