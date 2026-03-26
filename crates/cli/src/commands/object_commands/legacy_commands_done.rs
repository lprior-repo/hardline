use clap::{Arg, Command as ClapCommand};

/// Build work command
pub fn build_work_command() -> ClapCommand {
    ClapCommand::new("work")
        .about("Start work on a task")
        .arg(Arg::new("bead").required(false))
        .arg(Arg::new("name").required(false))
        .arg(Arg::new("agent-id").long("agent-id").value_name("ID"))
        .arg(
            Arg::new("no-agent")
                .long("no-agent")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build abort command
pub fn build_abort_command() -> ClapCommand {
    ClapCommand::new("abort")
        .about("Abort work")
        .arg(Arg::new("name").required(false))
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .value_name("NAME"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build checkpoint command
pub fn build_checkpoint_command() -> ClapCommand {
    ClapCommand::new("checkpoint")
        .about("Create checkpoint")
        .visible_alias("ckpt")
        .arg(Arg::new("name").required(false))
        .arg(super::helpers::json_arg())
}

/// Build undo command
pub fn build_undo_command() -> ClapCommand {
    ClapCommand::new("undo")
        .about("Undo last operation")
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(super::helpers::json_arg())
}

/// Build revert command
pub fn build_revert_command() -> ClapCommand {
    ClapCommand::new("revert")
        .about("Revert changes")
        .arg(Arg::new("name").required(true))
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(super::helpers::json_arg())
}
