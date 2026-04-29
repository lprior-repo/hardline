//! Data types for the retry command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the retry command,
//! which retries the last failed VCS operation.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the retry command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct RetryOptions {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Whether to use verbose output.
    pub verbose: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the retry command.
///
/// Errors are propagated via `Result`, not stored in this struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct RetryOutput {
    /// Whether the retry succeeded.
    pub success: bool,
    /// Number of attempts made.
    pub attempts: u32,
    /// Message describing the outcome.
    pub message: String,
}

// ============================================================================
// Operation Log Types
// ============================================================================

/// A recorded VCS operation for retry purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastOperation {
    /// The type of VCS operation (e.g. "push", "pull", "commit").
    pub operation: String,
    /// Arguments passed to the operation.
    pub args: Vec<String>,
    /// Whether the original operation succeeded.
    pub succeeded: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
    /// Timestamp of the operation (ISO 8601).
    pub timestamp: String,
}

/// The file path where the last operation is recorded.
pub const LAST_OPERATION_PATH: &str = ".hd/last_operation.json";
