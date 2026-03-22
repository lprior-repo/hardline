//! Validated task ID and bead ID
//!
//! Semantic newtypes for task and bead identifiers that guarantee valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::validation::validate_task_id;

/// A validated task ID (bead ID format)
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::TaskId;
///
/// let task = TaskId::parse("bd-abc123def456")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with "bd-" prefix
/// - Followed by hexadecimal characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TaskId(String);

impl TaskId {
    /// Parse and validate a task ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_task_id(&s)?;
        Ok(Self(s))
    }

    /// Get the task ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for TaskId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated bead ID (same as task ID)
///
/// Alias for `TaskId` since beads and tasks use the same ID format.
pub type BeadId = TaskId;
