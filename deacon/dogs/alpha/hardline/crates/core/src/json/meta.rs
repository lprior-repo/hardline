//! Response metadata structures

use serde::{Deserialize, Serialize};

/// Response metadata for debugging and tracing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseMeta {
    /// Command that generated this response
    pub command: String,
    /// Timestamp of response generation (ISO 8601)
    pub timestamp: String,
    /// Duration of command execution in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether this was a dry-run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    /// Whether the operation is reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    /// Command to undo this operation (if reversible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Agent ID if executed by an agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl ResponseMeta {
    /// Create new metadata for a command
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
            dry_run: None,
            reversible: None,
            undo_command: None,
            request_id: None,
            agent_id: None,
        }
    }

    /// Set duration
    #[must_use]
    pub fn with_duration(self, ms: u64) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: Some(ms),
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Mark as dry run
    #[must_use]
    pub fn as_dry_run(self) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: Some(true),
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Mark as reversible with undo command
    #[must_use]
    pub fn with_undo(self, undo_cmd: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: Some(true),
            undo_command: Some(undo_cmd.into()),
            request_id: self.request_id,
            agent_id: self.agent_id,
        }
    }

    /// Set agent ID
    #[must_use]
    pub fn with_agent(self, agent_id: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: self.request_id,
            agent_id: Some(agent_id.into()),
        }
    }

    /// Set request ID
    #[must_use]
    pub fn with_request_id(self, request_id: impl Into<String>) -> Self {
        Self {
            command: self.command,
            timestamp: self.timestamp,
            duration_ms: self.duration_ms,
            dry_run: self.dry_run,
            reversible: self.reversible,
            undo_command: self.undo_command,
            request_id: Some(request_id.into()),
            agent_id: self.agent_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ResponseMeta::new ────────────────────────────────────────────────────

    #[test]
    fn test_meta_new() {
        let meta = ResponseMeta::new("status");
        assert_eq!(meta.command, "status");
        assert!(meta.duration_ms.is_none());
        assert!(meta.dry_run.is_none());
        assert!(meta.reversible.is_none());
        assert!(meta.undo_command.is_none());
        assert!(meta.request_id.is_none());
        assert!(meta.agent_id.is_none());
        // timestamp should be a valid ISO 8601 string
        assert!(!meta.timestamp.is_empty());
    }

    #[test]
    fn test_meta_new_sets_timestamp() {
        let before = chrono::Utc::now();
        let meta = ResponseMeta::new("test");
        let after = chrono::Utc::now();
        // Parse the timestamp and verify it's between before and after
        let ts = chrono::DateTime::parse_from_rfc3339(&meta.timestamp)
            .expect("valid timestamp");
        assert!(ts >= before);
        assert!(ts <= after);
    }

    #[test]
    fn test_meta_new_with_string_type() {
        let meta = ResponseMeta::new(String::from("list"));
        assert_eq!(meta.command, "list");
    }

    // ── with_duration ────────────────────────────────────────────────────────

    #[test]
    fn test_meta_with_duration() {
        let meta = ResponseMeta::new("test").with_duration(250);
        assert_eq!(meta.duration_ms, Some(250));
        // Other fields preserved
        assert_eq!(meta.command, "test");
    }

    #[test]
    fn test_meta_with_duration_zero() {
        let meta = ResponseMeta::new("test").with_duration(0);
        assert_eq!(meta.duration_ms, Some(0));
    }

    // ── as_dry_run ───────────────────────────────────────────────────────────

    #[test]
    fn test_meta_as_dry_run() {
        let meta = ResponseMeta::new("test").as_dry_run();
        assert_eq!(meta.dry_run, Some(true));
        assert_eq!(meta.command, "test");
    }

    // ── with_undo ────────────────────────────────────────────────────────────

    #[test]
    fn test_meta_with_undo() {
        let meta = ResponseMeta::new("delete").with_undo("session restore");
        assert_eq!(meta.reversible, Some(true));
        assert_eq!(meta.undo_command.as_deref(), Some("session restore"));
    }

    #[test]
    fn test_meta_with_undo_string_type() {
        let meta = ResponseMeta::new("delete").with_undo(String::from("undo cmd"));
        assert_eq!(meta.undo_command.as_deref(), Some("undo cmd"));
    }

    // ── with_agent ───────────────────────────────────────────────────────────

    #[test]
    fn test_meta_with_agent() {
        let meta = ResponseMeta::new("test").with_agent("agent-1");
        assert_eq!(meta.agent_id.as_deref(), Some("agent-1"));
    }

    // ── with_request_id ──────────────────────────────────────────────────────

    #[test]
    fn test_meta_with_request_id() {
        let meta = ResponseMeta::new("test").with_request_id("req-123");
        assert_eq!(meta.request_id.as_deref(), Some("req-123"));
    }

    // ── Chained builders ─────────────────────────────────────────────────────

    #[test]
    fn test_meta_chained_builders() {
        let meta = ResponseMeta::new("apply")
            .with_duration(500)
            .as_dry_run()
            .with_undo("session rollback")
            .with_agent("claude")
            .with_request_id("uuid-abc");

        assert_eq!(meta.command, "apply");
        assert_eq!(meta.duration_ms, Some(500));
        assert_eq!(meta.dry_run, Some(true));
        assert_eq!(meta.reversible, Some(true));
        assert_eq!(meta.undo_command.as_deref(), Some("session rollback"));
        assert_eq!(meta.agent_id.as_deref(), Some("claude"));
        assert_eq!(meta.request_id.as_deref(), Some("uuid-abc"));
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_meta_serde_roundtrip_minimal() {
        let meta = ResponseMeta::new("status");
        let json = serde_json::to_string(&meta).expect("serialize ok");
        let deserialized: ResponseMeta =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(meta.command, deserialized.command);
        assert_eq!(meta.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_meta_serde_roundtrip_full() {
        let meta = ResponseMeta::new("apply")
            .with_duration(300)
            .as_dry_run()
            .with_undo("rollback")
            .with_agent("test-agent")
            .with_request_id("req-xyz");

        let json = serde_json::to_string(&meta).expect("serialize ok");
        let deserialized: ResponseMeta =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.command, "apply");
        assert_eq!(deserialized.duration_ms, Some(300));
        assert_eq!(deserialized.dry_run, Some(true));
        assert_eq!(deserialized.reversible, Some(true));
        assert_eq!(deserialized.undo_command.as_deref(), Some("rollback"));
        assert_eq!(deserialized.agent_id.as_deref(), Some("test-agent"));
        assert_eq!(deserialized.request_id.as_deref(), Some("req-xyz"));
    }

    #[test]
    fn test_meta_serde_skips_none_fields() {
        let meta = ResponseMeta::new("test");
        let json_val = serde_json::to_value(&meta).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("duration_ms"));
        assert!(!obj.contains_key("dry_run"));
        assert!(!obj.contains_key("reversible"));
        assert!(!obj.contains_key("undo_command"));
        assert!(!obj.contains_key("request_id"));
        assert!(!obj.contains_key("agent_id"));
    }

    #[test]
    fn test_meta_serde_includes_optional_fields() {
        let meta = ResponseMeta::new("test").with_duration(100);
        let json_val = serde_json::to_value(&meta).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(obj.contains_key("duration_ms"));
    }

    // ── Clone / Debug / PartialEq ────────────────────────────────────────────

    #[test]
    fn test_meta_clone() {
        let meta = ResponseMeta::new("test").with_duration(42);
        let cloned = meta.clone();
        assert_eq!(meta, cloned);
    }

    #[test]
    fn test_meta_equality() {
        // Two metas created at the same time with same fields should be equal
        let meta = ResponseMeta::new("test").with_duration(100);
        assert_eq!(meta.command, "test");
        // Note: equality also compares timestamp, which may differ for separate constructions
        let a = meta.clone();
        assert_eq!(meta, a);
    }

    #[test]
    fn test_meta_inequality() {
        let a = ResponseMeta::new("a");
        let b = ResponseMeta::new("b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_meta_debug() {
        let meta = ResponseMeta::new("debug-cmd");
        let debug = format!("{meta:?}");
        assert!(debug.contains("debug-cmd"));
    }
}
