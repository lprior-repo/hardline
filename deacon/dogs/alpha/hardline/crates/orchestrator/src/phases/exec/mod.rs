//! Implementation module for phases

pub mod executor;
pub mod impl_failure;
pub mod impl_parallel;
pub mod impl_phases;
pub mod impl_pipeline;
pub mod impl_state;
pub mod types;

pub use executor::PipelineExecutor;
pub use types::{Decision, PhaseError, PhaseResult};
