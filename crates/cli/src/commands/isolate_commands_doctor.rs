//! Doctor command: diagnostics and health checks

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Run diagnostics and health checks
pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run diagnostics and health checks")
        .alias("check")
        .subcommand_required(false)
        .subcommand(
            ClapCommand::new("check")
                .about("Run diagnostics")
                .alias("check")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("fix")
                .about("Fix detected issues")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview what would be fixed without making changes"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed progress during fixes"),
                ),
        )
        .subcommand(
            ClapCommand::new("integrity")
                .about("Check system integrity")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("clean")
                .about("Clean up invalid sessions")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                )
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
                ),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline doctor                    Run all system health checks (legacy)",
                "hardline doctor check             Run all system health checks",
                "hardline doctor fix              Auto-fix issues where possible",
                "hardline doctor fix --dry-run    Preview what would be fixed without making changes",
                "hardline doctor fix --verbose    Show detailed progress during fixes",
                "hardline doctor integrity        Run database integrity check",
                "hardline doctor clean            Remove stale sessions",
                "hardline doctor --json           Export check results to JSON (legacy)",
            ],
            Some(json_docs::doctor()),
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (legacy mode)"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible (legacy mode)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .requires("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Preview what would be fixed without making changes"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .requires("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Show detailed progress during fixes"),
        )
}
