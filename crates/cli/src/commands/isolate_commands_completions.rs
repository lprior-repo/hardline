//! Completions command: generate shell completions

use clap::{Arg, Command as ClapCommand};

/// Generate shell completions
pub fn cmd_completions() -> ClapCommand {
    ClapCommand::new("completions")
        .about("Generate shell completions")
        .arg(
            Arg::new("shell")
                .required(true)
                .help("Shell to generate completions for"),
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
                "hardline completions bash             Generate bash completions",
                "hardline completions zsh              Generate zsh completions",
                "hardline completions fish             Generate fish completions",
            ],
            None,
        ))
}
