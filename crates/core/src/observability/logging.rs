//! Structured logging types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::tracer::{SpanId, TraceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: serde_json::Value,
}

impl KeyValue {
    #[must_use]
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub span_id: Option<SpanId>,
    pub trace_id: Option<TraceId>,
    pub attributes: Vec<KeyValue>,
}

impl LogEntry {
    #[must_use]
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            target: target.into(),
            message: message.into(),
            span_id: None,
            trace_id: None,
            attributes: Vec::new(),
        }
    }

    pub fn with_span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }

    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    pub fn with_attributes(mut self, attributes: Vec<KeyValue>) -> Self {
        self.attributes = attributes;
        self
    }
}

#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)*) => {
        tracing::info!(target: $target, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $span_id:expr, $error:expr, $($arg:tt)*) => {
        tracing::error!(
            target: $target,
            span_id = %$span_id,
            error = %$error,
            $($arg)*
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn test_key_value_new() {
        let kv = KeyValue::new("key", serde_json::json!("value"));
        assert_eq!(kv.key, "key");
        assert_eq!(kv.value, serde_json::json!("value"));
    }

    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new(LogLevel::Info, "test_target", "test message");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "test_target");
        assert_eq!(entry.message, "test message");
        assert!(entry.span_id.is_none());
        assert!(entry.trace_id.is_none());
    }

    #[test]
    fn test_log_entry_with_span() {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        let entry = LogEntry::new(LogLevel::Info, "test", "msg")
            .with_span_id(span_id)
            .with_trace_id(trace_id);

        assert_eq!(entry.span_id, Some(span_id));
        assert_eq!(entry.trace_id, Some(trace_id));
    }

    #[test]
    fn test_log_entry_with_attributes() {
        let attrs = vec![
            KeyValue::new("key1", serde_json::json!("value1")),
            KeyValue::new("key2", serde_json::json!(42)),
        ];
        let entry = LogEntry::new(LogLevel::Info, "test", "msg").with_attributes(attrs);

        assert_eq!(entry.attributes.len(), 2);
        assert_eq!(entry.attributes[0].key, "key1");
        assert_eq!(entry.attributes[1].key, "key2");
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(LogLevel::Info, "test_target", "test message");
        let json = serde_json::to_string(&entry).expect("should serialize");
        assert!(json.contains("\"level\":\"Info\""));
        assert!(json.contains("test_target"));
        assert!(json.contains("test message"));
    }
}
