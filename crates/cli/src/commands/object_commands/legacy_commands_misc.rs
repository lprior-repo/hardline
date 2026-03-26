use clap::{Arg, Command as ClapCommand};

/// Build spawn command
pub fn build_spawn_command() -> ClapCommand {
    ClapCommand::new("spawn")
        .about("Spawn session")
        .arg(Arg::new("bead").required(true))
        .arg(super::helpers::dry_run_arg())
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(Arg::new("agent").long("agent").value_name("AGENT"))
        .arg(
            Arg::new("no-auto-merge")
                .long("no-auto-merge")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("no-auto-cleanup")
                .long("no-auto-cleanup")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("background")
                .short('b')
                .long("background")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .default_value("14400"),
        )
        .arg(
            Arg::new("agent-command")
                .long("agent-command")
                .value_name("COMMAND")
                .default_value("claude"),
        )
        .arg(Arg::new("agent-args").long("agent-args").num_args(0..))
}

/// Build sync command
pub fn build_sync_command() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync session")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build clone command
pub fn build_clone_command() -> ClapCommand {
    ClapCommand::new("clone")
        .about("Clone session")
        .arg(Arg::new("source").required(true))
        .arg(Arg::new("dest").required(true))
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build rename command
pub fn build_rename_command() -> ClapCommand {
    ClapCommand::new("rename")
        .about("Rename session")
        .arg(Arg::new("old_name").required(true))
        .arg(Arg::new("new_name").required(true))
        .arg(super::helpers::json_arg())
}

/// Build pause command
pub fn build_pause_command() -> ClapCommand {
    ClapCommand::new("pause")
        .about("Pause session")
        .arg(Arg::new("name").required(false))
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build resume command
pub fn build_resume_command() -> ClapCommand {
    ClapCommand::new("resume")
        .about("Resume session")
        .arg(Arg::new("name").required(false))
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}
