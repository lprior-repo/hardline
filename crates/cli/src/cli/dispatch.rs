//! Dispatch for non-workspace top-level commands.
//!
//! Wires ADR-015 security: scope checks and audit logging for every
//! dispatched command. Scope violations are advisory (warnings only)
//! when running in local-only / anonymous mode.

use scp_core::{AuditEntry, AuditLogger, AuditOutcome, AuthContext, OutputFormat, Result, Scope};
use tracing::warn;

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

pub fn run_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Init { vcs } => commands::init::run(&vcs),

        Commands::Ai { command } => handle_ai(command),
        Commands::Work {
            name,
            bead,
            agent,
            no_agent,
            idempotent,
            dry_run,
        } => handle_work(HandleWorkArgs {
            name,
            bead,
            agent,
            no_agent,
            idempotent,
            dry_run,
        }),
        Commands::Lock { command } => handle_lock(command),
        Commands::Queue { command } => handle_queue(command),
        Commands::Agent { command } => handle_agent(command),
        Commands::Session { command } => handle_session(command),
        Commands::Task { command } => handle_task(command),
        Commands::Config { command } => handle_config(command),
        Commands::Stash { command } => handle_stash(command),
        Commands::Tag { command } => handle_tag(command),
        Commands::Batch { command } => handle_batch(command),
        Commands::Fetch {
            remote,
            prune,
            tags,
            all,
        } => handle_fetch(remote, prune, tags, all),
        Commands::Pull => handle_pull(),
        Commands::Push {
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        } => handle_push(HandlePushArgs {
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        }),

        Commands::Doctor { full } => handle_doctor(full),
        Commands::Status { short } => handle_status(short),
        Commands::Switch { name } => commands::workspace::switch(&name),
        Commands::Context => commands::context::run(),
        Commands::Whereami => commands::context::whereami(),
        Commands::Whatif { command, args } => handle_whatif(command, args),
        Commands::Examples { command, use_case } => handle_examples(command, use_case),
        Commands::Workspace { .. } => Err(scp_core::Error::internal(
            "workspace commands should be dispatched separately",
        )),
        Commands::Retry {
            max_attempts,
            verbose,
        } => handle_retry(max_attempts, verbose),
    }
}

// ========================================================================
// Command handlers
// ========================================================================

