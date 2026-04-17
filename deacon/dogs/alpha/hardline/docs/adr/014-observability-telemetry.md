# ADR-014: Observability & Telemetry

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs observability for:

1. **Debugging** - What happened when something goes wrong
2. **Performance** - Is the system slow, where are bottlenecks
3. **Usage patterns** - How are agents using the system
4. **Alerting** - When does something break
5. **Audit trail** - Who did what and when

For 600+ concurrent agents, observability is critical. This ADR defines the telemetry system.

---

## Decision

### Core Telemetry Types

```rust
/// Span: A unit of work with timing
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
pub struct KeyValue {
    pub key: String,
    pub value: JsonValue,
}

/// Log entry
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
    pub attributes: Vec<KeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetricValue {
    Counter { value: f64 },
    Gauge { value: f64 },
    Histogram { values: Vec<f64>, count: usize },
}
```

### Trace/Span IDs

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl TraceId {
    pub fn new() -> Self {
        Self(rand::random())
    }
}

impl SpanId {
    pub fn new() -> Self {
        Self(rand::random())
    }
}
```

### Tracer Implementation

```rust
pub struct Tracer {
    inner: Arc<TracerInner>,
}

struct TracerInner {
    processor: Arc<dyn SpanProcessor>,
    sampler: Arc<dyn Sampler>,
}

impl Tracer {
    pub fn new(processor: Arc<dyn SpanProcessor>, sampler: Arc<dyn Sampler>) -> Self {
        Self {
            inner: Arc::new(TracerInner { processor, sampler }),
        }
    }
    
    /// Start a new span
    pub fn start_span(&self, name: &str, parent: Option<&Span>) -> Span {
        let trace_id = parent
            .map(|p| p.trace_id)
            .unwrap_or_else(TraceId::new);
        
        let span_id = SpanId::new();
        let parent_span_id = parent.map(|p| p.span_id);
        
        Span {
            trace_id,
            span_id,
            parent_span_id,
            operation_name: name.to_string(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Ok,
            attributes: Vec::new(),
            events: Vec::new(),
        }
    }
    
    /// Record an event within a span
    pub fn add_event(&self, span: &mut Span, name: &str, attributes: Vec<KeyValue>) {
        span.events.push(SpanEvent {
            timestamp: Utc::now(),
            name: name.to_string(),
            attributes,
        });
    }
    
    /// Set span status to error
    pub fn set_error(&self, span: &mut Span, error: &dyn std::error::Error) {
        span.status = SpanStatus::Error;
        span.attributes.push(KeyValue {
            key: "error.message".to_string(),
            value: json!(error.to_string()),
        });
        span.attributes.push(KeyValue {
            key: "error.type".to_string(),
            value: json!(std::any::type_name_of_val(error)),
        });
    }
    
    /// End span and process
    pub fn end_span(&self, span: Span) {
        let mut span = span;
        span.end_time = Some(Utc::now());
        span.duration_ms = Some(
            (span.end_time.unwrap() - span.start_time).num_milliseconds() as u64
        );
        
        self.inner.processor.process(span);
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new(
            Arc::new(ConsoleSpanProcessor),
            Arc::new(AlwaysSample),
        )
    }
}
```

### Span Processor

```rust
pub trait SpanProcessor: Send + Sync {
    fn process(&self, span: Span);
}

pub struct ConsoleSpanProcessor;

impl SpanProcessor for ConsoleSpanProcessor {
    fn process(&self, span: Span) {
        let duration = span.duration_ms.map(|d| format!("{}ms", d)).unwrap_or_default();
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

pub struct BatchingSpanProcessor {
    sender: mpsc::Sender<Span>,
    buffer: Vec<Span>,
    batch_size: usize,
    flush_interval: Duration,
}

impl BatchingSpanProcessor {
    pub fn new(sender: mpsc::Sender<Span>, batch_size: usize, flush_interval: Duration) -> Self {
        Self {
            sender,
            buffer: Vec::with_capacity(batch_size),
            batch_size,
            flush_interval,
        }
    }
}

impl SpanProcessor for BatchingSpanProcessor {
    fn process(&self, span: Span) {
        self.buffer.push(span);
        
        if self.buffer.len() >= self.batch_size {
            // Flush
            let spans = std::mem::take(&mut self.buffer);
            self.sender.send(spans).ok();
        }
    }
}
```

### Metrics Collection

```rust
pub struct MetricsCollector {
    counters: RwLock<HashMap<String, f64>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, Histogram>>,
}

pub struct Histogram {
    values: Vec<f64>,
    sum: f64,
    count: usize,
}

impl MetricsCollector {
    pub fn increment_counter(&self, name: &str, value: f64, attributes: Vec<KeyValue>) {
        let mut counters = self.counters.write();
        *counters.entry(name.to_string()).or_insert(0.0) += value;
    }
    
    pub fn set_gauge(&self, name: &str, value: f64, attributes: Vec<KeyValue>) {
        let mut gauges = self.gauges.write();
        gauges.insert(name.to_string(), value);
    }
    
    pub fn record_histogram(&self, name: &str, value: f64, attributes: Vec<KeyValue>) {
        let mut histograms = self.histograms.write();
        let hist = histograms.entry(name.to_string()).or_insert_with(|| Histogram {
            values: Vec::new(),
            sum: 0.0,
            count: 0,
        });
        hist.values.push(value);
        hist.sum += value;
        hist.count += 1;
    }
    
    pub fn export(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        let now = Utc::now();
        
        // Export counters
        let counters = self.counters.read();
        for (name, value) in counters.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Counter { value: *value },
                timestamp: now,
                attributes: Vec::new(),
            });
        }
        
