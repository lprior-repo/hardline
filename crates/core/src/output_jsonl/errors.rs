//! Error types for JSONL output operations
//!
//! All output operations return `Result<T, OutputLineError>` to ensure
//! validation happens at construction time.

use thiserror::Error;

/// Errors that can occur when creating output lines.
#[derive(Debug, Clone, Error)]
pub enum OutputLineError {
    #[error("message is required but was empty")]
    EmptyMessage,
    #[error("title is required but was empty")]
    EmptyTitle,
    #[error("description is required but was empty")]
    EmptyDescription,
    #[error("session name is required but was empty")]
    EmptySessionName,
    #[error("at least one action is required")]
    NoActions,
    #[error("plan step count exceeds u32::MAX")]
    PlanStepOverflow,
    #[error("recovery action count exceeds u32::MAX")]
    RecoveryActionOverflow,
    #[error("workspace path must be absolute")]
    RelativePath,
    #[error("invalid warning code: {0}")]
    InvalidWarningCode(String),
    #[error("invalid action verb: {0}")]
    InvalidActionVerb(String),
    #[error("invalid action target: {0}")]
    InvalidActionTarget(String),
}
