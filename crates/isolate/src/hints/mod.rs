//! Isolate hints module - contextual hints and suggestions for workspace operations.

pub mod generation;
pub mod hint_impl;
pub mod next_actions;
pub mod types;

pub use generation::{generate_hints, hints_for_error, suggest_next_actions};
pub use next_actions::next_actions_for_command;
pub use types::{
    ActionRisk, CommandContext, Hint, HintType, NextAction, SystemState, WorkspaceInfo,
};
