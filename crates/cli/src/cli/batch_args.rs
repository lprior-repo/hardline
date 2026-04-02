//! Batch command definitions
//!
//! Subcommand enum for batch command execution.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum BatchCommands {
    /// Execute a batch of commands atomically
    Run {
        /// Workspace name (default: current workspace)
        #[arg(short, long)]
        workspace: Option<String>,

        /// Commands to execute
        #[arg(trailing_var_arg = true)]
        commands: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct BatchParser {
        #[command(subcommand)]
        command: BatchCommands,
    }

    fn parse(args: &[&str]) -> BatchCommands {
        let full: Vec<&str> = std::iter::once("scp")
            .chain(args.iter().copied())
            .collect();
        BatchParser::parse_from(full).command
    }

    #[test]
    fn run_default_workspace() {
        let BatchCommands::Run { workspace, commands } = parse(&["run", "echo", "hello"]);
        assert_eq!(workspace, None);
        assert_eq!(commands, vec!["echo", "hello"]);
    }

    #[test]
    fn run_with_workspace() {
        let BatchCommands::Run { workspace, commands } =
            parse(&["run", "-w", "my-ws", "echo", "hello"]);
        assert_eq!(workspace, Some("my-ws".to_string()));
        assert_eq!(commands, vec!["echo", "hello"]);
    }

    #[test]
    fn run_with_long_workspace() {
        let BatchCommands::Run { workspace, commands } =
            parse(&["run", "--workspace", "my-ws", "cmd1", "cmd2", "cmd3"]);
        assert_eq!(workspace, Some("my-ws".to_string()));
        assert_eq!(commands, vec!["cmd1", "cmd2", "cmd3"]);
    }

    #[test]
    fn run_single_command() {
        let BatchCommands::Run { commands, .. } = parse(&["run", "just-build"]);
        assert_eq!(commands, vec!["just-build"]);
    }

    #[test]
    fn run_empty_commands_allowed() {
        // clap with trailing_var_arg may accept zero args
        let BatchCommands::Run { commands, .. } = parse(&["run"]);
        assert!(commands.is_empty());
    }
}
