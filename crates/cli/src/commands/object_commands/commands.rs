use super::config::cmd_config;
use super::doctor::cmd_doctor;
use super::helpers::{json_arg, verbose_arg};
use super::legacy_commands::build_legacy_commands;
use super::{cmd_session, cmd_status, cmd_task};
use clap::Command as ClapCommand;

/// Build the complete object-based CLI
///
/// This creates the new `isolate <object> <action>` command structure
/// while maintaining compatibility with existing handlers.
#[allow(clippy::too_many_lines)]
pub fn build_object_cli() -> ClapCommand {
    ClapCommand::new("isolate")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Isolate Contributors")
        .about("Isolate - Isolated workspace manager (object-based CLI)")
        .long_about(
            "Isolate creates isolated JJ workspaces.\n\n\
             Object-based command structure:\n\
             \n\
   isolate task <action>     Manage tasks and work items\n\
             \n\
   isolate session <action>  Manage workspaces and sessions\n\
             \n\
   isolate status <action>   Query system status\n\
             \n\
   isolate config <action>   Manage configuration\n\
             \n\
   isolate doctor <action>   Run diagnostics\n",
        )
        .subcommand_required(true)
        .arg(json_arg().global(true))
        .arg(verbose_arg().global(true))
        .arg(
            clap::Arg::new("on-success")
                .long("on-success")
                .global(true)
                .value_name("CMD")
                .help("Command to run after successful execution"),
        )
        .arg(
            clap::Arg::new("on-failure")
                .long("on-failure")
                .global(true)
                .value_name("CMD")
                .help("Command to run after failed execution"),
        )
        .arg(
            clap::Arg::new("command-id")
                .long("command-id")
                .global(true)
                .hide(true)
                .value_name("ID")
                .help("Override idempotency command id base for retries"),
        )
        .subcommand(cmd_task())
        .subcommand(cmd_session())
        .subcommand(cmd_status())
        .subcommand(cmd_config())
        .subcommand(cmd_doctor())
        // Legacy commands - route to same handlers
        .subcommand(build_legacy_commands())
}
