//! Unified identifier error type
//!
//! This module provides the canonical `IdentifierError` enum used by all
//! identifier types in the domain layer.

use thiserror::Error;

/// Unified error type for all identifier validation.
///
/// This follows DDD principles: clear error taxonomy for expected domain failures.
/// All identifier types use this single error type, making error handling consistent
/// across the domain layer.
///
/// # Error Categories
///
/// 1. **`Empty`**: Identifier is empty or whitespace-only
/// 2. **`TooLong`**: Exceeds maximum length for identifier type
/// 3. **`InvalidCharacters`**: Contains characters not allowed for identifier type
/// 4. **`InvalidFormat`**: Does not match required format/pattern
/// 5. **`InvalidPrefix`**: Missing or wrong prefix (e.g., "bd-" for task IDs)
/// 6. **`NotAbsolutePath`**: Path is not absolute
///
/// # Module-Specific Aliases
///
/// Each module can provide type aliases for backward compatibility:
/// ```rust,ignore
/// use crate::domain::identifiers::IdentifierError;
/// type SessionNameError = IdentifierError;
/// type AgentIdError = IdentifierError;
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    /// Identifier is empty or contains only whitespace
    #[error("identifier cannot be empty")]
    Empty,

    /// Identifier exceeds maximum length
    #[error("identifier too long: {actual} characters (max {max})")]
    TooLong {
        /// The maximum allowed length
        max: usize,
        /// The actual length provided
        actual: usize,
    },

    /// Identifier contains invalid characters
    #[error("identifier contains invalid characters: {details}")]
    InvalidCharacters {
        /// Human-readable explanation of what's invalid
        details: String,
    },

    /// Identifier format is invalid (generic format error)
    #[error("invalid identifier format: {details}")]
    InvalidFormat {
        /// Human-readable explanation of format requirements
        details: String,
    },

    /// Identifier must start with a letter (alphabetic character)
    #[error("identifier must start with a letter")]
    InvalidStart {
        /// The expected starting character/pattern (for context)
        expected: char,
    },

    /// Identifier has invalid prefix (e.g., must start with "bd-")
    #[error("identifier must have prefix '{prefix}' (got: {value})")]
    InvalidPrefix {
        /// The required prefix (e.g., "bd-")
        prefix: &'static str,
        /// The actual value that was provided
        value: String,
    },

    /// Identifier hex format is invalid
    #[error("identifier has invalid hex format: {value}")]
    InvalidHex {
        /// The value that failed hex validation
        value: String,
    },

    /// Path is not absolute
    #[error("path is not absolute: {value}")]
    NotAbsolutePath {
        /// The path that was provided
        value: String,
    },

    /// Path contains null bytes
    #[error("path contains null bytes")]
    NullBytesInPath,

    /// Identifier must be ASCII
    #[error("identifier must be ASCII only: {value}")]
    NotAscii {
        /// The value that failed ASCII validation
        value: String,
    },

    /// Identifier contains path separators
    #[error("identifier cannot contain path separators")]
    ContainsPathSeparators,
}

// ============================================================================
// BACKWARD COMPATIBILITY ALIASES
// ============================================================================

/// Legacy alias for backward compatibility.
///
/// # Deprecated
/// Use `IdentifierError` instead.
pub type IdError = IdentifierError;

/// Error type for session name validation.
pub type SessionNameError = IdentifierError;

/// Error type for agent ID validation.
pub type AgentIdError = IdentifierError;

/// Error type for workspace name validation.
pub type WorkspaceNameError = IdentifierError;

/// Error type for task ID validation.
pub type TaskIdError = IdentifierError;

/// Error type for bead ID validation.
pub type BeadIdError = IdentifierError;

/// Error type for session ID validation.
pub type SessionIdError = IdentifierError;

/// Error type for absolute path validation.
pub type AbsolutePathError = IdentifierError;

// ============================================================================
// HELPER METHODS
// ============================================================================

impl IdentifierError {
    /// Create an `Empty` error variant
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Create a `TooLong` error variant
    #[must_use]
    pub const fn too_long(max: usize, actual: usize) -> Self {
        Self::TooLong { max, actual }
    }

    /// Create an `InvalidCharacters` error variant
    #[must_use]
    pub fn invalid_characters(details: impl Into<String>) -> Self {
        Self::InvalidCharacters {
            details: details.into(),
        }
    }

    /// Create an `InvalidFormat` error variant
    #[must_use]
    pub fn invalid_format(details: impl Into<String>) -> Self {
        Self::InvalidFormat {
            details: details.into(),
        }
    }

    /// Create an `InvalidStart` error variant
    #[must_use]
    pub const fn invalid_start(expected: char) -> Self {
        Self::InvalidStart { expected }
    }

    /// Create an `InvalidPrefix` error variant
    #[must_use]
    pub fn invalid_prefix(prefix: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidPrefix {
            prefix,
            value: value.into(),
        }
    }

    /// Create an `InvalidHex` error variant
    #[must_use]
    pub fn invalid_hex(value: impl Into<String>) -> Self {
        Self::InvalidHex {
            value: value.into(),
        }
    }

