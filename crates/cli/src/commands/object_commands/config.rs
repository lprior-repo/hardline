use super::helpers::json_arg;
use clap::{Arg, Command as ClapCommand};

/// Build the Config object command with all subcommands
pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("Manage isolate configuration")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List configuration values")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show global config instead of project"),
                ),
        )
        .subcommand(
            ClapCommand::new("get")
                .about("Get a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Get from global config"),
                )
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to get"),
                ),
        )
        .subcommand(
            ClapCommand::new("set")
                .about("Set a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Set in global config"),
                )
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to set"),
                )
                .arg(Arg::new("value").required(true).help("Value to set")),
        )
        .subcommand(
            ClapCommand::new("schema")
                .about("Show configuration schema")
                .arg(json_arg()),
        )
}
