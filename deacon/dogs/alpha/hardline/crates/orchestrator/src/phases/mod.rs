//! Pipeline phase executor module
//!
//! Splits the original phases.rs into logical submodules:
//! - exec/types: PhaseError, PhaseResult, Decision
//! - exec/executor: PipelineExecutor struct and all method implementations

pub mod exec;

pub use exec::{Decision, PhaseError, PhaseResult, PipelineExecutor};
