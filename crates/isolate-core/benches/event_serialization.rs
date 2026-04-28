#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(
    unused_imports,
    dead_code,
    clippy::expect_used,
    clippy::explicit_iter_loop,
    clippy::uninlined_format_args,
    clippy::needless_borrows_for_generic_args,
    clippy::redundant_closure
)]

//! Benchmark domain event serialization performance.
//!
//! This benchmark measures JSON serialization/deserialization overhead for
//! domain events. Events are used for event sourcing and audit logging,
//! so serialization performance is critical.

use std::path::PathBuf;

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use isolate_core::domain::{
    events::{serialize_event, serialize_event_bytes, DomainEvent},
    identifiers::{AgentId, BeadId, SessionName, WorkspaceName},
};

// ============================================================================
// FIXTURES
// ============================================================================

/// Create a session created event
fn session_created_event() -> DomainEvent {
    DomainEvent::session_created(
        "session-abc123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        Utc::now(),
    )
}

/// Create a session completed event
fn session_completed_event() -> DomainEvent {
    DomainEvent::session_completed(
        "session-abc123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        Utc::now(),
    )
}

/// Create a session failed event
fn session_failed_event() -> DomainEvent {
    DomainEvent::session_failed(
        "session-abc123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        "Out of memory".to_string(),
        Utc::now(),
    )
}

/// Create a workspace created event
fn workspace_created_event() -> DomainEvent {
    DomainEvent::workspace_created(
        WorkspaceName::parse("my-workspace").expect("valid"),
        PathBuf::from("/home/user/workspace"),
        Utc::now(),
    )
}

/// Create a queue entry added event
fn queue_entry_added_event() -> DomainEvent {
    session_created_event()
}

/// Create a queue entry claimed event
fn queue_entry_claimed_event() -> DomainEvent {
    session_created_event()
}

/// Create a bead created event
fn bead_created_event() -> DomainEvent {
    DomainEvent::bead_created(
        BeadId::parse("bd-abc123").expect("valid"),
        "Fix the bug".to_string(),
        Some("Critical issue in production".to_string()),
        Utc::now(),
    )
}

/// Create a bead closed event
fn bead_closed_event() -> DomainEvent {
    DomainEvent::bead_closed(
        BeadId::parse("bd-abc123").expect("valid"),
        Utc::now(),
        Utc::now(),
    )
}

/// Get all event types for benchmarking
fn all_events() -> Vec<(&'static str, DomainEvent)> {
    vec![
        ("session_created", session_created_event()),
        ("session_completed", session_completed_event()),
        ("session_failed", session_failed_event()),
        ("workspace_created", workspace_created_event()),
        ("bead_created", bead_created_event()),
        ("bead_closed", bead_closed_event()),
    ]
}

// ============================================================================
// BENCHMARKS: Serialization
// ============================================================================

fn bench_serialize_single_event(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("serialize_single_event");

    for (name, event) in events {
        // Measure JSON size for throughput
        let json = serialize_event(&event).expect("serialization failed");
        let bytes = json.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new(name, bytes), &event, |b, event| {
            b.iter(|| {
                let _json = serialize_event(black_box(event)).expect("serialization failed");
            });
        });
    }

    group.finish();
}

