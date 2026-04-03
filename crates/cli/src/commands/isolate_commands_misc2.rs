//! Miscellaneous commands part 2: recover, retry, rollback, abort, backup

use clap::{Arg, Command as ClapCommand};

/// Recover from inconsistent state or restore from operation log
pub fn cmd_recover() -> ClapCommand {
    ClapCommand::new("recover")
        .about("Recover from inconsistent state or restore from operation log")
        .arg(
            Arg::new("session")
                .value_name("SESSION")
                .help("Session name to recover (optional, uses current workspace if not specified)")
                .num_args(0..=1)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("diagnose")
                .short('d')
                .long("diagnose")
                .action(clap::ArgAction::SetTrue)
                .help("Only diagnose system issues without fixing (system recovery mode)"),
        )
        .arg(
            Arg::new("op")
                .long("op")
                .value_name("ID")
                .help("Restore to specific operation ID (operation log mode)")
                .num_args(1)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("last")
                .long("last")
                .action(clap::ArgAction::SetTrue)
                .help("Restore to previous operation (quick undo)"),
        )
        .arg(
            Arg::new("list-ops")
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List operation log without restoring (default when no --op or --last)"),
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
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline recover                      Auto-diagnose and fix issues",
                "hardline recover --diagnose           Only diagnose, don't fix",
                "hardline recover feature-x            Recover specific session",
            ],
            None,
        ))
}

/// Retry the last failed operation
pub fn cmd_retry() -> ClapCommand {
    ClapCommand::new("retry")
        .about("Retry the last failed operation")
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
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &["hardline retry                       Retry last failed operation"],
            None,
        ))
}

/// Rollback session to a specific checkpoint
pub fn cmd_rollback() -> ClapCommand {
    ClapCommand::new("rollback")
        .about("Rollback session to a specific checkpoint")
        .arg(Arg::new("session").required(true).help("Session name"))
        .arg(
            Arg::new("to")
                .long("to")
                .required(true)
                .help("Checkpoint ID to rollback to"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview rollback without executing"),
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
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline rollback feature-x --to 123  Rollback to checkpoint",
                "hardline rollback --dry-run feature-x --to 123  Preview rollback",
            ],
            None,
        ))
}

/// Abort work and abandon workspace changes
pub fn cmd_abort() -> ClapCommand {
    ClapCommand::new("abort")
        .about("Abort work and abandon workspace changes")
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .visible_alias("session")
                .value_name("NAME")
                .help("Workspace/session to abort (uses current if not specified)"),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue)
                .help("Don't update bead status"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue)
                .help("Keep workspace files (just remove from hardline tracking)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show execution hints and common patterns"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline abort                       Abandon current workspace",
                "hardline abort --session feature-x   Abort specific workspace",
                "hardline abort --keep-workspace      Keep files, just remove from hardline",
                "hardline abort --dry-run             Preview abort without executing",
            ],
            None,
        ))
}

/// Backup command - manage database backups
pub fn cmd_backup() -> ClapCommand {
    ClapCommand::new("backup")
        .about("Manage automated database backups")
        .long_about(
            "Create, list, restore, and manage backups of hardline databases (state.db, beads.db).\n\n\
            Backups include:\n\
            - state.db: Session, workspace state, and merge queue\n\
            - beads.db: Issue tracking database\n\n\
            Note: queue.db has been consolidated into state.db.\n\n\
            Backups are stored with timestamps and SHA-256 checksums for integrity verification.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline backup --create                     Create backups of all databases",
                "hardline backup --list                       List all available backups",
                "hardline backup --restore state.db           Restore latest backup of state.db",
                "hardline backup --restore beads.db --timestamp 20250101-010101  Restore specific backup by timestamp",
                "hardline backup --status                     Show backup status and retention info",
                "hardline backup --retention                  Apply retention policy (remove old backups)",
                "hardline backup --create --json              Create backups with JSON output",
            ],
            None,
        ))
        .arg(
            Arg::new("create")
                .long("create")
                .action(clap::ArgAction::SetTrue)
                .help("Create new backups of all databases"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List all available backups"),
        )
        .arg(
            Arg::new("restore")
                .long("restore")
                .value_name("DATABASE")
                .help("Restore database from backup (state.db, beads.db)"),
        )
        .arg(
            Arg::new("timestamp")
                .short('t')
                .long("timestamp")
                .value_name("TIMESTAMP")
                .requires("restore")
                .help("Specific backup timestamp to restore (format: YYYYMMDD-HHMMSS)"),
        )
        .arg(
            Arg::new("status")
                .long("status")
                .action(clap::ArgAction::SetTrue)
                .help("Show backup status and retention policy information"),
        )
        .arg(
            Arg::new("retention")
                .long("retention")
                .action(clap::ArgAction::SetTrue)
                .help("Apply retention policy and remove old backups"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}