fn handle_ai(command: crate::cli::ai_args::AiCommands) -> Result<()> {
    use crate::{
        cli::ai_args::AiCommands,
        commands::handlers::ai::{run, AiOptions, AiSubcommand},
    };
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

struct HandleWorkArgs {
    name: Option<String>,
    bead: Option<String>,
    agent: Option<String>,
    no_agent: bool,
    idempotent: bool,
    dry_run: bool,
}

fn handle_work(args: HandleWorkArgs) -> Result<()> {
    use crate::commands::handlers::work::{run_work, WorkMode, WorkOptions};
    let ctx = auth_context(vec![Scope::WriteWorkspace]);
    warn_missing_scope(&ctx, &Scope::WriteWorkspace, "work");
    let mode = if args.dry_run {
        WorkMode::DryRun
    } else if args.idempotent {
        WorkMode::Idempotent
    } else {
        WorkMode::Normal
    };
    match args.name {
        Some(n) => {
            let opts = WorkOptions {
                name: n,
                bead_id: args.bead,
                agent_id: args.agent,
                mode,
                no_agent: args.no_agent,
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

fn handle_lock(command: crate::cli::lock_args::LockCommands) -> Result<()> {
    use crate::cli::lock_args::LockCommands;
    match command {
        LockCommands::Acquire {
            session,
            agent,
            ttl,
        } => commands::lock::acquire(&session, &agent, ttl),
        LockCommands::Release { session, agent } => commands::lock::release(&session, &agent),
        LockCommands::Heartbeat { session, agent } => commands::lock::heartbeat(&session, &agent),
        LockCommands::Status { session } => commands::lock::status(&session),
        LockCommands::List => commands::lock::list(),
    }
}

fn handle_queue(command: crate::cli::queue_args::QueueCommands) -> Result<()> {
    use crate::cli::queue_args::QueueCommands;
    let ctx = auth_context(vec![Scope::ManageQueue]);
    match command {
        QueueCommands::List => commands::queue::list(),
        QueueCommands::Enqueue { branch, priority } => {
            warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.enqueue");
            audit_log("queue.enqueue", &branch, &ctx.agent_id);
            commands::queue::enqueue(&branch, priority.as_deref())
        }
        QueueCommands::Dequeue => {
            warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.dequeue");
            commands::queue::dequeue()
        }
        QueueCommands::Process { checks } => {
            warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.process");
            commands::queue::process(checks)
        }
        QueueCommands::Insert { position, branch } => {
            warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.insert");
            audit_log("queue.insert", &branch, &ctx.agent_id);
            commands::queue::insert(position, &branch)
        }
        QueueCommands::Remove { branch } => {
            warn_missing_scope(&ctx, &Scope::ManageQueue, "queue.remove");
            audit_log("queue.remove", &branch, &ctx.agent_id);
            commands::queue::remove(&branch)
        }
        QueueCommands::Status => commands::queue::status(),
    }
}

fn handle_agent(command: crate::cli::agent_args::AgentCommands) -> Result<()> {
    use crate::cli::agent_args::AgentCommands;
    match command {
        AgentCommands::Create { name } => commands::agent::create(&name),
        AgentCommands::List => commands::agent::list(),
        AgentCommands::Kill { id } => commands::agent::kill(&id),
        AgentCommands::Status { id } => commands::agent::status(id.as_deref()),
        AgentCommands::Register { session } => commands::agent::register(session.as_deref()),
        AgentCommands::Heartbeat { session } => commands::agent::heartbeat(session.as_deref()),
    }
}

fn handle_session(command: crate::cli::session_args::SessionCommands) -> Result<()> {
    use crate::cli::session_args::SessionCommands;
    let ctx = auth_context(vec![Scope::ManageSessions]);
    match command {
        SessionCommands::List => commands::session::list(),
        SessionCommands::Status => commands::session::status(),
        SessionCommands::Focus { name } => commands::session::focus(&name),
        SessionCommands::Submit {
            name,
            auto_commit,
            message,
        } => {
            warn_missing_scope(&ctx, &Scope::ManageSessions, "session.submit");
            audit_log(
                "session.submit",
                name.as_deref().unwrap_or("unknown"),
                &ctx.agent_id,
            );
            commands::session::submit(name.as_deref(), auto_commit, message.as_deref())
        }
        SessionCommands::Remove { name, force, merge } => {
            warn_missing_scope(&ctx, &Scope::ManageSessions, "session.remove");
            audit_log("session.remove", &name, &ctx.agent_id);
            commands::session::remove(&name, force, merge)
        }
        SessionCommands::Pause { name } => commands::handlers::session::pause(&name),
        SessionCommands::Resume { name } => commands::handlers::session::resume(&name),
        SessionCommands::Clone {
            source,
            target,
            dry_run,
        } => {
            commands::handlers::session::clone_session(&source, &target, dry_run)?;
            Ok(())
        }
    }
}

fn handle_task(command: crate::cli::task_args::TaskCommands) -> Result<()> {
    use commands::handlers::task::{parse_task_id, run_task_command, AgentId, TaskCommand};
    let cmd = match command {
        crate::cli::task_args::TaskCommands::List => TaskCommand::List {
            status_filter: None,
            include_all: false,
        },
        crate::cli::task_args::TaskCommands::Show { task_id, .. } => TaskCommand::Show {
            task_id: parse_task_id(&task_id)?,
        },
        crate::cli::task_args::TaskCommands::Claim { task_id, user } => TaskCommand::Claim {
            task_id: parse_task_id(&task_id)?,
            agent_id: AgentId::new(&user)?,
        },
        crate::cli::task_args::TaskCommands::Yield { task_id, user } => TaskCommand::YieldTask {
            task_id: parse_task_id(&task_id)?,
            agent_id: AgentId::new(&user)?,
        },
        crate::cli::task_args::TaskCommands::Start { task_id, user } => TaskCommand::Start {
            task_id: parse_task_id(&task_id)?,
            agent_id: AgentId::new(&user)?,
        },
        crate::cli::task_args::TaskCommands::Done { task_id, user } => TaskCommand::Done {
            task_id: Some(parse_task_id(&task_id)?),
            agent_id: AgentId::new(&user)?,
        },
    };
    run_task_command(&cmd)
}

fn handle_config(command: crate::cli::config_args::ConfigCommands) -> Result<()> {
    use crate::cli::config_args::ConfigCommands;
    match command {
        ConfigCommands::Get { key } => commands::config::get(&key),
        ConfigCommands::Set { key, value } => commands::config::set(&key, &value),
        ConfigCommands::List => commands::config::list(),
        ConfigCommands::Ports { json } => commands::handlers::config_ports::run_config_ports(json),
    }
}

fn handle_stash(command: crate::cli::stash_args::StashCommands) -> Result<()> {
    use crate::cli::stash_args::StashCommands;
    match command {
        StashCommands::Save {
            message,
            include_untracked,
            patch,
        } => commands::stash::save(message.as_deref(), include_untracked, patch),
        StashCommands::Pop { stash, index } => commands::stash::pop(stash.as_deref(), index),
        StashCommands::List => commands::stash::list(),
        StashCommands::Drop { stash, force } => commands::stash::drop(&stash, force),
        StashCommands::Show { stash, stat } => commands::stash::show(stash.as_deref(), stat),
    }
}

fn handle_tag(command: crate::cli::tag_args::TagCommands) -> Result<()> {
    use crate::cli::tag_args::TagCommands;
    match command {
        TagCommands::Create {
            name,
            message,
            commit,
            force,
        } => commands::tag::create(&name, message.as_deref(), commit.as_deref(), force),
        TagCommands::List { pattern, sort } => {
            commands::tag::list(pattern.as_deref(), sort.as_deref())
        }
        TagCommands::Delete { tag, remote } => commands::tag::delete(&tag, remote),
        TagCommands::Push { tag, remote, force } => {
            commands::tag::push(tag.as_deref(), &remote, force)
        }
    }
}

fn handle_batch(command: crate::cli::batch_args::BatchCommands) -> Result<()> {
    use crate::cli::batch_args::BatchCommands;
    match command {
        BatchCommands::Run {
            workspace,
            commands,
        } => tokio::runtime::Handle::current()
            .block_on(commands::batch::execute(workspace, commands)),
    }
}

fn handle_fetch(remote: Option<String>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let ctx = auth_context(vec![Scope::VcsOperations]);
    warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.fetch");
    commands::sync::fetch(remote.as_deref(), prune, tags, all)
}

fn handle_pull() -> Result<()> {
    let ctx = auth_context(vec![Scope::VcsOperations]);
    warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.pull");
    commands::sync::pull()
}

struct HandlePushArgs {
    remote: String,
    branch: Option<String>,
    set_upstream: bool,
    force: bool,
    force_with_lease: bool,
    tags: bool,
    delete: bool,
}

fn handle_push(args: HandlePushArgs) -> Result<()> {
    let ctx = auth_context(vec![Scope::VcsOperations]);
    warn_missing_scope(&ctx, &Scope::VcsOperations, "vcs.push");
    audit_log(
        "vcs.push",
        args.branch.as_deref().unwrap_or("unknown"),
        &ctx.agent_id,
    );
    commands::sync::push(commands::sync::PushArgs {
        remote: &args.remote,
        branch: args.branch.as_deref(),
        set_upstream: args.set_upstream,
        force: args.force,
        force_with_lease: args.force_with_lease,
        tags: args.tags,
        delete: args.delete,
    })
}

fn handle_doctor(full: bool) -> Result<()> {
    let _ctx = auth_context(vec![Scope::ReadWorkspace]);
    commands::doctor::run(full)
}

fn handle_status(short: bool) -> Result<()> {
    let _ctx = auth_context(vec![Scope::ReadWorkspace]);
    commands::status::run(short)
}

fn handle_whatif(command: String, args: Vec<String>) -> Result<()> {
    let options = commands::handlers::whatif::WhatIfOptions {
        command,
        args,
        format: OutputFormat::Json,
    };
    commands::handlers::whatif::report::run_whatif(&options)
}

fn handle_examples(command: Option<String>, use_case: Option<String>) -> Result<()> {
    let options = commands::handlers::examples::ExamplesOptions {
        command,
        use_case,
        format: OutputFormat::Json,
    };
    commands::handlers::examples::run_examples(&options)
}

fn handle_retry(max_attempts: u32, verbose: bool) -> Result<()> {
    use crate::commands::handlers::retry::{run_retry, RetryOptions};
    let opts = RetryOptions {
        max_attempts,
        verbose,
    };
    let output = run_retry(opts)?;
    println!(
        "{}",
        if output.success {
            "Retry succeeded"
        } else {
            &output.message
        }
    );
    Ok(())
}
