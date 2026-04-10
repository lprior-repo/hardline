//! Implementation module for phases

pub mod executor;
pub mod framework;
pub mod impl_failure;
pub mod impl_parallel;
pub mod impl_phases;
pub mod impl_pipeline;
pub mod impl_state;
pub mod types;

pub use executor::PipelineExecutor;
pub use framework::{
    DefaultAgentDevelopmentPhase, DefaultSpecReviewPhase, DefaultUniverseSetupPhase,
    DefaultValidationPhase, Phase, PhaseContext, PhaseRegistry,
};
pub use types::{Decision, PhaseError, PhaseResult};
