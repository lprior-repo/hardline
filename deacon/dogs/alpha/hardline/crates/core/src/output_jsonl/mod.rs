//! # Output Types for AI-First CLI
//!
//! This module provides **JSONL output types** for the AI-first control plane design.
//! Each line of output is a valid JSON object that can be parsed independently by AI consumers.
//!
//! ## Design Philosophy
//!
//! The output module follows these core principles:
//!
//! 1. **One JSON object per line** - Each output line is a complete, parseable JSON object
//! 2. **Self-describing types** - Every object includes a `"type"` field for easy routing
//! 3. **Machine-readable only** - No human-readable formatting optimization
//! 4. **Streaming-friendly** - Emit one line at a time without buffering
//! 5. **Semantic validation** - Types enforce valid output structure
//!
//! ## Architecture
//!
//! ### Domain-Driven Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//!
//! - **Parse at boundaries, validate once** - Validate output structure at emission time
//! - **Make illegal states unrepresentable** - Use enums instead of `bool`/`Option`
//! - **Use semantic newtypes** - Domain types instead of primitives
//!
//! ### Module Organization
//!
//! **Core output types:**
//! - [`action`] - Action execution reporting
//! - [`conflict`] - Conflict detection and resolution
//! - [`errors`] - Error types for output operations
//! - [`issue`] - Issue detection and reporting
//! - [`output_line`] - Top-level OutputLine enum
//! - [`plan`] - Plan structure for multi-step operations
//! - [`recovery`] - Error recovery reporting
//! - [`result`] - Operation result reporting
//! - [`session`] - Session state information
//! - [`summary`] - Summary information
//! - [`warning`] - Warning reporting
//!
//! **Domain types** (`domain_types`):
//! - [`ActionResult`] - Action execution result
//! - [`ActionTarget`] - Target of an action
//! - [`ActionVerb`] - Verb describing the action
//! - [`IssueId`] - Issue identifier
//! - [`IssueScope`] - Scope of an issue
//! - [`Message`] - A message string
//! - [`Outcome`] - Operation outcome (success/failure)
//! - [`PlanDescription`] - Plan description
//! - [`PlanTitle`] - Plan title
//! - [`RecoveryCapability`] - Recovery capability
//! - [`RecoveryExecution`] - Recovery execution mode
//! - [`SessionName`] - Session name
//! - [`WarningCode`] - Warning code
//!
//! ## Output Writers
//!
//! **Writer types:**
//! - [`JsonlWriter`] - Generic JSONL writer
//! - [`JsonlConfig`] - Writer configuration
//! - [`emit`] - Emit to any writer
//! - [`emit_stdout`] - Emit to stdout
//! - [`emit_all_stdout`] - Emit multiple lines to stdout
//!
//! **Test emitters:**
//! - [`OutputEmitter`] - Trait for output emission
//! - [`VecEmitter`] - In-memory collector for testing
//! - [`StdoutEmitter`] - Stdout emitter
//!
//! ## Error Handling
//!
//! Output errors are represented by [`OutputLineError`]:
//! - **EmptyMessage** - Message is required but was empty
//! - **EmptyTitle** - Title is required but was empty
//! - **EmptyDescription** - Description is required but was empty
//! - **EmptySessionName** - Session name is required but was empty
//! - **NoActions** - At least one action is required
//! - **PlanStepOverflow** - Plan step count exceeds u32::MAX
//! - **RecoveryActionOverflow** - Recovery action count exceeds u32::MAX
//! - **RelativePath** - Workspace path must be absolute
//! - **InvalidWarningCode** - Invalid warning code
//! - **InvalidActionVerb** - Invalid action verb
//! - **InvalidActionTarget** - Invalid action target
//!
//! All output operations return `Result<(), OutputLineError>`.

// Split modules - organized by domain concern
pub mod action;
pub mod action_verb;
pub mod conflict;
pub mod domain_types;
pub mod errors;
pub mod identifier_types;
pub mod issue;
pub mod metadata_type;
pub mod output_line;
pub mod plan;
pub mod recovery;
pub mod result;
pub mod session;
pub mod state_enums;
pub mod summary;
pub mod target_types;
pub mod test_utils;
pub mod text_types;
pub mod warning;
pub mod warning_code;

mod writer;

pub use action::Action;
pub use conflict::{
    ConflictAnalysis, ConflictDetail, ConflictType, ResolutionOption, ResolutionRisk,
    ResolutionStrategy,
};
pub use domain_types::{
    ActionResult, ActionTarget, ActionVerb, AgentAssignment, BaseRef, BeadAttachment, BeadId,
    Command, ExecutionMode, IssueId, IssueScope, IssueTitle, Message, Outcome, PlanDescription,
    PlanTitle, RecoveryCapability, RecoveryExecution, ValidatedMetadata, WarningCode,
};
pub use errors::OutputLineError;
pub use issue::{Issue, IssueKind, IssueSeverity};
pub use output_line::OutputLine;
pub use plan::{ActionStatus, Plan, PlanStep};
pub use recovery::{Assessment, ErrorSeverity, Recovery, RecoveryAction};
pub use result::{ResultKind, ResultOutput};
pub use session::{Session, SessionOutput, SessionState};
pub use summary::{Summary, SummaryType};
pub use test_utils::{OutputEmitter, StdoutEmitter, VecEmitter};
pub use warning::{Context, Warning};
pub use writer::{emit, emit_all_stdout, emit_stdout, JsonlConfig, JsonlWriter};

#[cfg(test)]
mod tests;
