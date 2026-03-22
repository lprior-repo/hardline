//! Contextual hints and smart suggestions for AI agents
//!
//! Provides context-aware hints based on system state:
//! - Suggested next actions
//! - State explanations
//! - Learning from errors
//! - Predictive hints

pub mod generation;
pub mod helpers;
pub mod hint_impl;
pub mod next_actions;
pub mod response;
pub mod tests;
pub mod types;

// Re-export types for convenience
pub use response::{HintsResponse, SystemContext};
pub use types::{ActionRisk, CommandContext, Hint, HintType, NextAction, SystemState};

// Re-export generation functions
pub use generation::{
    generate_hints, generate_hints_response, hints_for_error, suggest_next_actions,
};

// Re-export next action functions
pub use next_actions::next_actions_for_command;

// Re-export helper functions
pub use helpers::{extract_session_name, hints_for_beads};
