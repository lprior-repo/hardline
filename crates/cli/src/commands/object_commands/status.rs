use super::helpers::{ai_hints_arg, contract_arg, json_arg};
use clap::{Arg, Command as ClapCommand};

/// Build the Status object command with all subcommands
pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Query system and session status")
        .subcommand_required(false)
        .arg(json_arg())
        .arg(contract_arg())
        .arg(ai_hints_arg())
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show current status")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("whereami")
                .about("Show current location")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("whoami")
                .about("Show current identity")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("context")
                .about("Show context information")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("field").help("Specific field to display"))
                .arg(
                    Arg::new("no-beads")
                        .long("no-beads")
                        .action(clap::ArgAction::SetTrue)
                        .help("Don't show beads in context"),
                )
                .arg(
                    Arg::new("no-health")
                        .long("no-health")
                        .action(clap::ArgAction::SetTrue)
                        .help("Don't show health checks in context"),
                )
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
}
