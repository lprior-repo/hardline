//! Import/export commands: clone, export, import, rename

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Clone a session into a new one
pub fn cmd_clone() -> ClapCommand {
    ClapCommand::new("clone")
        .about("Clone a session into a new one")
        .arg(
            Arg::new("source")
                .required(true)
                .help("Source session name"),
        )
        .arg(
            Arg::new("dest")
                .required(true)
                .help("Destination session name"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &["hardline clone feature-x feature-y     Clone session"],
            None,
        ))
}

/// Export session state to a file
pub fn cmd_export() -> ClapCommand {
    ClapCommand::new("export")
        .about("Export session state to a file")
        .long_about(
            "Export session state to a file or stdout.

The SESSION argument specifies which session to export. If omitted, all sessions
are exported. To write to a file, you MUST use the -o/--output flag. This
prevents ambiguity between session names and file paths.

IMPORTANT: Output file paths require -o/--output flag:
  - 'hardline export -o export.json'    - Correct: export all sessions to file
  - 'hardline export export.json'       - WRONG: 'export.json' treated as session name!",
        )
        .arg(Arg::new("session").help("Session name to export (all if omitted)"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path (REQUIRED when writing to a file)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline export feature-x -o state.json  Export specific session to file",
                "hardline export -o state.json            Export all sessions to file",
                "hardline export --json                   Export all sessions as JSON to stdout",
                "hardline export                          Export all sessions to stdout",
                "",
                "NOTE: Always use -o when writing to a file:",
                "  CORRECT:   hardline export -o output.json",
                "  INCORRECT: hardline export output.json   (interprets as session name!)",
            ],
            Some(json_docs::export()),
        ))
}

/// Import session state from a file
pub fn cmd_import() -> ClapCommand {
    ClapCommand::new("import")
        .about("Import session state from a file")
        .arg(Arg::new("file").required(true).help("Input file path"))
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("skip-existing")
                .help("Overwrite existing sessions"),
        )
        .arg(
            Arg::new("skip-existing")
                .long("skip-existing")
                .action(clap::ArgAction::SetTrue)
                .help("Skip sessions that already exist"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview import without changes"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline import state.json           Import session from file",
                "hardline import -f state.json        Force overwrite existing",
                "hardline import --dry-run state.json  Preview import",
            ],
            None,
        ))
}

/// Rename an existing session
pub fn cmd_rename() -> ClapCommand {
    ClapCommand::new("rename")
        .about("Rename an existing session")
        .arg(
            Arg::new("old_name")
                .required(true)
                .help("Current session name"),
        )
        .arg(Arg::new("new_name").required(true).help("New session name"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &["hardline rename old-name new-name        Rename a session"],
            None,
        ))
}
