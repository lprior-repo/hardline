//! Warning output types
//!
//! Provides warning reporting for non-critical issues.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{Message, WarningCode};
use crate::output_jsonl::errors::OutputLineError;

/// Warning output line for non-critical issues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Warning {
    pub code: WarningCode,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// Context for a warning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub session: String,
    pub workspace: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<serde_json::Value>,
}

impl Warning {
    /// Create a new warning output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(code: WarningCode, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            code,
            message,
            context: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_context(self, session: String, workspace: PathBuf) -> Self {
        Self {
            context: Some(Context {
                session,
                workspace,
                additional: None,
            }),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output_jsonl::domain_types::{Message, WarningCode};

    fn make_warning_code() -> WarningCode {
        WarningCode::new("W001").expect("valid code")
    }

    // ── Warning::new ─────────────────────────────────────────────────────────

    #[test]
    fn test_warning_new() {
        let msg = Message::new("disk space low").expect("valid");
        let warning = Warning::new(make_warning_code(), msg).expect("valid");
        assert_eq!(warning.message.as_str(), "disk space low");
        assert!(warning.context.is_none());
    }

    // ── with_context ─────────────────────────────────────────────────────────

    #[test]
    fn test_warning_with_context() {
        let msg = Message::new("warning msg").expect("valid");
        let warning = Warning::new(make_warning_code(), msg)
            .expect("valid")
            .with_context("session-1".to_string(), PathBuf::from("/tmp/ws"));

        assert!(warning.context.is_some());
        let ctx = warning.context.expect("has context");
        assert_eq!(ctx.session, "session-1");
        assert_eq!(ctx.workspace, PathBuf::from("/tmp/ws"));
        assert!(ctx.additional.is_none());
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_warning_serde_roundtrip_minimal() {
        let msg = Message::new("test warning").expect("valid");
        let warning = Warning::new(make_warning_code(), msg).expect("valid");
        let json = serde_json::to_string(&warning).expect("serialize ok");
        let deserialized: Warning =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(warning.code, deserialized.code);
        assert!(deserialized.context.is_none());
    }

    #[test]
    fn test_warning_serde_roundtrip_with_context() {
        let msg = Message::new("ctx warning").expect("valid");
        let warning = Warning::new(make_warning_code(), msg)
            .expect("valid")
            .with_context("s1".to_string(), PathBuf::from("/home/user/ws"));

        let json = serde_json::to_string(&warning).expect("serialize ok");
        let deserialized: Warning =
            serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.context.is_some());
        let ctx = deserialized.context.expect("has context");
        assert_eq!(ctx.session, "s1");
    }

    #[test]
    fn test_warning_serde_skips_none_context() {
        let msg = Message::new("no ctx").expect("valid");
        let warning = Warning::new(make_warning_code(), msg).expect("valid");
        let json_val = serde_json::to_value(&warning).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("context"));
    }

    // ── Context ──────────────────────────────────────────────────────────────

    #[test]
    fn test_context_equality() {
        let a = Context {
            session: "s1".to_string(),
            workspace: PathBuf::from("/tmp"),
            additional: None,
        };
        let b = Context {
            session: "s1".to_string(),
            workspace: PathBuf::from("/tmp"),
            additional: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_context_inequality() {
        let a = Context {
            session: "s1".to_string(),
            workspace: PathBuf::from("/tmp"),
            additional: None,
        };
        let b = Context {
            session: "s2".to_string(),
            workspace: PathBuf::from("/tmp"),
            additional: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_context_serde_roundtrip() {
        let ctx = Context {
            session: "session-x".to_string(),
            workspace: PathBuf::from("/home/user"),
            additional: Some(serde_json::json!({"key": "val"})),
        };
        let json = serde_json::to_string(&ctx).expect("serialize ok");
        let deserialized: Context =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(ctx.session, deserialized.session);
        assert!(deserialized.additional.is_some());
    }

    // ── Clone / Debug ────────────────────────────────────────────────────────

    #[test]
    fn test_warning_clone() {
        let msg = Message::new("clone").expect("valid");
        let warning = Warning::new(make_warning_code(), msg).expect("valid");
        let cloned = warning.clone();
        assert_eq!(warning.code, cloned.code);
    }

    #[test]
    fn test_warning_debug() {
        let msg = Message::new("debug").expect("valid");
        let warning = Warning::new(make_warning_code(), msg).expect("valid");
        let debug = format!("{warning:?}");
        assert!(debug.contains("Warning"));
    }
}
