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
//! ### Type Hierarchy
//!
//! **Core output types:**
//! - [`OutputLine`] - Top-level output line enum (all possible outputs)
//! - [`Session`] - Session state and information
//! - [`Action`] - Action execution status
//!
//! **Domain types** (`domain_types`):
//! - [`BeadId`] - Bead/task identifier
//! - [`IssueId`] - Issue identifier
//! - [`AgentId`] - Agent identifier
//! - [`Command`] - Command execution metadata
//!
//! ## Usage Patterns
//!
//! ### Basic Output Emission
//!
//! ```rust
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use std::path::PathBuf;
//!
//! use chrono::Utc;
//! use isolate_core::{
//!     output::{emit_stdout, OutputLine, SessionOutput},
//!     types::SessionStatus,
//!     WorkspaceState,
//! };
//!
//! let session = SessionOutput {
//!     name: "my-session".to_string(),
//!     status: SessionStatus::Active,
//!     state: WorkspaceState::Working,
//!     workspace_path: PathBuf::from("/path/to/workspace"),
//!     branch: None,
//!     metadata: None,
//!     created_at: Utc::now(),
//!     updated_at: Utc::now(),
//! };
//!
//! let output = OutputLine::Session(session);
//! emit_stdout(&output)?;
//! # Ok(())
//! # }
//! ```
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
//! - **Serialization failed** - Could not serialize to JSON
//! - **Write failed** - Could not write to output stream
//! - **Invalid data** - Output data violates schema
//!
//! All output operations return `Result<(), OutputLineError>`.
//!
//! ## Related Modules
//!
//! - **`crate::domain`** - Core domain types
//! - **`crate::coordination`** - Coordination output types
//! - **`crate::beads`** - Beads output types

pub mod domain_types;
pub mod test_utils;
mod types;
mod writer;

pub use domain_types::{
    ActionResult, ActionTarget, ActionVerb, AgentAssignment, BaseRef, BeadAttachment, BeadId,
    Command, ExecutionMode, IssueId, IssueScope, IssueTitle, Message, Outcome, PlanDescription,
    PlanTitle, RecoveryCapability, RecoveryExecution, ValidatedMetadata, WarningCode,
};
pub use test_utils::{OutputEmitter, StdoutEmitter, VecEmitter};
pub use types::{
    Action, ActionStatus, Assessment, ConflictAnalysis, ConflictDetail, ConflictType, Context,
    ErrorSeverity, Issue, IssueKind, IssueSeverity, OutputLine, OutputLineError, Plan, PlanStep,
    Recovery, RecoveryAction, ResolutionOption, ResolutionRisk, ResolutionStrategy, ResultKind,
    ResultOutput, Session, SessionOutput, SessionState, Summary, SummaryType, Warning,
};
pub use writer::{emit, emit_all_stdout, emit_stdout, JsonlConfig, JsonlWriter};

#[cfg(test)]
mod tests;