    /// Create a `NotAbsolutePath` error variant
    #[must_use]
    pub fn not_absolute_path(value: impl Into<String>) -> Self {
        Self::NotAbsolutePath {
            value: value.into(),
        }
    }

    /// Check if this is an `Empty` error
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Check if this is a `TooLong` error
    #[must_use]
    pub const fn is_too_long(&self) -> bool {
        matches!(self, Self::TooLong { .. })
    }

    /// Check if this is an `InvalidCharacters` error
    #[must_use]
    pub const fn is_invalid_characters(&self) -> bool {
        matches!(self, Self::InvalidCharacters { .. })
    }

    /// Check if this is an `InvalidFormat` error
    #[must_use]
    pub const fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_display() {
        let err = IdentifierError::Empty;
        let msg = format!("{err}");
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn too_long_display() {
        let err = IdentifierError::TooLong {
            max: 50,
            actual: 100,
        };
        let msg = format!("{err}");
        assert!(msg.contains("50"));
        assert!(msg.contains("100"));
        assert!(msg.contains("too long"));
    }

    #[test]
    fn invalid_characters_display() {
        let err = IdentifierError::InvalidCharacters {
            details: "spaces not allowed".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("spaces not allowed"));
        assert!(msg.contains("invalid characters"));
    }

    #[test]
    fn invalid_format_display() {
        let err = IdentifierError::InvalidFormat {
            details: "must be lowercase".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("must be lowercase"));
        assert!(msg.contains("invalid identifier format"));
    }

    #[test]
    fn invalid_start_display() {
        let err = IdentifierError::InvalidStart { expected: 'a' };
        let msg = format!("{err}");
        assert!(msg.contains("start with a letter"));
    }

    #[test]
    fn invalid_prefix_display() {
        let err = IdentifierError::InvalidPrefix {
            prefix: "bd-",
            value: "abc123".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("bd-"));
        assert!(msg.contains("abc123"));
    }

    #[test]
    fn invalid_hex_display() {
        let err = IdentifierError::InvalidHex {
            value: "xyz".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("xyz"));
        assert!(msg.contains("hex"));
    }

    #[test]
    fn not_absolute_path_display() {
        let err = IdentifierError::NotAbsolutePath {
            value: "relative/path".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("relative/path"));
        assert!(msg.contains("not absolute"));
    }

    #[test]
    fn null_bytes_in_path_display() {
        let err = IdentifierError::NullBytesInPath;
        let msg = format!("{err}");
        assert!(msg.contains("null bytes"));
    }

    #[test]
    fn not_ascii_display() {
        let err = IdentifierError::NotAscii {
            value: "café".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("café"));
        assert!(msg.contains("ASCII"));
    }

    #[test]
    fn contains_path_separators_display() {
        let err = IdentifierError::ContainsPathSeparators;
        let msg = format!("{err}");
        assert!(msg.contains("path separators"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = IdentifierError::Empty;
        let _ = IdentifierError::TooLong { max: 0, actual: 0 };
        let _ = IdentifierError::InvalidCharacters {
            details: String::new(),
        };
        let _ = IdentifierError::InvalidFormat {
            details: String::new(),
        };
        let _ = IdentifierError::InvalidStart { expected: 'a' };
        let _ = IdentifierError::InvalidPrefix {
            prefix: "",
            value: String::new(),
        };
        let _ = IdentifierError::InvalidHex {
            value: String::new(),
        };
        let _ = IdentifierError::NotAbsolutePath {
            value: String::new(),
        };
        let _ = IdentifierError::NullBytesInPath;
        let _ = IdentifierError::NotAscii {
            value: String::new(),
        };
        let _ = IdentifierError::ContainsPathSeparators;
    }

    #[test]
    fn helper_constructors() {
        let err = IdentifierError::empty();
        assert!(err.is_empty());

        let err = IdentifierError::too_long(10, 20);
        assert!(err.is_too_long());

        let err = IdentifierError::invalid_characters("bad");
        assert!(err.is_invalid_characters());

        let err = IdentifierError::invalid_format("bad");
        assert!(err.is_invalid_format());

        let err = IdentifierError::invalid_start('a');
        assert!(matches!(err, IdentifierError::InvalidStart { .. }));

        let err = IdentifierError::invalid_prefix("bd-", "abc");
        assert!(matches!(err, IdentifierError::InvalidPrefix { .. }));

        let err = IdentifierError::invalid_hex("xyz");
        assert!(matches!(err, IdentifierError::InvalidHex { .. }));

        let err = IdentifierError::not_absolute_path("rel");
        assert!(matches!(err, IdentifierError::NotAbsolutePath { .. }));
    }

    #[test]
    fn equality_and_clone() {
        let a = IdentifierError::Empty;
        let b = IdentifierError::Empty;
        assert_eq!(a, b);

        let c = IdentifierError::TooLong {
            max: 10,
            actual: 20,
        };
        let d = c.clone();
        assert_eq!(c, d);
    }
}