        // Export gauges
        let gauges = self.gauges.read();
        for (name, value) in gauges.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Gauge { value: *value },
                timestamp: now,
                attributes: Vec::new(),
            });
        }
        
        // Export histograms
        let histograms = self.histograms.read();
        for (name, hist) in histograms.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Histogram {
                    values: hist.values.clone(),
                    count: hist.count,
                },
                timestamp: now,
                attributes: vec![KeyValue {
                    key: "sum".to_string(),
                    value: json!(hist.sum),
                }],
            });
        }
        
        metrics
    }
}
```

### Structured Logging

```rust
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

/// Example usage
fn do_workspace_operation(workspace_id: &WorkspaceId) -> Result<(), Error> {
    let span = tracer.start_span("workspace.operation", None);
    
    log_info!("workspace", span.span_id, "Starting workspace operation";
        "workspace_id" => %workspace_id
    );
    
    let result = workspace_service.create(workspace_id);
    
    match &result {
        Ok(ws) => {
            log_info!("workspace", span.span_id, "Workspace created";
                "workspace_id" => %workspace_id,
                "backend" => ?ws.backend
            );
        }
        Err(e) => {
            log_error!("workspace", span.span_id, e, "Workspace operation failed";
                "workspace_id" => %workspace_id
            );
        }
    }
    
    span.end();
    result
}
```

### Health Checks

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub checks: Vec<HealthCheck>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthState,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
}

pub struct HealthChecker {
    db_check: Arc<DatabaseHealthCheck>,
    vcs_check: Arc<VcsHealthCheck>,
    disk_check: Arc<DiskSpaceCheck>,
}

impl HealthChecker {
    pub async fn check_all(&self) -> HealthStatus {
        let mut checks = Vec::new();
        
        checks.push(self.db_check.check().await);
        checks.push(self.vcs_check.check().await);
        checks.push(self.disk_check.check().await);
        
        let worst = checks.iter().map(|c| c.status).max();
        
        HealthStatus {
            status: worst.unwrap_or(HealthState::Healthy),
            checks,
            timestamp: Utc::now(),
        }
    }
}
```

---

## Variants

### Variant A: Custom Telemetry (CHOSEN)

```rust
// Own types, own processing
// Pro: Full control, no external deps
// Con: Reinventing wheels
```

**Chosen because:**
- Minimal dependencies
- Full control over format
- Can export to any backend

### Variant B: OpenTelemetry SDK

**Rejected because:**
- Heavy dependency
- Complex setup
- Over-engineered for hardline's needs

### Variant C: No Telemetry

**Rejected because:**
- Impossible to debug 600+ agent issues
- No performance visibility

---

## Invariants

### Span Invariants

```rust
/// INVARIANT: Ended span has end_time and duration_ms
fn assert_ended_span_has_times(span: &Span) {
    if span.end_time.is_some() {
        assert!(span.duration_ms.is_some());
        assert!(span.end_time > span.start_time);
    }
}

/// INVIANt: Error span has error status
fn assert_error_span_has_status(span: &Span) {
    if span.status == SpanStatus::Error {
        assert!(span.attributes.iter().any(|kv| kv.key == "error.message"));
    }
}

/// INVARIANT: Child span has parent trace
fn assert_child_span_has_parent_trace(child: &Span, parent: &Span) {
    if child.parent_span_id.is_some() {
        assert_eq!(child.trace_id, parent.trace_id);
    }
}
```

### Metrics Invariants

```rust
/// INVARIANT: Counter values are non-negative
fn assert_counter_non_negative(metrics: &[Metric]) {
    for metric in metrics {
        if let MetricValue::Counter { value } = metric.value {
            assert!(value >= 0.0);
        }
    }
}

/// INVARIANT: Histogram count matches values length
fn assert_histogram_count_consistent(metric: &Metric) {
    if let MetricValue::Histogram { values, count } = &metric.value {
        assert_eq!(values.len(), *count);
    }
}
```

### Health Check Invariants

```rust
/// INVARIANT: HealthState is derived from checks
fn assert_health_state_derived(status: &HealthStatus) {
    let worst = status.checks.iter().map(|c| c.status).max();
    assert_eq!(status.status, worst.unwrap_or(HealthState::Healthy));
}

/// INVARIANT: Latency is non-negative
fn assert_latency_non_negative(check: &HealthCheck) {
    if let Some(latency) = check.latency_ms {
        assert!(latency >= 0);
    }
}
```

---

## Consequences

### Positive

1. **Debugging** - Trace spans show exactly what happened
2. **Performance** - Histograms show latency percentiles
3. **Alerting** - Health checks enable proactive alerting
4. **Audit** - Logs provide audit trail

### Negative

1. **Overhead** - Tracing adds ~1ms per operation
2. **Storage** - Telemetry data needs storage
3. **Complexity** - More moving parts

### CLI Commands

```bash
hardline doctor check           # Run health checks
hardline doctor status          # Current health status
hardline trace list <trace-id> # Get trace by ID
hardline metrics export         # Export metrics
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/observability/tracer.rs` | Tracer, Span types |
| `crates/core/src/observability/metrics.rs` | Metrics collector |
| `crates/core/src/observability/logging.rs` | Structured logging |
| `crates/core/src/observability/health.rs` | Health checks |

---

## Related ADRs

- ADR-001: CLI Architecture (doctor command)
- ADR-002: Durable Workflow Execution (spans for operations)
- ADR-010: Agent Registry & Heartbeat (agent observability)
