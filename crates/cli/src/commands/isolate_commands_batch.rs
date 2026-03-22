//! Batch command: execute multiple commands in a batch

use clap::{Arg, Command as ClapCommand};

/// Execute multiple commands in a batch
pub fn cmd_batch() -> ClapCommand {
    ClapCommand::new("batch")
        .about("Execute multiple commands in a batch")
        .long_about(
            "Runs multiple commands in sequence or from a file.


            Features:
  
            - Atomic transactional mode (all succeed or all roll back)
  
            - Stop-on-error control
  
            - Combined results output",
        )
        .arg(
            Arg::new("file")
                .long("file")
                .short('f')
                .value_name("FILE")
                .help("File containing commands (one per line)"),
        )
        .arg(
            Arg::new("atomic")
                .long("atomic")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .help("Execute all or none (requires checkpoint support)"),
        )
        .arg(
            Arg::new("stop-on-error")
                .long("stop-on-error")
                .action(clap::ArgAction::SetTrue)
                .help("Stop execution if a command fails"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("commands")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Commands to execute"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview batch execution"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "isolate batch add feat1 add feat2     Execute multiple commands",
                "isolate batch -f commands.txt        Execute commands from file",
                "isolate batch --atomic --dry-run     Preview execution",
            ],
            None,
        ))
}
