//! AI/Agent commands: ai, spawn, work, checkpoint

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// AI-first commands for streamlined workflows
pub fn cmd_ai() -> ClapCommand {
    ClapCommand::new("ai")
        .about("AI-first commands for streamlined workflows")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("work")
                .about("Start work on a task in an isolated environment")
                .arg(
                    Arg::new("task_id")
                        .required(false)
                        .help("The identifier for the task"),
                ),
        )
}

/// Create session for automated agent work on a bead (issue)
pub fn cmd_spawn() -> ClapCommand {
    ClapCommand::new("spawn")
        .about("Create session for automated agent work on a bead (issue)")
        .long_about(
            "Creates a JJ workspace, runs an agent (default: claude), and auto-merges on success.

            Use this when an AI AGENT should work autonomously on a bead.


            For manual interactive work, use 'isolate add' instead.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate spawn isolate-abc12               Spawn workspace for bead with Claude",
                "isolate spawn isolate-xyz34 -b            Run agent in background",
                "isolate spawn isolate-def56 --agent-command=llm-run  Use custom agent",
                "isolate spawn isolate-ghi78 --no-auto-merge  Don't auto-merge on success",
            ],
            Some(json_docs::spawn()),
        ))
        .arg(
            Arg::new("bead")
                .required(true)
                .help("Bead ID to work on (e.g., isolate-xxxx)"),
        )
        .arg(
            Arg::new("agent-command")
                .long("agent-command")
                .value_name("COMMAND")
                .default_value("claude")
                .help("Agent command to run"),
        )
        .arg(
            Arg::new("agent-args")
                .long("agent-args")
                .value_name("ARGS")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Additional agent arguments"),
        )
        .arg(
            Arg::new("no-auto-merge")
                .long("no-auto-merge")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Don't merge on success"),
        )
        .arg(
            Arg::new("no-auto-cleanup")
                .long("no-auto-cleanup")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Don't cleanup on failure"),
        )
        .arg(
            Arg::new("background")
                .long("background")
                .short('b')
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Run agent in background"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .default_value("14400")
                .help("Timeout in seconds (default: 14400 = 4 hours)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("AI: Show execution hints and common patterns"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Succeed if workspace already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Preview spawn without executing"),
        )
}

/// Start working on a task (create workspace + register agent)
pub fn cmd_work() -> ClapCommand {
    ClapCommand::new("work")
        .about("Start working on a task (create workspace + register agent)")
        .long_about(
            "Unified workflow start command for AI agents.


            Combines multiple steps:
  
            1. Create workspace (or reuse if --idempotent)
  
            2. Register as agent (unless --no-agent)
  
            3. Set environment variables
  
            4. Output workspace info


            This is the AI-friendly entry point for starting work.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate work feature-auth              Start working on feature-auth",
                "isolate work bug-fix --bead isolate-123    Start work on bead",
                "isolate work test --idempotent         Reuse existing session if exists",
                "isolate work --dry-run feature         Preview what would be created",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required_unless_present_any(["contract", "ai-hints"])
                .help("Session name to create/use"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID")
                .help("Bead ID to associate with this work"),
        )
        .arg(
            Arg::new("agent-id")
                .long("agent-id")
                .value_name("ID")
                .help("Agent ID to register (auto-generated if not provided)"),
        )
        .arg(
            Arg::new("no-agent")
                .long("no-agent")
                .action(clap::ArgAction::SetTrue)
                .help("Don't register as agent"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without creating"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show workflow patterns and best practices"),
        )
}

/// Save and restore session state snapshots
pub fn cmd_checkpoint() -> ClapCommand {
    ClapCommand::new("checkpoint")
        .about("Save and restore session state snapshots")
        .alias("ckpt")
        .subcommand_required(true)
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate checkpoint create --description=\"before lunch\"  Snapshot current sessions",
                "isolate checkpoint list                 Show all available checkpoints",
                "isolate checkpoint restore ckpt-123     Restore workspace state from checkpoint",
            ],
            Some(json_docs::checkpoint()),
        ))
        .subcommand(
            ClapCommand::new("create")
                .about("Create a checkpoint of all current sessions")
                .arg(
                    Arg::new("description")
                        .short('d')
                        .long("description")
                        .value_name("DESC")
                        .help("Description for this checkpoint"),
                ),
        )
        .subcommand(
            ClapCommand::new("restore")
                .about("Restore sessions to a checkpoint state")
                .arg(
                    Arg::new("checkpoint_id")
                        .required(true)
                        .help("Checkpoint ID to restore"),
                ),
        )
        .subcommand(ClapCommand::new("list").about("List all available checkpoints"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
}
