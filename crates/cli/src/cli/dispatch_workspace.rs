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

pub(crate) fn run(cmd: WorkspaceCommands) -> Result<()> {
    match cmd {
        WorkspaceCommands::Spawn { name, sync } => {
            let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
            workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.spawn");
            workspace_audit("workspace.spawn", &name, &ctx.agent_id);
            commands::workspace::spawn(&name, commands::workspace::SyncOption::from_bool(sync))
        }
        WorkspaceCommands::Switch { name } => commands::workspace::switch(&name),
        WorkspaceCommands::List => commands::workspace::list(),
        WorkspaceCommands::Status => commands::workspace::status(),
        WorkspaceCommands::Sync { name, all } => commands::workspace::sync(name.as_deref(), all),
        WorkspaceCommands::Done {
            name,
            message,
            keep_workspace,
            squash,
            dry_run,
            detect_conflicts,
            no_bead_update,
        } => {
            let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
            workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.done");
            let ws_name = name.as_deref().unwrap_or("unknown");
            workspace_audit("workspace.done", ws_name, &ctx.agent_id);
            let options = commands::handlers::done::DoneOptions {
                workspace: name,
                message,
                keep_workspace,
                squash,
                dry_run,
                detect_conflicts,
                no_bead_update,
            };
            commands::handlers::done::run_done(&options)?;
            Ok(())
        }
        WorkspaceCommands::Abort { name } => {
            let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
            workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.abort");
            let ws_name = name.as_deref().unwrap_or("unknown");
            workspace_audit("workspace.abort", ws_name, &ctx.agent_id);
            commands::workspace::abort(name.as_deref())
        }
        WorkspaceCommands::Log { limit } => commands::workspace::log(limit),
        WorkspaceCommands::Diff { path } => commands::workspace::diff(path.as_deref()),
        WorkspaceCommands::Uncommitted => commands::workspace::uncommitted(),
        WorkspaceCommands::Commit { message } => commands::workspace::commit(&message),
        WorkspaceCommands::Branches => commands::workspace::branches(),
        WorkspaceCommands::Branch { name } => commands::workspace::branch_create(&name),
        WorkspaceCommands::BranchDelete { name } => {
            let ctx = workspace_auth(vec![Scope::WriteWorkspace]);
            workspace_warn_scope(&ctx, &Scope::WriteWorkspace, "workspace.branch_delete");
            workspace_audit("workspace.branch_delete", &name, &ctx.agent_id);
            commands::workspace::branch_delete(&name)
        }
        WorkspaceCommands::BranchCurrent => commands::workspace::branch_current(),
        WorkspaceCommands::BranchRename {
            old_name,
            new_name,
            dry_run,
        } => {
            let options = commands::handlers::branch::BranchRenameOptions {
                old_name,
                new_name,
                dry_run,
            };
            commands::handlers::branch::run_branch_rename(&options)
        }
        WorkspaceCommands::Add { path } => commands::workspace::add(&path),
        WorkspaceCommands::Fork { name, from } => commands::workspace::fork(&name, from.as_deref()),
        WorkspaceCommands::Merge { name } => commands::workspace::merge(&name),
        WorkspaceCommands::Revert { name, dry_run } => {
            let options = commands::handlers::revert::RevertOptions {
                session_name: name,
                dry_run,
            };
            commands::handlers::revert::run_revert(&options)?;
            Ok(())
        }
        WorkspaceCommands::IntegrityValidate { workspace } => {
            let subcommand =
                commands::handlers::integrity::IntegritySubcommand::Validate { workspace };
            commands::handlers::integrity::run_integrity(&subcommand)
        }
        WorkspaceCommands::IntegrityRepair { workspace, force } => {
            let subcommand =
                commands::handlers::integrity::IntegritySubcommand::Repair { workspace, force };
            commands::handlers::integrity::run_integrity(&subcommand)
        }
        WorkspaceCommands::IntegrityBackupList => {
            let subcommand = commands::handlers::integrity::IntegritySubcommand::BackupList;
            commands::handlers::integrity::run_integrity(&subcommand)
        }
        WorkspaceCommands::IntegrityBackupRestore { backup_id, force } => {
            let subcommand = commands::handlers::integrity::IntegritySubcommand::BackupRestore {
                backup_id,
                force,
            };
            commands::handlers::integrity::run_integrity(&subcommand)
        }
        WorkspaceCommands::Recover {
            target,
            diagnose,
            dry_run,
            verbose,
        } => {
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
        WorkspaceCommands::Rollback {
            session,
            commit,
            dry_run,
        } => {
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
        WorkspaceCommands::Rename {
            old_name,
            new_name,
            dry_run,
        } => {
            let options = commands::handlers::rename::RenameOptions {
                old_name,
                new_name,
                dry_run,
            };
            commands::handlers::rename::run_rename(&options)?;
            Ok(())
        }
        WorkspaceCommands::Export { session, output } => {
            let options = commands::handlers::export_import::ExportOptions { session, output };
            commands::handlers::export_import::run_export(&options)
        }
        WorkspaceCommands::Import {
            input,
            force,
            skip_existing,
            dry_run,
        } => {
            let options = commands::handlers::export_import::ImportOptions {
                input,
                force,
                skip_existing,
                dry_run,
            };
            commands::handlers::export_import::run_import(&options)
        }
        WorkspaceCommands::Contract { command } => {
            let options = commands::handlers::contract::ContractOptions { command };
            commands::handlers::contract::run_contract(&options)
        }
        WorkspaceCommands::Validate {
            command,
            args,
            dry_run,
        } => {
            let options = commands::handlers::validate::ValidateOptions {
                command,
                args,
                dry_run,
            };
            commands::handlers::validate::run_validate(&options)
        }
        WorkspaceCommands::Query {
            query_type,
            argument,
            status,
            agent,
        } => {
            let qt = commands::handlers::query::data::QueryType::from_str(&query_type).ok_or_else(
                || Error::validation_error(format!("Unknown query type: {query_type}")),
            )?;
            let options = commands::handlers::query::QueryOptions {
                query_type: qt,
                argument,
                status_filter: status,
                agent_filter: agent,
            };
            commands::handlers::query::run_query(&options)
        }
        WorkspaceCommands::CanI { action, resource } => {
            let options = commands::handlers::can_i::CanIOptions { action, resource };
            commands::handlers::can_i::run_can_i(&options)?;
            Ok(())
        }
        WorkspaceCommands::Events {
            session,
            event_type,
            follow,
            limit,
        } => {
            let options = commands::handlers::events::EventsOptions {
                session,
                event_type,
                follow,
                limit,
                since: None,
            };
            commands::handlers::events::run_events(&options)
        }
        WorkspaceCommands::Clean {
            dry_run,
            force,
            verbose,
            ..
        } => {
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
        WorkspaceCommands::Bookmark { command } => {
            use crate::cli::workspace_args::BookmarkCommands;
            let subcmd = match command {
                BookmarkCommands::Create { name } => {
                    commands::handlers::bookmark::BookmarkSubcommand::Create { name, push: false }
                }
                BookmarkCommands::List => {
                    commands::handlers::bookmark::BookmarkSubcommand::List { show_all: false }
                }
                BookmarkCommands::Delete { name } => {
                    commands::handlers::bookmark::BookmarkSubcommand::Delete { name }
                }
                BookmarkCommands::Track { name } => {
                    commands::handlers::bookmark::BookmarkSubcommand::Track { name, remote: None }
                }
            };
            let options = commands::handlers::bookmark::BookmarkOptions { subcommand: subcmd };
            commands::handlers::bookmark::run_bookmark(&options)?;
            Ok(())
        }
        WorkspaceCommands::Work {
            name,
            bead,
            agent,
            no_agent,
            idempotent,
            dry_run,
        } => {
            let mode = if dry_run {
                commands::handlers::work::WorkMode::DryRun
            } else if idempotent {
                commands::handlers::work::WorkMode::Idempotent
            } else {
                commands::handlers::work::WorkMode::Normal
            };
            let options = commands::handlers::work::WorkOptions {
                name: name.unwrap_or_default(),
                bead_id: bead,
                agent_id: agent,
                no_agent,
                mode,
                format: OutputFormat::Json,
            };
            commands::handlers::work::run_work(&options)
        }
        WorkspaceCommands::Whoami { json } => {
            let options = commands::handlers::whoami::WhoamiOptions { json };
            commands::handlers::whoami::run_whoami(&options)
        }
        WorkspaceCommands::Wait {
            condition,
            timeout,
            poll_interval,
        } => {
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
        WorkspaceCommands::Undo { dry_run, list } => {
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
        WorkspaceCommands::Checkpoint { command } => {
            use crate::cli::workspace_args::CheckpointCommands;
            let action = match command {
                CheckpointCommands::Create { message } => {
                    commands::handlers::checkpoint::CheckpointAction::Create {
                        description: message,
                    }
                }
                CheckpointCommands::Restore { id } => {
                    commands::handlers::checkpoint::CheckpointAction::Restore { checkpoint_id: id }
                }
                CheckpointCommands::List => commands::handlers::checkpoint::CheckpointAction::List,
            };
            let options = commands::handlers::checkpoint::CheckpointOptions {
                action,
                format: OutputFormat::Json,
            };
            commands::handlers::checkpoint::run_checkpoint(&options)
        }
        WorkspaceCommands::Introspect { target } => {
            let options = commands::handlers::introspect::IntrospectOptions::from_cli(target);
            commands::handlers::introspect::run_introspect(&options)
        }
        WorkspaceCommands::Completions { shell } => {
            let shell_type = shell
                .parse::<commands::handlers::completions::Shell>()
                .map_err(|e| Error::validation_error(e.to_string()))?;
            let options = commands::handlers::completions::CompletionsOptions { shell: shell_type };
            commands::handlers::completions::run_completions(&options)
        }
        WorkspaceCommands::Prune { yes, dry_run } => {
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
        WorkspaceCommands::Schema { name, list, all } => {
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
    }
}
