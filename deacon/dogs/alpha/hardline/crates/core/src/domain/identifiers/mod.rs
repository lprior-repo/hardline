//! Semantic newtypes for domain identifiers
//!
//! # Parse-at-Boundaries Pattern
//!
//! Each identifier type:
//! - Validates its input on construction (parse-once pattern)
//! - Trims whitespace before validation (boundary sanitization)
//! - Cannot represent invalid states
//! - Provides safe access to the underlying value
//! - Implements serde serialization/deserialization with validation
//!
//! # Single Source of Truth
//!
//! This module is the canonical implementation of identifier types.
//! Other modules (`types.rs`, `cli_contracts`) should re-export these types
//! rather than defining their own implementations.
//!
//! # Unified Error Type
//!
//! All identifier validation uses a single `IdentifierError` enum with clear
//! categorization:
//! - **`Empty`**: Identifier is empty or whitespace-only
//! - **`TooLong`**: Exceeds type-specific maximum length
//! - **`InvalidCharacters`**: Contains characters not allowed for the type
//! - **`InvalidFormat`**: Generic format validation error
//! - **`InvalidStart`**: Does not start with required character
//! - **`InvalidPrefix`**: Missing required prefix (e.g., "bd-" for task IDs)
//! - **`InvalidHex`**: Invalid hexadecimal format
//! - **`NotAbsolutePath`**: Path is not absolute
//! - **`NullBytesInPath`**: Path contains null bytes
//! - **`NotAscii`**: Identifier must be ASCII-only
//! - **`ContainsPathSeparators`**: Identifier contains path separators
//!
//! This follows DDD principle of clear error taxonomy for expected domain failures.
//!
//! # Module-Specific Error Aliases
//!
//! For backward compatibility and semantic clarity, each identifier type has
//! a corresponding error alias:
//! - `SessionNameError` = `IdentifierError`
//! - `AgentIdError` = `IdentifierError`
//! - `WorkspaceNameError` = `IdentifierError`
//! - `TaskIdError` = `IdentifierError`
//! - `BeadIdError` = `IdentifierError`
//! - `SessionIdError` = `IdentifierError`
//! - `AbsolutePathError` = `IdentifierError`
//!
//! The legacy `IdError` alias is also maintained for backward compatibility.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

mod error;
mod validation;

mod absolute_path;
mod agent_id;
mod queue_entry_id;
mod session_id;
mod session_name;
mod task_id;
mod workspace_name;

#[cfg(test)]
mod tests;

// Re-export for use in validation functions
pub use error::IdentifierError;

// Re-export error aliases
pub use error::{
    AbsolutePathError, AgentIdError, BeadIdError, IdError, SessionIdError, SessionNameError,
    TaskIdError, WorkspaceNameError,
};

// Re-export identifier types
pub use absolute_path::AbsolutePath;
pub use agent_id::AgentId;
pub use queue_entry_id::QueueEntryId;
pub use session_id::SessionId;
pub use session_name::SessionName;
pub use task_id::{BeadId, TaskId};
pub use workspace_name::WorkspaceName;
