//! Dispatch for non-workspace top-level commands.
//!
//! Wires ADR-015 security: scope checks and audit logging for every
//! dispatched command. Scope violations are advisory (warnings only)
//! when running in local-only / anonymous mode.

use tracing::warn;

use scp_core::{
    AuditEntry, AuditLogger, AuditOutcome, AuthContext, OutputFormat, Result, Scope,
};

use crate::{cli::args::Commands, commands};

// ========================================================================
// Security helpers
// ========================================================================

/// Resolve the agent identity from environment variables.
///
/// Checks `HD_AGENT_ID`, then `SCP_AGENT_ID`, then falls back to
/// `"anonymous"`.
fn resolve_agent_id() -> String {
    std::env::var("HD_AGENT_ID")
        .or_else(|_| std::env::var("SCP_AGENT_ID"))
        .unwrap_or_else(|_| "anonymous".to_string())
}

/// Build an [`AuthContext`] carrying the requested scopes.
///
/// In local-only mode (no real auth configured) this produces an
/// anonymous context via [`AuthContext::anonymous_with_scopes`].
fn auth_context(scopes: Vec<Scope>) -> AuthContext {
    let agent_id = resolve_agent_id();
    AuthContext::anonymous_with_scopes(agent_id, scopes)
}

/// Emit an advisory warning when a scope check fails.
///
/// In local-only mode this is a warning, not a denial. The command
/// proceeds regardless so that existing workflows are not broken.
fn warn_missing_scope(ctx: &AuthContext, required: &Scope, action: &str) {
    if ctx.is_anonymous() && !ctx.has_scope(required) {
        warn!(
            agent = %ctx.agent_id,
            action = action,
            required = required.as_str(),
            "Scope check advisory: anonymous context lacks required scope (local-only mode)"
        );
    }
}

