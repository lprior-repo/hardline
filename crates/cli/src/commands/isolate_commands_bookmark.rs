//! Bookmark command: manage JJ bookmarks/branches

use clap::{Arg, Command as ClapCommand};

/// Manage JJ bookmarks/branches
pub fn cmd_bookmark() -> ClapCommand {
    ClapCommand::new("bookmark")
        .about("Manage JJ bookmarks/branches")
        .long_about(
            "Manage bookmarks (branches) in JJ workspaces.


            isolate wraps JJ completely - use 'isolate bookmark' not 'jj bookmark'.

            Provides: list, create, delete, move operations.",
        )
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("list")
                .about("List bookmarks in a session workspace")
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show all bookmarks including remote"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("create")
                .about("Create a new bookmark at current revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new bookmark"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("push")
                        .long("push")
                        .short('p')
                        .action(clap::ArgAction::SetTrue)
                        .help("Push bookmark to remote after creation"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("delete")
                .about("Delete a bookmark")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to delete"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("move")
                .about("Move a bookmark to a different revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to move"),
                )
                .arg(
                    Arg::new("to")
                        .long("to")
                        .required(true)
                        .value_name("REVISION")
                        .help("Target revision (commit hash or revset)"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate bookmark list                List bookmarks in current workspace",
                "isolate bookmark list --all          Show all bookmarks including remote",
                "isolate bookmark create feature-x    Create bookmark at current revision",
                "isolate bookmark create -p stable    Create and push to remote",
                "isolate bookmark delete old-fix      Delete a bookmark",
                "isolate bookmark move stable --to @  Move bookmark to current revision",
            ],
            None,
        ))
}
