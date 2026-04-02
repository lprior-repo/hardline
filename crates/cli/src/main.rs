//! Source Control Plane - Unified CLI
//!
//! One CLI for workspace isolation (Isolate) and queue management (Stak).

#![allow(dead_code, unused_imports, unexpected_cfgs, unknown_lints)]
#![allow(clippy::must_use_attr, clippy::module_inception, clippy::from_str_instead_of_fromstr, clippy::single_call_fn, clippy::double_must_use)]

use std::process::ExitCode;

mod cli;
mod commands;

fn main() -> ExitCode {
    cli::main::main()
}
