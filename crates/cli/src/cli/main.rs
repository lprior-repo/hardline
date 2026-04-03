//! CLI execution logic
//!
//! Contains the main entry point and command dispatch logic.

use crate::cli::args::Cli;
use crate::commands;
use clap::Parser;
use scp_core::{output::Output, Result};
use std::process::ExitCode;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Main entry point for the CLI
pub fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up verbosity for output module
    Output::set_verbose(cli.verbose, cli.quiet);

    // Set database path if provided via flag
    if let Some(db_path) = &cli.database {
        std::env::set_var("SCP_DATABASE_PATH", db_path);
    }

    // Initialize logging with appropriate level
    let log_level = if cli.quiet {
        "error".to_string()
    } else if cli.verbose {
        "debug".to_string()
    } else {
        "info".to_string()
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or(log_level),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Run the appropriate command
    let result = run_command(cli);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            if let Some(suggestion) = e.suggestion() {
                eprintln!("{}", suggestion);
            }
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Execute the appropriate command based on CLI arguments
pub fn run_command(cli: Cli) -> Result<()> {
    use crate::cli::args::Commands;

    match cli.command {
        Commands::Init { vcs } => commands::init::run(&vcs),

        Commands::Workspace { command } => match command {
            crate::cli::workspace_args::WorkspaceCommands::Spawn { name, sync } => {
                commands::workspace::spawn(&name, commands::workspace::SyncOption::from_bool(sync))
            }
            crate::cli::workspace_args::WorkspaceCommands::Switch { name } => {
                commands::workspace::switch(&name)
            }
            crate::cli::workspace_args::WorkspaceCommands::List => commands::workspace::list(),
            crate::cli::workspace_args::WorkspaceCommands::Status => commands::workspace::status(),
            crate::cli::workspace_args::WorkspaceCommands::Sync { name, all } => {
                commands::workspace::sync(name.as_deref(), all)
            }
            crate::cli::workspace_args::WorkspaceCommands::Done {
                name,
                message,
                keep_workspace,
                squash,
                dry_run,
                detect_conflicts,
                no_bead_update,
            } => {
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
            crate::cli::workspace_args::WorkspaceCommands::Abort { name } => {
                commands::workspace::abort(name.as_deref())
            }
            crate::cli::workspace_args::WorkspaceCommands::Log { limit } => {
                commands::workspace::log(limit)
            }
            crate::cli::workspace_args::WorkspaceCommands::Diff { path } => {
                commands::workspace::diff(path.as_deref())
            }
            crate::cli::workspace_args::WorkspaceCommands::Uncommitted => {
                commands::workspace::uncommitted()
            }
            crate::cli::workspace_args::WorkspaceCommands::Commit { message } => {
                commands::workspace::commit(&message)
            }
            crate::cli::workspace_args::WorkspaceCommands::Branches => {
                commands::workspace::branches()
            }
            crate::cli::workspace_args::WorkspaceCommands::Branch { name } => {
                commands::workspace::branch_create(&name)
            }
            crate::cli::workspace_args::WorkspaceCommands::BranchDelete { name } => {
                commands::workspace::branch_delete(&name)
            }
            crate::cli::workspace_args::WorkspaceCommands::BranchCurrent => {
                commands::workspace::branch_current()
            }
            crate::cli::workspace_args::WorkspaceCommands::Add { path } => {
                commands::workspace::add(&path)
            }
            crate::cli::workspace_args::WorkspaceCommands::Fork { name, from } => {
                commands::workspace::fork(&name, from.as_deref())
            }
            crate::cli::workspace_args::WorkspaceCommands::Merge { name } => {
                commands::workspace::merge(&name)
            }
            crate::cli::workspace_args::WorkspaceCommands::Revert { name, dry_run } => {
                let options = commands::handlers::revert::RevertOptions {
                    session_name: name,
                    dry_run,
                };
                commands::handlers::revert::run_revert(&options)?;
                Ok(())
            }
            crate::cli::workspace_args::WorkspaceCommands::IntegrityValidate { workspace } => {
                let subcommand = commands::handlers::integrity::IntegritySubcommand::Validate {
                    workspace,
                };
                commands::handlers::integrity::run_integrity(&subcommand)
            }
            crate::cli::workspace_args::WorkspaceCommands::IntegrityRepair { workspace, force } => {
                let subcommand = commands::handlers::integrity::IntegritySubcommand::Repair {
                    workspace,
                    force,
                };
                commands::handlers::integrity::run_integrity(&subcommand)
            }
            crate::cli::workspace_args::WorkspaceCommands::IntegrityBackupList => {
                let subcommand = commands::handlers::integrity::IntegritySubcommand::BackupList;
                commands::handlers::integrity::run_integrity(&subcommand)
            }
            crate::cli::workspace_args::WorkspaceCommands::IntegrityBackupRestore { backup_id, force } => {
                let subcommand = commands::handlers::integrity::IntegritySubcommand::BackupRestore {
                    backup_id,
                    force,
                };
                commands::handlers::integrity::run_integrity(&subcommand)
            }
            crate::cli::workspace_args::WorkspaceCommands::Recover { target, diagnose, dry_run, verbose } => {
                let options = commands::handlers::recover::RecoverOptions {
                    diagnose_only: diagnose,
                    target,
                    dry_run,
                    verbose,
                };
                let output = commands::handlers::recover::run_recover(&options)?;
                if verbose {
                    scp_core::output::Output::info(&format!("Status: {}", output.status));
                    scp_core::output::Output::info(&format!("Fixed: {}, Remaining: {}", output.fixed_count, output.remaining_count));
                }
                Ok(())
            }
            crate::cli::workspace_args::WorkspaceCommands::Rollback { session, commit, dry_run } => {
                let options = commands::handlers::recover::RollbackOptions {
                    session,
                    commit,
                    dry_run,
                };
                let output = commands::handlers::recover::run_rollback(&options)?;
                if output.succeeded {
                    scp_core::output::Output::success(&output.message);
                } else {
                    scp_core::output::Output::info(&output.message);
                }
                Ok(())
            }
            crate::cli::workspace_args::WorkspaceCommands::Rename { old_name, new_name, dry_run } => {
                let options = commands::handlers::rename::RenameOptions {
                    old_name,
                    new_name,
                    dry_run,
                };
                commands::handlers::rename::run_rename(&options)?;
                Ok(())
            }
            crate::cli::workspace_args::WorkspaceCommands::Export { session, output } => {
                let options = commands::handlers::export_import::ExportOptions {
                    session,
                    output,
                };
                commands::handlers::export_import::run_export(&options)
            }
            crate::cli::workspace_args::WorkspaceCommands::Import { input, force, skip_existing, dry_run } => {
                let options = commands::handlers::export_import::ImportOptions {
                    input,
                    force,
                    skip_existing,
                    dry_run,
                };
                commands::handlers::export_import::run_import(&options)
            }
            crate::cli::workspace_args::WorkspaceCommands::Contract { command } => {
                let options = commands::handlers::contract::ContractOptions {
                    command,
                };
                commands::handlers::contract::run_contract(&options)
            }
            crate::cli::workspace_args::WorkspaceCommands::Validate { command, args, dry_run } => {
                let options = commands::handlers::validate::ValidateOptions {
                    command,
                    args,
                    dry_run,
                };
                commands::handlers::validate::run_validate(&options)
            }
            crate::cli::workspace_args::WorkspaceCommands::Query { query_type, argument, status, agent } => {
                let qt = commands::handlers::query::data::QueryType::from_str(&query_type)
                    .ok_or_else(|| scp_core::Error::validation_error(format!("Unknown query type: {query_type}")))?;
                let options = commands::handlers::query::QueryOptions {
                    query_type: qt,
                    argument,
                    status_filter: status,
                    agent_filter: agent,
                };
                commands::handlers::query::run_query(&options)
            }
        },

        Commands::Lock { command } => match command {
            crate::cli::lock_args::LockCommands::Acquire {
                session,
                agent,
                ttl,
            } => commands::lock::acquire(&session, &agent, ttl),
            crate::cli::lock_args::LockCommands::Release { session, agent } => {
                commands::lock::release(&session, &agent)
            }
            crate::cli::lock_args::LockCommands::Heartbeat { session, agent } => {
                commands::lock::heartbeat(&session, &agent)
            }
            crate::cli::lock_args::LockCommands::Status { session } => {
                commands::lock::status(&session)
            }
            crate::cli::lock_args::LockCommands::List => commands::lock::list(),
        },

        Commands::Queue { command } => match command {
            crate::cli::queue_args::QueueCommands::List => commands::queue::list(),
            crate::cli::queue_args::QueueCommands::Enqueue { branch, priority } => {
                commands::queue::enqueue(&branch, priority.as_deref())
            }
            crate::cli::queue_args::QueueCommands::Dequeue => commands::queue::dequeue(),
            crate::cli::queue_args::QueueCommands::Process { checks } => {
                commands::queue::process(checks)
            }
            crate::cli::queue_args::QueueCommands::Insert { position, branch } => {
                commands::queue::insert(position, &branch)
            }
            crate::cli::queue_args::QueueCommands::Remove { branch } => {
                commands::queue::remove(&branch)
            }
            crate::cli::queue_args::QueueCommands::Status => commands::queue::status(),
        },

        Commands::Agent { command } => match command {
            crate::cli::agent_args::AgentCommands::Create { name } => {
                commands::agent::create(&name)
            }
            crate::cli::agent_args::AgentCommands::List => commands::agent::list(),
            crate::cli::agent_args::AgentCommands::Kill { id } => commands::agent::kill(&id),
            crate::cli::agent_args::AgentCommands::Status { id } => {
                commands::agent::status(id.as_deref())
            }
            crate::cli::agent_args::AgentCommands::Register { session } => {
                commands::agent::register(session.as_deref())
            }
            crate::cli::agent_args::AgentCommands::Heartbeat { session } => {
                commands::agent::heartbeat(session.as_deref())
            }
        },

        Commands::Session { command } => match command {
            crate::cli::session_args::SessionCommands::List => commands::session::list(),
            crate::cli::session_args::SessionCommands::Status => commands::session::status(),
            crate::cli::session_args::SessionCommands::Focus { name } => {
                commands::session::focus(&name)
            }
            crate::cli::session_args::SessionCommands::Submit {
                name,
                auto_commit,
                message,
            } => commands::session::submit(name.as_deref(), auto_commit, message.as_deref()),
            crate::cli::session_args::SessionCommands::Remove { name, force, merge } => {
                commands::session::remove(&name, force, merge)
            }
            crate::cli::session_args::SessionCommands::Pause { name } => {
                commands::handlers::session::pause(&name)
            }
            crate::cli::session_args::SessionCommands::Resume { name } => {
                commands::handlers::session::resume(&name)
            }
            crate::cli::session_args::SessionCommands::Clone {
                source,
                target,
                dry_run,
            } => {
                commands::handlers::session::clone_session(&source, &target, dry_run)?;
                Ok(())
            }
        },

        Commands::Task { command } => {
            use commands::handlers::task::{parse_task_id, run_task_command, AgentId, TaskCommand};
            let cmd = match command {
                crate::cli::task_args::TaskCommands::List => TaskCommand::List {
                    status_filter: None,
                    include_all: false,
                },
                crate::cli::task_args::TaskCommands::Show { task_id, .. } => TaskCommand::Show {
                    task_id: parse_task_id(&task_id)?,
                },
                crate::cli::task_args::TaskCommands::Claim { task_id, user } => {
                    TaskCommand::Claim {
                        task_id: parse_task_id(&task_id)?,
                        agent_id: AgentId::new(&user)?,
                    }
                }
                crate::cli::task_args::TaskCommands::Yield { task_id, user } => {
                    TaskCommand::YieldTask {
                        task_id: parse_task_id(&task_id)?,
                        agent_id: AgentId::new(&user)?,
                    }
                }
                crate::cli::task_args::TaskCommands::Start { task_id, user } => {
                    TaskCommand::Start {
                        task_id: parse_task_id(&task_id)?,
                        agent_id: AgentId::new(&user)?,
                    }
                }
                crate::cli::task_args::TaskCommands::Done { task_id, user } => TaskCommand::Done {
                    task_id: Some(parse_task_id(&task_id)?),
                    agent_id: AgentId::new(&user)?,
                },
            };
            run_task_command(&cmd)
        }

        Commands::Config { command } => match command {
            crate::cli::config_args::ConfigCommands::Get { key } => commands::config::get(&key),
            crate::cli::config_args::ConfigCommands::Set { key, value } => {
                commands::config::set(&key, &value)
            }
            crate::cli::config_args::ConfigCommands::List => commands::config::list(),
        },

        Commands::Stash { command } => match command {
            crate::cli::stash_args::StashCommands::Save {
                message,
                include_untracked,
                patch,
            } => commands::stash::save(message.as_deref(), include_untracked, patch),
            crate::cli::stash_args::StashCommands::Pop { stash, index } => {
                commands::stash::pop(stash.as_deref(), index)
            }
            crate::cli::stash_args::StashCommands::List => commands::stash::list(),
            crate::cli::stash_args::StashCommands::Drop { stash, force } => {
                commands::stash::drop(&stash, force)
            }
            crate::cli::stash_args::StashCommands::Show { stash, stat } => {
                commands::stash::show(stash.as_deref(), stat)
            }
        },

        Commands::Tag { command } => match command {
            crate::cli::tag_args::TagCommands::Create {
                name,
                message,
                commit,
                force,
            } => commands::tag::create(&name, message.as_deref(), commit.as_deref(), force),
            crate::cli::tag_args::TagCommands::List { pattern, sort } => {
                commands::tag::list(pattern.as_deref(), sort.as_deref())
            }
            crate::cli::tag_args::TagCommands::Delete { tag, remote } => {
                commands::tag::delete(&tag, remote)
            }
            crate::cli::tag_args::TagCommands::Push { tag, remote, force } => {
                commands::tag::push(tag.as_deref(), &remote, force)
            }
        },

        Commands::Batch { command } => match command {
            crate::cli::batch_args::BatchCommands::Run {
                workspace,
                commands,
            } => tokio::runtime::Handle::current()
                .block_on(commands::batch::execute(workspace, commands)),
        },

        Commands::Fetch {
            remote,
            prune,
            tags,
            all,
        } => commands::sync::fetch(remote.as_deref(), prune, tags, all),

        Commands::Pull => commands::sync::pull(),

        Commands::Push {
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        } => commands::sync::push(
            &remote,
            branch.as_deref(),
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        ),

        Commands::Doctor { full } => commands::doctor::run(full),

        Commands::Status { short } => commands::status::run(short),

        Commands::Switch { name } => commands::workspace::switch(&name),

        Commands::Context => commands::context::run(),

        Commands::Whereami => commands::context::whereami(),
    }
}
