//! AI command - AI-first entry point for the CLI.
//!
//! This command is the "start here" for AI agents.
//! Provides status, workflows, and quick-start guidance.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Location, Priority, AiStatusOutput, WorkflowInfo,
//!   WorkflowStep, AiSubcommand, NextActionOutput, QuickCommand, AiOverview
//!   (inert, serializable)
//! - **Calculations** (`calculations.rs`): determine_ready_state,
//!   format_session_count, build_workflow, build_quick_start, build_overview,
//!   determine_next_action, format_status_human (pure functions)
//! - **Actions** (`actions.rs`): run, run_status, run_workflow, run_quick_start,
//!   run_next, run_default (I/O boundary: serialization + Output)
//!
//! # Module split (DEFECT-9NB-1)
//!
//! Previously a single 1480-line file. Split into submodules to stay under
//! the 300-line limit.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

mod actions;
mod calculations;
mod data;

#[cfg(test)]
mod tests;

// Re-export all public types from submodules
pub use actions::{run, run_default, run_next, run_quick_start, run_status, run_workflow};
pub use calculations::{
    build_overview, build_quick_start, build_workflow, determine_next_action, determine_ready_state,
    format_session_count, format_status_human,
};
pub use data::{
    AiEnvelope, AiOptions, AiOverview, AiStatusOutput, AiSubcommand, Location, NextActionOutput,
    Priority, QuickCommand, QuickStartOutput, SubcommandInfo, WorkflowInfo, WorkflowStep,
};
