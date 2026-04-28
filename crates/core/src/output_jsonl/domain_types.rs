//! Semantic domain types for output - following Scott Wlaschin's DDD principles
//!
//! This module implements:
//! - Parse at boundaries, validate once
//! - Use semantic newtypes instead of primitives
//! - Make illegal states unrepresentable
//!
//! This module re-exports from the following sibling modules:
//! - `action_verb` - Action verb enumeration
//! - `identifier_types` - `IssueId`, `BeadId`, `SessionName`
//! - `metadata_type` - `ValidatedMetadata`
//! - `state_enums` - State enums replacing bool/Option
//! - `target_types` - `ActionTarget`, `BaseRef`, Command
//! - `text_types` - `IssueTitle`, `PlanTitle`, `PlanDescription`, Message
//! - `warning_code` - `WarningCode` enumeration

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// Re-export from sibling modules
pub use crate::output_jsonl::{
    action_verb::ActionVerb,
    identifier_types::{BeadId, IssueId, SessionName},
    metadata_type::ValidatedMetadata,
    state_enums::{
        ActionResult, AgentAssignment, BeadAttachment, ExecutionMode, IssueScope, MergeAnalysis,
        Outcome, RecoveryCapability, RecoveryExecution,
    },
    target_types::{ActionTarget, BaseRef, Command},
    text_types::{IssueTitle, Message, PlanDescription, PlanTitle},
    warning_code::WarningCode,
};
