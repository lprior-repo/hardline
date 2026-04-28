#![allow(
    clippy::single_match_else,
    clippy::redundant_closure,
    clippy::uninlined_format_args,
    clippy::expect_used,
    clippy::used_underscore_binding
)]

//! Benchmark identifier parsing performance.
//!
//! This benchmark measures the overhead of validating identifiers at boundaries.
//! Identifiers use the "parse-at-boundaries" pattern - validation happens once
//! during construction, ensuring the type cannot represent invalid states.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use isolate_core::domain::identifiers::{AgentId, SessionName, TaskId, WorkspaceName};

// ============================================================================
// FIXTURES
// ============================================================================

/// Valid session names of varying lengths
fn session_names() -> Vec<String> {
    vec![
        "a".to_string(),
        "ab".to_string(),
        "abc".to_string(),
        "session-name".to_string(),
        "my-session-123".to_string(),
        "session_name_with_underscores".to_string(),
        "very-long-session-name-with-many-parts".to_string(),
        "a".repeat(20),
        "session-with-numbers-12345".to_string(),
        "test-session-name-final".to_string(),
    ]
}

/// Valid agent IDs
fn agent_ids() -> Vec<String> {
    vec![
        "agent-1".to_string(),
        "agent-123".to_string(),
        "pid-12345".to_string(),
        "agent.example.com".to_string(),
        "agent:8080".to_string(),
        "agent-cluster-01".to_string(),
        "build-agent-linux".to_string(),
        "test-agent-runner".to_string(),
        "ci-agent-production".to_string(),
        "agent-with.dots:and.colons".to_string(),
    ]
}

/// Valid task/bead IDs (must start with "bd-")
fn task_ids() -> Vec<String> {
    vec![
        "bd-abc123".to_string(),
        "bd-ABC123DEF456".to_string(),
        "bd-1234567890abcdef".to_string(),
        "bd-a1b2c3d4e5f6".to_string(),
        "bd-0000000000000000".to_string(),
        "bd-ffffffffffffffff".to_string(),
        "bd-deadbeefcafe".to_string(),
        "bd-feeb1ed5cafe".to_string(),
    ]
}

/// Valid workspace names
fn workspace_names() -> Vec<String> {
    vec![
        "my-workspace".to_string(),
        "project-alpha".to_string(),
        "workspace_123".to_string(),
        "test-workspace-final".to_string(),
        "production-build".to_string(),
        "a".repeat(50),
        "workspace-with-multiple-dashes".to_string(),
        "workspace_with_underscores_too".to_string(),
    ]
}

// ============================================================================
// BENCHMARKS: SessionName
// ============================================================================

fn bench_session_name_parse(c: &mut Criterion) {
    let names = session_names();

    let mut group = c.benchmark_group("session_name_parse");

    for name in &names {
        group.bench_with_input(BenchmarkId::from_parameter(name.as_str()), name, |b, s| {
            b.iter(|| {
                match SessionName::parse(black_box(s)) {
                    Ok(_name) => {
                        // Successfully parsed - zero overhead after validation
                    }
                    Err(_) => {
                        // Should not happen with valid input
                    }
                }
            });
        });
    }

    group.finish();
}

