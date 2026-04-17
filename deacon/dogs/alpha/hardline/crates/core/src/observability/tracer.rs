//! Tracing and Span types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::logging::KeyValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(u128);

impl TraceId {
    #[must_use]
    pub fn new() -> Self {
        Self(rand::random())
    }

    #[must_use]
    pub fn as_u128(self) -> u128 {
        self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl SpanId {
    #[must_use]
    pub fn new() -> Self {
        Self(rand::random())
    }

    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub timestamp: DateTime<Utc>,
    pub name: String,
    pub attributes: Vec<KeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
    pub attributes: Vec<KeyValue>,
    pub events: Vec<SpanEvent>,
}

impl Span {
    #[must_use]
    pub fn new(
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: Option<SpanId>,
        operation_name: String,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id,
            operation_name,
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Ok,
            attributes: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn end(&mut self) {
        let now = Utc::now();
        self.end_time = Some(now);
        if let Some(end) = self.end_time {
            self.duration_ms = Some((end - self.start_time).num_milliseconds() as u64);
        }
    }

    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    pub fn add_attribute(&mut self, key: String, value: serde_json::Value) {
        self.attributes.push(KeyValue { key, value });
    }

    pub fn add_event(&mut self, name: String, attributes: Vec<KeyValue>) {
        self.events.push(SpanEvent {
            timestamp: Utc::now(),
            name,
            attributes,
        });
    }
}

pub trait SpanProcessor: Send + Sync {
    fn process(&self, span: Span);
}

pub struct ConsoleSpanProcessor;

impl SpanProcessor for ConsoleSpanProcessor {
    fn process(&self, span: Span) {
        let duration = span
            .duration_ms
            .map(|d| format!("{}ms", d))
            .unwrap_or_default();
        let status = format!("{:?}", span.status);

        println!(
            "[{}] {} - {}{} ({:?})",
            span.start_time.format("%Y-%m-%d %H:%M:%S%.3f"),
            span.operation_name,
            span.trace_id,
            duration,
            status
        );

        for event in &span.events {
            println!("  Event: {} at {}", event.name, event.timestamp);
        }
    }
}

pub trait Sampler: Send + Sync {
    fn should_sample(&self, operation_name: &str) -> bool;
}

pub struct AlwaysSample;

impl Sampler for AlwaysSample {
    fn should_sample(&self, _operation_name: &str) -> bool {
        true
    }
}

pub struct NeverSample;

impl Sampler for NeverSample {
    fn should_sample(&self, _operation_name: &str) -> bool {
        false
    }
}

struct TracerInner {
    processor: Arc<dyn SpanProcessor>,
    #[allow(dead_code)]
    sampler: Arc<dyn Sampler>,
}

pub struct Tracer {
    inner: Arc<TracerInner>,
}

impl Tracer {
    #[must_use]
    pub fn new(processor: Arc<dyn SpanProcessor>, sampler: Arc<dyn Sampler>) -> Self {
        Self {
            inner: Arc::new(TracerInner { processor, sampler }),
        }
    }

    pub fn start_span(&self, name: &str, parent: Option<&Span>) -> Span {
        let trace_id = parent.map_or_else(TraceId::new, |p| p.trace_id);
        let span_id = SpanId::new();
        let parent_span_id = parent.map(|p| p.span_id);

        Span::new(trace_id, span_id, parent_span_id, name.to_string())
    }

    pub fn add_event(&self, span: &mut Span, name: &str, attributes: Vec<KeyValue>) {
        span.add_event(name.to_string(), attributes);
    }

    pub fn set_error(&self, span: &mut Span, error: &dyn std::error::Error) {
        span.status = SpanStatus::Error;
        span.attributes.push(KeyValue {
            key: "error.message".to_string(),
            value: serde_json::json!(error.to_string()),
        });
        span.attributes.push(KeyValue {
            key: "error.type".to_string(),
            value: serde_json::json!(std::any::type_name_of_val(error)),
        });
    }

    pub fn end_span(&self, span: Span) {
        let mut span = span;
        span.end();
        self.inner.processor.process(span);
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new(Arc::new(ConsoleSpanProcessor), Arc::new(AlwaysSample))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_new() {
        let id = TraceId::new();
        assert!(id.as_u128() != 0);
    }

    #[test]
    fn test_span_id_new() {
        let id = SpanId::new();
        assert!(id.as_u64() != 0);
    }

    #[test]
    fn test_span_creation() {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        let span = Span::new(trace_id, span_id, None, "test_operation".to_string());

        assert_eq!(span.operation_name, "test_operation");
        assert!(span.end_time.is_none());
        assert!(span.duration_ms.is_none());
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn test_span_end() {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        let mut span = Span::new(trace_id, span_id, None, "test_operation".to_string());

        span.end();

        assert!(span.end_time.is_some());
        assert!(span.duration_ms.is_some());
    }

    #[test]
    fn test_span_add_event() {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        let mut span = Span::new(trace_id, span_id, None, "test_operation".to_string());

        span.add_event(
            "test_event".to_string(),
            vec![KeyValue {
                key: "key".to_string(),
                value: serde_json::json!("value"),
            }],
        );

        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "test_event");
    }

    #[test]
    fn test_span_display() {
        let trace_id = TraceId::new();
        let span_id = SpanId::new();
        assert_eq!(format!("{}", trace_id).len(), 32);
        assert_eq!(format!("{}", span_id).len(), 16);
    }
}
