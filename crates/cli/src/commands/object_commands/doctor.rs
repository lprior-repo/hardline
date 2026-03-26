use super::helpers::json_arg;
use clap::{Arg, Command as ClapCommand};

/// Build the Doctor object command with all subcommands
pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run diagnostics and health checks")
        .subcommand_required(false)
        .arg(json_arg())
        // Legacy flags for backward compatibility
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible (legacy mode)"),
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
        )
        .subcommand(
            ClapCommand::new("check")
                .about("Run diagnostics")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("fix")
                .about("Fix detected issues")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
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
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("clean")
                .about("Clean up invalid sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force cleanup without confirmation"),
                ),
        )
}
