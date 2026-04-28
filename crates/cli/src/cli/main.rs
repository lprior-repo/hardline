//! CLI execution logic
//!
//! Contains the main entry point and command dispatch logic.

use std::process::ExitCode;

use clap::Parser;
use scp_core::{output::Output, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::args::Cli;

/// Main entry point for the CLI
pub fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up verbosity for output module
    Output::set_verbose(cli.verbose, cli.quiet);

    // Set database path if provided via flag
    if let Some(db_path) = &cli.database {
        std::env::set_var("SCP_DATABASE_PATH", db_path);
    }

    // Initialize logging with appropriate level
    let log_level = if cli.quiet {
        "error".to_string()
    } else if cli.verbose {
        "debug".to_string()
    } else {
        "info".to_string()
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or(log_level),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Run the appropriate command
    let result = run_command(cli);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            if let Some(suggestion) = e.suggestion() {
                eprintln!("{}", suggestion);
            }
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Execute the appropriate command based on CLI arguments
pub fn run_command(cli: Cli) -> Result<()> {
    use crate::cli::args::Commands;

    match cli.command {
        Commands::Workspace { command } => crate::cli::dispatch_workspace::run(command),
        other => crate::cli::dispatch::run_command(other),
    }
}
