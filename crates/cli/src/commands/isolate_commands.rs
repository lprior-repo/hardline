//! CLI command definitions using `clap`
//! Split module: contains after_help_text helper and build_cli

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::{json_docs, object_commands};

pub fn after_help_text(examples: &[&str], json_output: Option<&'static str>) -> String {
    let mut text = String::from("EXAMPLES:\n");
    for example in examples {
        text.push_str("  ");
        text.push_str(example);
        text.push('\n');
    }
    if let Some(json) = json_output {
        text.push('\n');
        text.push_str(json);
        if !json.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}

pub fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize hardline in a Git repository (or create one)")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview initialization without executing"),
        )
        .after_help(after_help_text(
            &[
                "hardline init                        Initialize Hardline in the current Git repository",
                "hardline init --json                 Output JSON metadata for automation",
                "hardline init --dry-run              Preview initialization",
            ],
            Some(json_docs::init()),
        ))
}

pub fn cmd_switch() -> ClapCommand {
    ClapCommand::new("switch")
        .about("Switch to a different workspace")
        .long_about(
            "Navigate between workspaces.

            Use this for quick workspace switching. Similar to 'hardline focus' but
            emphasizes navigation between existing sessions.",
        )
        .after_help(after_help_text(
            &[
                "hardline switch feature-auth           Switch to named session",
                "hardline switch                        Interactive session selection",
                "hardline switch test --show-context    Switch and show session details",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Name of the session to switch to (interactive if omitted)"),
        )
        .arg(
            Arg::new("show-context")
                .long("show-context")
                .action(clap::ArgAction::SetTrue)
                .help("Show session details after switching"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .after_help(after_help_text(
            &[
                "hardline config                      Show current project config",
                "hardline config workspace_dir        Display the workspace_dir setting",
                "hardline config workspace_dir /new/path --json  Update key and emit JSON",
            ],
            Some(json_docs::config()),
        ))
        .arg(Arg::new("key").help("Config key to view/set (dot notation)"))
        .arg(Arg::new("value").help("Value to set (omit to view)"))
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help("Operate on global config instead of project"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_help() -> ClapCommand {
    ClapCommand::new("help")
        .about("Print help for a command")
        .arg(
            Arg::new("command")
                .required(false)
                .num_args(0..)
                .action(clap::ArgAction::Append)
                .allow_hyphen_values(true)
                .help("Command path to show help for (omit for top-level help)"),
        )
}

pub fn build_cli() -> ClapCommand {
    object_commands::build_object_cli()
}
