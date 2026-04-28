//! Output format for AI-first CLI design.
//!
//! All output is JSONL (JSON Lines) format for machine consumption.
//! Each line is a complete, parseable JSON object.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

/// Output format marker - always JSONL for AI-first design.
///
/// All output from isolate is JSONL (JSON Lines) format.
/// Each output line is a complete, parseable JSON object.
///
/// # Examples
///
/// ```
/// use isolate_core::OutputFormat;
///
/// let format = OutputFormat::Json;
/// assert!(format.is_json());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON output format - structured, machine-readable
    #[default]
    Json,
}

impl OutputFormat {
    /// Create a new JSON output format.
    #[must_use]
    pub const fn json() -> Self {
        Self::Json
    }

    /// Always returns true - all output is JSON.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        true
    }

    /// Convert a boolean to `OutputFormat`.
    ///
    /// For backward compatibility with `--json` flag pattern.
    /// Always returns `Json` variant since all output is JSON.
    #[must_use]
    pub const fn from_json_flag(_json: bool) -> Self {
        Self::Json
    }

    /// Always returns true - all output is JSON.
    #[must_use]
    pub const fn to_json_flag(&self) -> bool {
        true
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_json_variant_exists() {
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_is_json_always_true() {
        let format = OutputFormat::Json;
        assert!(format.is_json());
    }

    #[test]
    fn test_output_format_default_is_json() {
        let format = OutputFormat::default();
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_display() {
        let format = OutputFormat::Json;
        assert_eq!(format.to_string(), "json");
    }

    #[test]
    fn test_output_format_from_json_flag_always_json() {
        assert_eq!(OutputFormat::from_json_flag(true), OutputFormat::Json);
        assert_eq!(OutputFormat::from_json_flag(false), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_to_json_flag_always_true() {
        let format = OutputFormat::Json;
        assert!(format.to_json_flag());
    }

    #[test]
    fn test_output_format_serde_round_trip() {
        let format = OutputFormat::Json;
        let json = serde_json::to_string(&format).expect("serialize");
        assert_eq!(json, "\"json\"");

        let deserialized: OutputFormat = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_clone() {
        let format = OutputFormat::Json;
        let cloned = format;
        assert_eq!(format, cloned);
    }

    #[test]
    fn test_output_format_copy() {
        let format = OutputFormat::Json;
        let copied = format;
        assert_eq!(format, copied);
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_json_constructor() {
        let format = OutputFormat::json();
        assert_eq!(format, OutputFormat::Json);
    }
}
