use clap::Command as ClapCommand;

/// Build a command with common arguments
fn with_common_args(cmd: ClapCommand) -> ClapCommand {
    cmd.arg(super::helpers::dry_run_arg())
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build legacy commands (init, add, list, remove)
pub fn build_legacy_commands() -> ClapCommand {
    use clap::Command as ClapCommand;

    ClapCommand::new("legacy")
        .about("Legacy commands for backward compatibility")
        .subcommand_required(true)
        .arg(super::helpers::dry_run_arg())
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .subcommand(build_init_command())
        .subcommand(build_add_command())
        .subcommand(build_list_command())
        .subcommand(build_remove_command())
}

/// Build init command
pub fn build_init_command() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize isolate")
        .arg(super::helpers::dry_run_arg())
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build add command
pub fn build_add_command() -> ClapCommand {
    ClapCommand::new("add")
        .about("Add session")
        .arg(clap::Arg::new("name").required_unless_present("example-json"))
        .arg(super::helpers::dry_run_arg())
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .arg(
            clap::Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID"),
        )
        .arg(
            clap::Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            clap::Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            clap::Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            clap::Arg::new("example-json")
                .long("example-json")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
}

/// Build list command
pub fn build_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .about("List sessions")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .arg(
            clap::Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(clap::Arg::new("bead").long("bead").value_name("BEAD_ID"))
        .arg(clap::Arg::new("agent").long("agent").value_name("AGENT"))
        .arg(clap::Arg::new("state").long("state").value_name("STATE"))
}

/// Build remove command
pub fn build_remove_command() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove session")
        .arg(clap::Arg::new("name").required(true))
        .arg(
            clap::Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}
