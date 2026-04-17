//! Observability & Telemetry
//!
//! Core telemetry types and interfaces for tracing, metrics, logging, and health checks.
//!
//! # Architecture
//!
//! - `tracer` - Tracing spans and trace collection
//! - `metrics` - Metrics collection (counters, gauges, histograms)
//! - `logging` - Structured logging
//! - `health` - Health checks

pub mod health;
pub mod logging;
pub mod metrics;
pub mod tracer;

pub use health::{HealthCheck, HealthChecker, HealthState, HealthStatus};
pub use logging::{KeyValue, LogEntry, LogLevel};
pub use metrics::{Histogram, Metric, MetricValue, MetricsCollector};
pub use tracer::{Span, SpanEvent, SpanId, SpanProcessor, SpanStatus, TraceId, Tracer};
