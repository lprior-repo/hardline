//! Source Control Plane - Unified CLI
//!
//! One CLI for workspace isolation (Isolate) and queue management (Stak).

use std::process::ExitCode;

mod cli;
mod commands;

fn main() -> ExitCode {
    cli::main::main()
}
