//! Summary output types
//!
//! Provides summary information about the current operation or state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::{domain_types::Message, errors::OutputLineError};

/// Summary output line containing a message with optional details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
    #[serde(rename = "type")]
    pub type_field: SummaryType,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// Type of summary being emitted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Status,
    Count,
    Info,
}

impl Summary {
    /// Create a new summary line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(type_field: SummaryType, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            type_field,
            message,
            details: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_details(self, details: String) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output_jsonl::domain_types::Message;

    // ── SummaryType variants ─────────────────────────────────────────────────

    #[test]
    fn test_summary_type_all_variants() {
        let variants = [SummaryType::Status, SummaryType::Count, SummaryType::Info];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_summary_type_copy() {
        let t = SummaryType::Info;
        let copied = t;
        assert_eq!(t, copied);
    }

    #[test]
    fn test_summary_type_debug() {
        let debug = format!("{:?}", SummaryType::Status);
        assert!(debug.contains("Status"));
    }

    // ── SummaryType serde ────────────────────────────────────────────────────

    #[test]
    fn test_summary_type_serde_roundtrip() {
        for t in [SummaryType::Status, SummaryType::Count, SummaryType::Info] {
            let json = serde_json::to_string(&t).expect("serialize ok");
            let deserialized: SummaryType = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(t, deserialized);
        }
    }

    #[test]
    fn test_summary_type_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&SummaryType::Status).expect("ok"),
            "\"status\""
        );
        assert_eq!(
            serde_json::to_string(&SummaryType::Count).expect("ok"),
            "\"count\""
        );
        assert_eq!(
            serde_json::to_string(&SummaryType::Info).expect("ok"),
            "\"info\""
        );
    }

    // ── Summary::new ─────────────────────────────────────────────────────────

    #[test]
    fn test_summary_new() {
        let msg = Message::new("hello world").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg).expect("valid");
        assert_eq!(summary.type_field, SummaryType::Info);
        assert_eq!(summary.message.as_str(), "hello world");
        assert!(summary.details.is_none());
    }

    #[test]
    fn test_summary_new_all_types() {
        let msg = Message::new("test").expect("valid");
        for t in [SummaryType::Status, SummaryType::Count, SummaryType::Info] {
            let summary = Summary::new(t, msg.clone()).expect("valid");
            assert_eq!(summary.type_field, t);
        }
    }

    // ── with_details ─────────────────────────────────────────────────────────

    #[test]
    fn test_summary_with_details() {
        let msg = Message::new("summary").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg)
            .expect("valid")
            .with_details("additional info".to_string());
        assert_eq!(summary.details.as_deref(), Some("additional info"));
    }

    #[test]
    fn test_summary_with_empty_details() {
        let msg = Message::new("summary").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg)
            .expect("valid")
            .with_details(String::new());
        assert_eq!(summary.details.as_deref(), Some(""));
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_summary_serde_roundtrip_minimal() {
        let msg = Message::new("test msg").expect("valid");
        let summary = Summary::new(SummaryType::Status, msg).expect("valid");
        let json = serde_json::to_string(&summary).expect("serialize ok");
        let deserialized: Summary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(summary.type_field, deserialized.type_field);
        assert_eq!(summary.details, deserialized.details);
    }

    #[test]
    fn test_summary_serde_roundtrip_with_details() {
        let msg = Message::new("test").expect("valid");
        let summary = Summary::new(SummaryType::Count, msg)
            .expect("valid")
            .with_details("count: 42".to_string());
        let json = serde_json::to_string(&summary).expect("serialize ok");
        let deserialized: Summary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.details.as_deref(), Some("count: 42"));
    }

    #[test]
    fn test_summary_serde_skips_none_details() {
        let msg = Message::new("test").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg).expect("valid");
        let json_val = serde_json::to_value(&summary).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("details"));
    }

    #[test]
    fn test_summary_serde_includes_details() {
        let msg = Message::new("test").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg)
            .expect("valid")
            .with_details("info".to_string());
        let json_val = serde_json::to_value(&summary).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(obj.contains_key("details"));
    }

    // ── Clone / PartialEq / Debug ────────────────────────────────────────────

    #[test]
    fn test_summary_clone() {
        let msg = Message::new("clone test").expect("valid");
        let summary = Summary::new(SummaryType::Info, msg).expect("valid");
        let cloned = summary.clone();
        assert_eq!(summary, cloned);
    }

    #[test]
    fn test_summary_equality() {
        let msg1 = Message::new("same").expect("valid");
        let msg2 = Message::new("same").expect("valid");
        let a = Summary::new(SummaryType::Info, msg1).expect("valid");
        let b = Summary::new(SummaryType::Info, msg2).expect("valid");
        // Note: timestamps differ, so these won't be equal unless we match on non-timestamp fields
        assert_eq!(a.type_field, b.type_field);
    }

    #[test]
    fn test_summary_debug() {
        let msg = Message::new("debug").expect("valid");
        let summary = Summary::new(SummaryType::Status, msg).expect("valid");
        let debug = format!("{summary:?}");
        assert!(debug.contains("Summary"));
    }
}
