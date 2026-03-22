//! Admin/system commands: clean, prune_invalid, integrity

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Remove stale sessions (where workspace no longer exists)
pub fn cmd_clean() -> ClapCommand {
    ClapCommand::new("clean")
        .about("Remove stale sessions (where workspace no longer exists)")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate clean                       Remove stale sessions",
                "isolate clean --dry-run             List stale sessions without deleting",
                "isolate clean --force --json        Force clean and emit JSON summary",
            ],
            Some(json_docs::clean()),
        ))
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List stale sessions without removing"),
        )
        .arg(
            Arg::new("periodic")
                .long("periodic")
                .action(clap::ArgAction::SetTrue)
                .help("Run as periodic cleanup daemon (1hr interval)"),
        )
        .arg(
            Arg::new("age-threshold")
                .long("age-threshold")
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(u64))
                .help("Age threshold for periodic cleanup (default: 7200 = 2hr)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

/// Remove all invalid session records in one deterministic command
pub fn cmd_prune_invalid() -> ClapCommand {
    ClapCommand::new("prune-invalid")
        .about("Remove all invalid session records in one deterministic command")
        .long_about(
            "Bulk cleanup primitive to remove all invalid session records.

Invalid sessions are those where the workspace directory no longer exists
but the session record still exists in the database.

This is useful for cleaning up after workspace directory deletions
or when sessions become orphaned.

Use --yes to skip confirmation for scripting/CI use.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate prune-invalid                Remove invalid sessions (with prompt)",
                "isolate prune-invalid --yes         Remove invalid sessions (no prompt)",
                "isolate prune-invalid --dry-run     List invalid sessions without deleting",
                "isolate prune-invalid --yes --json Remove with JSON output",
            ],
            None,
        ))
        .arg(
            Arg::new("yes")
                .long("yes")
                .short('y')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt (for scripting/CI)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List invalid sessions without removing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

/// Manage workspace integrity and corruption recovery
pub fn cmd_integrity() -> ClapCommand {
    ClapCommand::new("integrity")
        .about("Manage workspace integrity and corruption recovery")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("validate")
                .about("Validate workspace integrity")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("repair")
                .about("Repair corrupted workspace")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                )
                .arg(
                    Arg::new("rebind")
                        .long("rebind")
                        .action(clap::ArgAction::SetTrue)
                        .help("Update session record when workspace is detected in a new location"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("backup")
                .about("Manage workspace backups")
                .subcommand_required(true)
                .subcommand(
                    ClapCommand::new("list")
                        .about("List available backups")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                )
                .subcommand(
                    ClapCommand::new("restore")
                        .about("Restore from a backup")
                        .arg(
                            Arg::new("backup_id")
                                .required(true)
                                .help("Backup ID to restore"),
                        )
                        .arg(
                            Arg::new("force")
                                .long("force")
                                .short('f')
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                ),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate integrity validate feature-x    Validate workspace integrity",
                "isolate integrity repair feature-x      Repair corrupted workspace",
                "isolate integrity repair -f feature-x   Repair without confirmation",
                "isolate integrity backup list           List available backups",
                "isolate integrity backup restore 123    Restore from backup ID",
            ],
            None,
        ))
}