fn bench_serialize_single_event_bytes(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("serialize_single_event_bytes");

    for (name, event) in events {
        // Measure bytes size for throughput
        let bytes_data = serialize_event_bytes(&event).expect("serialization failed");
        let size = bytes_data.len();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new(name, size), &event, |b, event| {
            b.iter(|| {
                let _bytes = serialize_event_bytes(black_box(event)).expect("serialization failed");
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Deserialization
// ============================================================================

fn bench_deserialize_single_event(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("deserialize_single_event");

    for (name, event) in events {
        let json = serialize_event(&event).expect("serialization failed");
        let bytes = json.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new(name, bytes), &json, |b, json| {
            b.iter(|| {
                let _event: DomainEvent =
                    serde_json::from_str(black_box(json)).expect("deserialization failed");
            });
        });
    }

    group.finish();
}

fn bench_deserialize_single_event_bytes(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("deserialize_single_event_bytes");

    for (name, event) in events {
        let bytes_data = serialize_event_bytes(&event).expect("serialization failed");
        let size = bytes_data.len();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new(name, size), &bytes_data, |b, bytes| {
            b.iter(|| {
                let _event: DomainEvent =
                    serde_json::from_slice(black_box(bytes)).expect("deserialization failed");
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Round-trip
// ============================================================================

fn bench_roundtrip_single_event(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("roundtrip_single_event");

    for (name, event) in events {
        let json = serialize_event(&event).expect("serialization failed");
        let bytes = json.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new(name, bytes), &event, |b, event| {
            b.iter(|| {
                let json = serialize_event(black_box(event)).expect("serialization failed");
                let _deserialized: DomainEvent =
                    serde_json::from_str(&json).expect("deserialization failed");
            });
        });
    }

    group.finish();
}

fn bench_roundtrip_single_event_bytes(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("roundtrip_single_event_bytes");

    for (name, event) in events {
        let bytes_data = serialize_event_bytes(&event).expect("serialization failed");
        let size = bytes_data.len();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new(name, size), &event, |b, event| {
            b.iter(|| {
                let bytes = serialize_event_bytes(black_box(event)).expect("serialization failed");
                let _deserialized: DomainEvent =
                    serde_json::from_slice(&bytes).expect("deserialization failed");
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Bulk operations
// ============================================================================

fn bench_serialize_multiple_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_multiple_events");

    // Different batch sizes
    for size in &[10, 50, 100, 500, 1000] {
        let events: Vec<DomainEvent> = (0..*size)
            .map(|i| {
                DomainEvent::session_created(
                    format!("session-{i}"),
                    SessionName::parse(&format!("session-{i}")).expect("valid"),
                    Utc::now(),
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &events, |b, events| {
            b.iter(|| {
                let _results: Result<Vec<String>, _> =
                    events.iter().map(|e| serialize_event(e)).collect();
            });
        });
    }

    group.finish();
}

fn bench_deserialize_multiple_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_multiple_events");

    // Different batch sizes
    for size in &[10, 50, 100, 500, 1000] {
        let events: Vec<DomainEvent> = (0..*size)
            .map(|i| {
                DomainEvent::session_created(
                    format!("session-{i}"),
                    SessionName::parse(&format!("session-{i}")).expect("valid"),
                    Utc::now(),
                )
            })
            .collect();

        let json_batch: Result<Vec<String>, _> =
            events.iter().map(|e| serialize_event(e)).collect();
        let json_batch = json_batch.expect("serialization failed");

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &json_batch,
            |b, batch| {
                b.iter(|| {
                    let _results: Result<Vec<DomainEvent>, _> =
                        batch.iter().map(|j| serde_json::from_str(j)).collect();
                });
            },
        );
    }

    group.finish();
}

fn bench_roundtrip_multiple_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_multiple_events");

    // Different batch sizes
    for size in &[10, 50, 100, 500] {
        let events: Vec<DomainEvent> = (0..*size)
            .map(|i| {
                DomainEvent::session_created(
                    format!("session-{i}"),
                    SessionName::parse(&format!("session-{i}")).expect("valid"),
                    Utc::now(),
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &events, |b, events| {
            b.iter(|| {
                let json_results: Result<Vec<String>, _> =
                    events.iter().map(|e| serialize_event(e)).collect();
                let jsons = json_results.expect("serialization failed");

                let _event_results: Result<Vec<DomainEvent>, _> =
                    jsons.iter().map(|j| serde_json::from_str(j)).collect();
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Event creation overhead
// ============================================================================

fn bench_event_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_creation");

    group.bench_function("session_created", |b| {
        b.iter(|| {
            let _event = DomainEvent::session_created(
                black_box("session-123".to_string()),
                SessionName::parse("my-session").expect("valid"),
                Utc::now(),
            );
        });
    });

    group.bench_function("workspace_created", |b| {
        b.iter(|| {
            let _event = DomainEvent::workspace_created(
                WorkspaceName::parse("my-workspace").expect("valid"),
                PathBuf::from("/home/user/workspace"),
                Utc::now(),
            );
        });
    });

    group.bench_function("bead_created", |b| {
        b.iter(|| {
            let _event = DomainEvent::bead_created(
                BeadId::parse("bd-abc123").expect("valid"),
                "Fix the bug".to_string(),
                Some("Critical issue".to_string()),
                Utc::now(),
            );
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Event metadata extraction
// ============================================================================

fn bench_event_metadata_extraction(c: &mut Criterion) {
    let events = all_events();

    let mut group = c.benchmark_group("event_metadata_extraction");

    for (name, event) in events {
        group.bench_function(name, |b| {
            b.iter(|| {
                let _type = black_box(&event).event_type();
                let _timestamp = black_box(&event).timestamp();
            });
        });
    }

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_serialize_single_event,
    bench_serialize_single_event_bytes,
    bench_deserialize_single_event,
    bench_deserialize_single_event_bytes,
    bench_roundtrip_single_event,
    bench_roundtrip_single_event_bytes,
    bench_serialize_multiple_events,
    bench_deserialize_multiple_events,
    bench_roundtrip_multiple_events,
    bench_event_creation,
    bench_event_metadata_extraction
);

criterion_main!(benches);
