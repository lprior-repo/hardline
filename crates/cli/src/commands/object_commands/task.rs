use clap::{Arg, Command as ClapCommand};

/// Create the JSON argument (common to all commands)
fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Output as JSON (machine-parseable format)")
}

/// Create the verbose argument
fn verbose_arg() -> Arg {
    Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Enable verbose output")
}

/// Create the dry-run argument
fn dry_run_arg() -> Arg {
    Arg::new("dry-run")
        .long("dry-run")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Preview without executing")
}

/// Create the contract argument (AI: Show machine-readable contract)
fn contract_arg() -> Arg {
    Arg::new("contract")
        .long("contract")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)")
}

/// Create the ai-hints argument (AI: Show execution hints)
fn ai_hints_arg() -> Arg {
    Arg::new("ai-hints")
        .long("ai-hints")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show execution hints and common patterns")
}

/// Build the Task object command with all subcommands
pub fn cmd_task() -> ClapCommand {
    ClapCommand::new("task")
        .about("Manage tasks and work items (beads)")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all tasks")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include completed tasks"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by task state"),
                ),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show task details")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to show")),
        )
        .subcommand(
            ClapCommand::new("start")
                .about("Start work on a task (creates session)")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to start")),
        )
        .subcommand(
            ClapCommand::new("done")
                .visible_alias("complete")
                .about("Complete a task")
                .arg(json_arg())
                .arg(Arg::new("id").help("Task/bead ID (uses current session if omitted)")),
        )
}
