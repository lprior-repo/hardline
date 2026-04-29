//! AI command - AI-first entry point for the CLI.
//!
//! This command is the "start here" for AI agents.
//! Provides status, workflows, and quick-start guidance.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Location, Priority, `AiStatusOutput`, `WorkflowInfo`, `WorkflowStep`,
//!   `AiSubcommand`, `NextActionOutput`, `QuickCommand`, `AiOverview` (inert, serializable)
//! - **Calculations** (`calculations.rs`): `determine_ready_state`, `format_session_count`,
//!   `build_workflow`, `build_quick_start`, `build_overview`, `determine_next_action`,
//!   `format_status_human` (pure functions)
//! - **Actions** (`actions.rs`): run, `run_status`, `run_workflow`, `run_quick_start`, `run_next`,
//!   `run_default` (I/O boundary: serialization + Output)
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

#[cfg(test)]
mod mod_tests {
    // =========================================================================
    // Module re-exports: verify all public types are accessible from this module
    // =========================================================================

    #[test]
    fn data_types_are_accessible_via_super() {
        // Verify all public data types can be referenced from the parent module path
        use crate::commands::handlers::ai::data::{
            AiEnvelope, AiOptions, AiOverview, AiStatusOutput, AiSubcommand, Location,
            NextActionOutput, Priority, QuickCommand, QuickStartOutput, SubcommandInfo,
            WorkflowInfo, WorkflowStep,
        };

        // Just verify the types exist and are usable
        let _env: Option<AiEnvelope<String>> = None;
        let _opts = AiOptions {
            subcommand: AiSubcommand::Default,
        };
        let _loc = Location::Main;
        let _pri = Priority::Medium;
    }

    #[test]
    fn calculation_functions_are_accessible() {
        use crate::commands::handlers::ai::calculations::{
            build_overview, build_quick_start, build_workflow, determine_next_action,
            determine_ready_state, format_session_count, format_status_human,
        };

        // Verify all public calculation functions are callable
        let (ready, _, _) = determine_ready_state(false, &Location::NotInRepo);
        assert!(!ready);

        let _count = format_session_count(0);
        let _workflow = build_workflow();
        let _qs = build_quick_start();
        let _overview = build_overview();
        let _action = determine_next_action(false, &Location::NotInRepo, None, 0);

        let status = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "ok".to_string(),
            next_command: "scp work".to_string(),
        };
        let _lines = format_status_human(&status);
    }

    #[test]
    fn action_functions_are_accessible() {
        use crate::commands::handlers::ai::actions::{
            run, run_default, run_next, run_quick_start, run_status, run_workflow,
        };

        // Verify all public action functions are callable and return Result
        let opts = AiOptions {
            subcommand: AiSubcommand::Default,
        };
        match run(&opts) {
            Ok(()) => {}
            Err(e) => panic!("run should succeed: {e}"),
        }

        match run_status() {
            Ok(()) => {}
            Err(e) => panic!("run_status should succeed: {e}"),
        }
        match run_workflow() {
            Ok(()) => {}
            Err(e) => panic!("run_workflow should succeed: {e}"),
        }
        match run_quick_start() {
            Ok(()) => {}
            Err(e) => panic!("run_quick_start should succeed: {e}"),
        }
        match run_next() {
            Ok(()) => {}
            Err(e) => panic!("run_next should succeed: {e}"),
        }
        match run_default() {
            Ok(()) => {}
            Err(e) => panic!("run_default should succeed: {e}"),
        }
    }

    use crate::commands::handlers::ai::data::{AiOptions, AiStatusOutput, AiSubcommand, Location};
}

// Re-export all public types from submodules
pub use actions::run;
pub use calculations::{
    build_overview, build_quick_start, build_workflow, determine_next_action,
    determine_ready_state, format_session_count, format_status_human,
};
pub use data::{
    AiEnvelope, AiOptions, AiOverview, AiStatusOutput, AiSubcommand, Location, NextActionOutput,
    Priority, QuickCommand, QuickStartOutput, SubcommandInfo, WorkflowInfo, WorkflowStep,
};
