use super::helpers::{json_arg, verbose_arg};
use clap::{Arg, Command as ClapCommand};

/// Build the Session object command with all subcommands
#[allow(clippy::too_many_lines)]
pub fn cmd_session() -> ClapCommand {
    ClapCommand::new("session")
        .about("Manage workspaces and sessions")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include closed sessions"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed information"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .value_name("BEAD_ID")
                        .help("Filter by bead ID"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Filter by agent owner"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by session state"),
                ),
        )
        .subcommand(
            ClapCommand::new("add")
                .visible_alias("create")
                .about("Create a new session for manual work")
                .arg(json_arg())
                .arg(super::helpers::dry_run_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .help("Succeed if session already exists (no-op)"),
                )
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new session"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .short('b')
                        .value_name("BEAD_ID")
                        .help("Associate with a bead ID"),
                )
                .arg(
                    Arg::new("no-open")
                        .long("no-open")
                        .action(clap::ArgAction::SetTrue)
                        .help("Create without opening terminal"),
                )
                .arg(
                    Arg::new("no-hooks")
                        .long("no-hooks")
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip post-create hooks"),
                ),
        )
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove a session")
                .arg(json_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .help("Succeed if session doesn't exist (no-op)"),
                )
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to remove"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force removal without confirmation"),
                ),
        )
        .subcommand(
            ClapCommand::new("pause")
                .about("Pause a session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("resume")
                .about("Resume a paused session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("clone")
                .about("Clone a session")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to clone"),
                )
                .arg(
                    Arg::new("new-name")
                        .long("new-name")
                        .value_name("NAME")
                        .help("Name for cloned session"),
                )
                .arg(super::helpers::dry_run_arg()),
        )
        .subcommand(
            ClapCommand::new("rename")
                .about("Rename a session")
                .arg(json_arg())
                .arg(
                    Arg::new("old-name")
                        .required(true)
                        .help("Current session name"),
                )
                .arg(Arg::new("new-name").required(true).help("New session name")),
        )
        .subcommand(
            ClapCommand::new("spawn")
                .about("Spawn session for automated agent work")
                .arg(json_arg())
                .arg(super::helpers::dry_run_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .default_value("false")
                        .help("Succeed if session already exists (no-op)"),
                )
                .arg(
                    Arg::new("bead")
                        .required(true)
                        .help("Bead ID for the spawned session"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Agent to assign"),
                ),
        )
        .subcommand(
            ClapCommand::new("sync")
                .visible_alias("rebase")
                .about("Sync session with remote")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)"))
                .arg(
                    Arg::new("push")
                        .long("push")
                        .action(clap::ArgAction::SetTrue)
                        .help("Push changes to remote"),
                )
                .arg(
                    Arg::new("pull")
                        .long("pull")
                        .action(clap::ArgAction::SetTrue)
                        .help("Pull changes from remote"),
                ),
        )
        .subcommand(
            ClapCommand::new("init")
                .about("Initialize isolate in a JJ repository")
                .arg(json_arg())
                .arg(super::helpers::dry_run_arg()),
        )
}