fn bench_session_name_parse_invalid(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_name_parse_invalid");

    // Empty string
    group.bench_function("empty", |b| {
        b.iter(|| {
            let _ = SessionName::parse(black_box(""));
        });
    });

    // Whitespace only
    group.bench_function("whitespace_only", |b| {
        b.iter(|| {
            let _ = SessionName::parse(black_box("   "));
        });
    });

    // Starts with number
    group.bench_function("starts_with_number", |b| {
        b.iter(|| {
            let _ = SessionName::parse(black_box("123-session"));
        });
    });

    // Too long (>63 chars)
    let long_name = "a".repeat(64);
    group.bench_with_input("too_long_64", &long_name, |b, s| {
        b.iter(|| {
            let _ = SessionName::parse(black_box(s));
        });
    });

    // Contains special characters
    group.bench_function("has_special_chars", |b| {
        b.iter(|| {
            let _ = SessionName::parse(black_box("my.session"));
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: AgentId
// ============================================================================

fn bench_agent_id_parse(c: &mut Criterion) {
    let ids = agent_ids();

    let mut group = c.benchmark_group("agent_id_parse");

    for id in &ids {
        group.bench_with_input(BenchmarkId::from_parameter(id.as_str()), id, |b, s| {
            b.iter(|| {
                match AgentId::parse(black_box(s)) {
                    Ok(_id) => {
                        // Successfully parsed
                    }
                    Err(_) => {
                        // Should not happen with valid input
                    }
                }
            });
        });
    }

    group.finish();
}

fn bench_agent_id_from_process(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_id_from_process");

    group.bench_function("from_process", |b| {
        b.iter(|| {
            let _id = AgentId::from_process();
            black_box(&_id);
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: TaskId/BeadId
// ============================================================================

fn bench_task_id_parse(c: &mut Criterion) {
    let ids = task_ids();

    let mut group = c.benchmark_group("task_id_parse");

    for id in &ids {
        group.bench_with_input(BenchmarkId::from_parameter(id.as_str()), id, |b, s| {
            b.iter(|| {
                match TaskId::parse(black_box(s)) {
                    Ok(_id) => {
                        // Successfully parsed
                    }
                    Err(_) => {
                        // Should not happen with valid input
                    }
                }
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: WorkspaceName
// ============================================================================

fn bench_workspace_name_parse(c: &mut Criterion) {
    let names = workspace_names();

    let mut group = c.benchmark_group("workspace_name_parse");

    for (idx, name) in names.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("name", idx), name, |b, s| {
            b.iter(|| {
                match WorkspaceName::parse(black_box(s)) {
                    Ok(_name) => {
                        // Successfully parsed
                    }
                    Err(_) => {
                        // Should not happen with valid input
                    }
                }
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: QueueEntryId
// ============================================================================

// fn bench_queue_entry_id_new(c: &mut Criterion) {
// let mut group = c.benchmark_group("queue_entry_id_new");
//
// Valid IDs
// group.bench_function("valid_small", |b| {
// b.iter(|| {
// match QueueEntryId::new(black_box(42)) {
// Ok(_id) => {
// Successfully created
// }
// Err(_) => {
// Should not happen with valid input
// }
// }
// });
// });
//
// group.bench_function("valid_large", |b| {
// b.iter(|| {
// match QueueEntryId::new(black_box(i64::MAX)) {
// Ok(_id) => {
// Successfully created
// }
// Err(_) => {
// Should not happen with valid input
// }
// }
// });
// });
//
// Invalid IDs
// group.bench_function("invalid_zero", |b| {
// b.iter(|| {
// let _ = QueueEntryId::new(black_box(0));
// });
// });
//
// group.bench_function("invalid_negative", |b| {
// b.iter(|| {
// let _ = QueueEntryId::new(black_box(-1));
// });
// });
//
// group.finish();
// }

// ============================================================================
// BENCHMARKS: AsRef/Display overhead
// ============================================================================

fn bench_identifier_as_str_overhead(c: &mut Criterion) {
    let name = SessionName::parse("test-session").expect("valid");

    let mut group = c.benchmark_group("identifier_as_str_overhead");

    group.bench_function("session_name_as_str", |b| {
        b.iter(|| {
            let _s = black_box(&name).as_str();
        });
    });

    group.bench_function("session_name_to_string", |b| {
        b.iter(|| {
            let _s = black_box(&name).to_string();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Bulk parsing
// ============================================================================

fn bench_bulk_session_name_parsing(c: &mut Criterion) {
    let names: Vec<String> = (0..100).map(|i| format!("session-{:?}", i)).collect();

    let mut group = c.benchmark_group("bulk_parsing");

    group.bench_function("parse_100_session_names", |b| {
        b.iter(|| {
            let _parsed: Result<Vec<_>, _> = names.iter().map(|n| SessionName::parse(n)).collect();
        });
    });

    // Compare with iterator approach (more idiomatic)
    group.bench_function("parse_100_session_names_iter", |b| {
        b.iter(|| {
            let _parsed: Result<Vec<_>, _> = names.iter().map(|n| SessionName::parse(n)).collect();
        });
    });

    // Filter and collect (common pattern)
    group.bench_function("parse_filter_valid_session_names", |b| {
        b.iter(|| {
            let _valid: Vec<_> = names
                .iter()
                .filter_map(|n| SessionName::parse(n).ok())
                .collect();
        });
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_session_name_parse,
    bench_session_name_parse_invalid,
    bench_agent_id_parse,
    bench_agent_id_from_process,
    bench_task_id_parse,
    bench_workspace_name_parse,
    bench_identifier_as_str_overhead,
    bench_bulk_session_name_parsing
);

criterion_main!(benches);
