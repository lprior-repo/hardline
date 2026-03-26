use clap::{Arg, Command as ClapCommand};

/// Build whoami command
pub fn build_whoami_command() -> ClapCommand {
    ClapCommand::new("whoami")
        .about("Who am I")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build whereami command
pub fn build_whereami_command() -> ClapCommand {
    ClapCommand::new("whereami")
        .about("Where am I")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
}

/// Build context command
pub fn build_context_command() -> ClapCommand {
    ClapCommand::new("context")
        .about("Show context")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .arg(
            Arg::new("field")
                .long("field")
                .value_name("PATH")
                .help("Extract single field (e.g., --field=repository.branch)"),
        )
        .arg(
            Arg::new("no-beads")
                .long("no-beads")
                .action(clap::ArgAction::SetTrue)
                .help("Skip beads database query (faster)"),
        )
        .arg(
            Arg::new("no-health")
                .long("no-health")
                .action(clap::ArgAction::SetTrue)
                .help("Skip health checks (faster)"),
        )
}

/// Build done command
pub fn build_done_command() -> ClapCommand {
    ClapCommand::new("done")
        .about("Done (complete work)")
        .visible_alias("submit")
        .arg(super::helpers::json_arg())
        .arg(super::helpers::contract_arg())
        .arg(super::helpers::ai_hints_arg())
        .arg(Arg::new("name").required(false))
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .value_name("NAME"),
        )
        .arg(
            Arg::new("message")
                .short('m')
                .long("message")
                .value_name("MSG"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-keep")
                .long("no-keep")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("squash")
                .long("squash")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("detect-conflicts")
                .long("detect-conflicts")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue),
        )
}