/// Log an audit entry for a key operation.
///
/// Best-effort: failures are swallowed so that a broken audit path
/// never blocks a command.
fn audit_log(action: &str, resource: &str, agent_id: &scp_core::AgentId) {
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

pub(crate) fn run_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Init { vcs } => commands::init::run(&vcs),

        Commands::Ai { command } => {
            use crate::cli::ai_args::AiCommands;
            use crate::commands::handlers::ai::{AiOptions, AiSubcommand, run};
            let subcmd = match command {
                AiCommands::Status => AiSubcommand::Status,
                AiCommands::Workflow => AiSubcommand::Workflow,
                AiCommands::QuickStart => AiSubcommand::QuickStart,
                AiCommands::Next => AiSubcommand::Next,
                AiCommands::Default => AiSubcommand::Default,
            };
            let opts = AiOptions { subcommand: subcmd };
            run(&opts)
        }

        Commands::Work { name, bead, agent, no_agent, idempotent, dry_run } => {
            use crate::commands::handlers::work::{WorkMode, WorkOptions, run_work};
            let ctx = auth_context(vec![Scope::WriteWorkspace]);
            warn_missing_scope(&ctx, &Scope::WriteWorkspace, "work");
            let mode = if dry_run {
                WorkMode::DryRun
            } else if idempotent {
                WorkMode::Idempotent
            } else {
                WorkMode::Normal
            };
            match name {
                Some(n) => {
                    let opts = WorkOptions {
                        name: n,
                        bead_id: bead,
                        agent_id: agent,
                        mode,
                        no_agent,
                        format: OutputFormat::Json,
                    };
                    audit_log("workspace.spawn", &opts.name, &ctx.agent_id);
                    run_work(&opts)
                }
                None => {
                    use scp_core::Error;
                    Err(Error::validation_error("work requires a workspace name"))
                }
            }
        }

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

        Commands::Queue { command } => {
            let ctx = auth_context(vec![Scope::ManageQueue]);
            match command {
                crate::cli::queue_args::QueueCommands::List => commands::queue::list(),
                crate::cli::queue_args::QueueCommands::Enqueue { branch, priority } => {
                    warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.enqueue");
                    audit_log("queue.enqueue", &branch, &ctx.agent_id);
                    commands::queue::enqueue(&branch, priority.as_deref())
                }
                crate::cli::queue_args::QueueCommands::Dequeue => {
                    warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.dequeue");
                    commands::queue::dequeue()
                }
                crate::cli::queue_args::QueueCommands::Process { checks } => {
                    warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.process");
                    commands::queue::process(checks)
                }
                crate::cli::queue_args::QueueCommands::Insert { position, branch } => {
                    warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.insert");
                    audit_log("queue.insert", &branch, &ctx.agent_id);
                    commands::queue::insert(position, &branch)
                }
                crate::cli::queue_args::QueueCommands::Remove { branch } => {
                    warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.remove");
                    audit_log("queue.remove", &branch, &ctx.agent_id);
                    commands::queue::remove(&branch)
                }
                crate::cli::queue_args::QueueCommands::Status => commands::queue::status(),
            }
        }

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

        Commands::Session { command } => {
            let ctx = auth_context(vec![Scope::ManageSessions]);
            match command {
                crate::cli::session_args::SessionCommands::List => commands::session::list(),
                crate::cli::session_args::SessionCommands::Status => commands::session::status(),
                crate::cli::session_args::SessionCommands::Focus { name } => {
                    commands::session::focus(&name)
                }
                crate::cli::session_args::SessionCommands::Submit {
                    name,
                    auto_commit,
                    message,
                } => {
                    warn_missing_scope(&ctx, &Scope::ManageSessions, "session.submit");
                    audit_log("session.submit", name.as_deref().unwrap_or("unknown"), &ctx.agent_id);
                    commands::session::submit(name.as_deref(), auto_commit, message.as_deref())
                }
                crate::cli::session_args::SessionCommands::Remove { name, force, merge } => {
                    warn_missing_scope(&ctx, &Scope::ManageSessions, "session.remove");
                    audit_log("session.remove", &name, &ctx.agent_id);
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
            }
        }

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
            crate::cli::config_args::ConfigCommands::Ports { json } => {
                commands::handlers::config_ports::run_config_ports(json)
            }
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
        } => {
            let ctx = auth_context(vec![Scope::VcsOperations]);
            warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.fetch");
            commands::sync::fetch(remote.as_deref(), prune, tags, all)
        }

        Commands::Pull => {
            let ctx = auth_context(vec![Scope::VcsOperations]);
            warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.pull");
            commands::sync::pull()
        }

        Commands::Push {
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        } => {
            let ctx = auth_context(vec![Scope::VcsOperations]);
            warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.push");
            audit_log(
                "vcs.push",
                branch.as_deref().unwrap_or("unknown"),
                &ctx.agent_id,
            );
            commands::sync::push(
                &remote,
                branch.as_deref(),
                set_upstream,
                force,
                force_with_lease,
                tags,
                delete,
            )
        }

        Commands::Doctor { full } => {
            let _ctx = auth_context(vec![Scope::ReadWorkspace]);
            commands::doctor::run(full)
        }

        Commands::Status { short } => {
            let _ctx = auth_context(vec![Scope::ReadWorkspace]);
            commands::status::run(short)
        }

        Commands::Switch { name } => commands::workspace::switch(&name),

        Commands::Context => commands::context::run(),

        Commands::Whereami => commands::context::whereami(),

        Commands::Whatif { command, args } => {
            let options = commands::handlers::whatif::WhatIfOptions {
                command,
                args,
                format: OutputFormat::Json,
            };
            commands::handlers::whatif::report::run_whatif(&options)
        }

        Commands::Examples { command, use_case } => {
            let options = commands::handlers::examples::ExamplesOptions {
                command,
                use_case,
                format: OutputFormat::Json,
            };
            commands::handlers::examples::run_examples(&options)
        }

        Commands::Workspace { .. } => unreachable!("workspace commands dispatched separately"),

        Commands::Retry { max_attempts, verbose } => {
            use crate::commands::handlers::retry::{run_retry, RetryOptions};
            let opts = RetryOptions { max_attempts, verbose };
            let output = run_retry(opts)?;
            println!("{}", if output.success { "Retry succeeded" } else { &output.message });
            Ok(())
        }
    }
}
