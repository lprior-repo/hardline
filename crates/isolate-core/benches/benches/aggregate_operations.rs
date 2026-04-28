#![allow(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Benchmark aggregate root operations.
//!
//! This benchmark measures the performance of aggregate operations including:
//! - Field updates with validation
//! - Builder pattern overhead
//! - Clone/copy operations
//! - Query method performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use isolate_core::{
    beads::{Assignee, Description, Issue, IssueBuilder, IssueId, Labels, Priority, Title},
    domain::{aggregates::bead::Bead, identifiers::BeadId},
};
use itertools::Itertools;

// ============================================================================
// FIXTURES
// ============================================================================

/// Create a test issue for benchmarks
fn test_issue() -> Issue {
    Issue::new("test-1", "Test Issue").expect("valid issue")
}

/// Create a test issue with full data
fn test_issue_full() -> Issue {
    let mut issue = Issue::new("test-1", "Test Issue with Description").expect("valid issue");
    issue
        .update_description("This is a detailed description of the issue".to_string())
        .expect("valid");
    issue.set_priority(Priority::P1);
    issue
        .set_labels(vec!["bug".to_string(), "critical".to_string()])
        .expect("valid");
    issue
        .set_assignee("user@example.com".to_string())
        .expect("valid");
    issue
}

/// Create a test bead for benchmarks
fn test_bead() -> Bead {
    let id = BeadId::parse("test-bead-1").expect("valid id");
    Bead::new(id, "Test Bead", None::<String>).expect("bead created")
}

// ============================================================================
// BENCHMARKS: Issue construction
// ============================================================================

fn bench_issue_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue_new");

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let _issue = Issue::new(black_box("test-1"), black_box("Test Issue"));
        });
    });

    group.bench_function("with_description", |b| {
        b.iter(|| {
            let mut issue =
                Issue::new(black_box("test-1"), black_box("Test Issue")).expect("valid");
            issue.update_description("A description".to_string()).ok();
        });
    });

    group.bench_function("full", |b| {
        b.iter(|| {
            let mut issue =
                Issue::new(black_box("test-1"), black_box("Test Issue")).expect("valid");
            issue.update_description("A description".to_string()).ok();
            issue.set_priority(Priority::P1);
            issue.set_labels(vec!["bug".to_string()]).ok();
            issue.set_assignee("user@example.com".to_string()).ok();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: IssueBuilder pattern
// ============================================================================

fn bench_issue_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue_builder");

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let _issue = IssueBuilder::new()
                .id(black_box("test-1"))
                .title(black_box("Test Issue"))
                .build();
        });
    });

    group.bench_function("with_fields", |b| {
        b.iter(|| {
            let _issue = IssueBuilder::new()
                .id(black_box("test-1"))
                .title(black_box("Test Issue"))
                .description(black_box("A description"))
                .priority(black_box(Priority::P1))
                .labels(vec!["bug".to_string()])
                .build();
        });
    });

    group.bench_function("full", |b| {
        b.iter(|| {
            let _issue = IssueBuilder::new()
                .id(black_box("test-1"))
                .title(black_box("Test Issue"))
                .description(black_box("A detailed description"))
                .priority(black_box(Priority::P1))
                .labels(vec!["bug".to_string(), "critical".to_string()])
                .assignee(black_box("user@example.com"))
                .depends_on(vec!["dep-1".to_string()])
                .build();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Field updates
// ============================================================================

fn bench_issue_field_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue_field_updates");

    // Title update
    group.bench_function("update_title", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue.update_title(black_box("Updated Title")).ok();
        });
    });

    // Description update
    group.bench_function("update_description", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue
                .update_description(black_box("Updated description"))
                .ok();
        });
    });

    // Priority set
    group.bench_function("set_priority", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue.set_priority(black_box(Priority::P1));
        });
    });

    // Labels add
    group.bench_function("add_label", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue.add_label(black_box("bug".to_string())).ok();
        });
    });

    // Assignee set
    group.bench_function("set_assignee", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue.set_assignee(black_box("user@example.com")).ok();
        });
    });

    // Dependencies set
    group.bench_function("set_depends_on", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue
                .set_depends_on(black_box(vec!["dep-1".to_string(), "dep-2".to_string()]))
                .ok();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Query methods
// ============================================================================

fn bench_issue_query_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue_query_methods");

    let issue = test_issue_full();

    group.bench_function("is_active", |b| {
        b.iter(|| {
            let _ = black_box(&issue).is_active();
        });
    });

    group.bench_function("is_blocked", |b| {
        b.iter(|| {
            let _ = black_box(&issue).is_blocked();
        });
    });

    group.bench_function("is_closed", |b| {
        b.iter(|| {
            let _ = black_box(&issue).is_closed();
        });
    });

    group.bench_function("has_parent", |b| {
        b.iter(|| {
            let _ = black_box(&issue).has_parent();
        });
    });

    group.bench_function("closed_at", |b| {
        b.iter(|| {
            let _ = black_box(&issue).closed_at();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Clone operations
// ============================================================================

fn bench_aggregate_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregate_clone");

    group.bench_function("issue_minimal", |b| {
        let issue = test_issue();
        b.iter(|| {
            let _ = black_box(&issue).clone();
        });
    });

    group.bench_function("issue_full", |b| {
        let issue = test_issue_full();
        b.iter(|| {
            let _ = black_box(&issue).clone();
        });
    });

    group.bench_function("bead", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).clone();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Bead operations
// ============================================================================

fn bench_bead_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bead_operations");

    group.bench_function("new", |b| {
        let id = BeadId::parse("test-bead").expect("valid");
        b.iter(|| {
            let _bead = Bead::new(
                black_box(id.clone()),
                black_box("Test Bead"),
                None::<String>,
            );
        });
    });

    group.bench_function("update_title", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = bead.update_title(black_box("Updated Title"));
        });
    });

    group.bench_function("update_description", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = bead.update_description(Some(black_box("New description")));
        });
    });

    group.bench_function("start", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = bead.start();
        });
    });

    group.bench_function("close", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = bead.close();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Bulk operations
// ============================================================================

fn bench_bulk_issue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_issue_operations");

    for count in &[10, 50, 100, 500] {
        // Create multiple issues
        group.bench_with_input(BenchmarkId::new("create", count), count, |b, count| {
            b.iter(|| {
                let _issues: Result<Vec<_>, _> = (0..*count)
                    .map(|i| Issue::new(format!("test-{i}"), format!("Issue {i}")))
                    .collect();
            });
        });

        // Update multiple issues
        group.bench_with_input(
            BenchmarkId::new("update_title", count),
            count,
            |b, count| {
                let issues: Result<Vec<_>, _> = (0..*count)
                    .map(|i| Issue::new(format!("test-{i}"), format!("Issue {i}")))
                    .collect();
                let mut issues = issues.expect("valid issues");

                b.iter(|| {
                    for issue in &mut issues {
                        issue.update_title("Updated").ok();
                    }
                });
            },
        );

        // Filter issues
        group.bench_with_input(
            BenchmarkId::new("filter_active", count),
            count,
            |b, count| {
                let issues: Result<Vec<_>, _> = (0..*count)
                    .map(|i| Issue::new(format!("test-{i}"), format!("Issue {i}")))
                    .collect();
                let issues = issues.expect("valid issues");

                b.iter(|| {
                    let _active: Vec<_> = issues.iter().filter(|i| i.is_active()).collect();
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Validation overhead
// ============================================================================

fn bench_validation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_overhead");

    // Valid input (should succeed)
    group.bench_function("title_valid", |b| {
        b.iter(|| {
            let _title = Title::new(black_box("Valid Title"));
        });
    });

    // Invalid input (should fail - more expensive due to error construction)
    group.bench_function("title_empty", |b| {
        b.iter(|| {
            let _title = Title::new(black_box(""));
        });
    });

    group.bench_function("description_valid", |b| {
        b.iter(|| {
            let _desc = Description::new(black_box("Valid description"));
        });
    });

    group.bench_function("issue_id_valid", |b| {
        b.iter(|| {
            let _id = IssueId::new(black_box("valid-id-123"));
        });
    });

    group.bench_function("issue_id_invalid", |b| {
        b.iter(|| {
            let _id = IssueId::new(black_box(""));
        });
    });

    group.bench_function("assignee_valid", |b| {
        b.iter(|| {
            let _assignee = Assignee::new(black_box("user@example.com"));
        });
    });

    group.bench_function("labels_valid", |b| {
        b.iter(|| {
            let _labels = Labels::new(vec!["bug".to_string(), "critical".to_string()]);
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Iterator operations
// ============================================================================

fn bench_aggregate_iterators(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregate_iterators");

    let issues: Vec<Issue> = (0..100)
        .map(|i| Issue::new(format!("test-{i}"), format!("Issue {i}")).expect("valid"))
        .collect();

    group.bench_function("filter_active", |b| {
        b.iter(|| {
            let _active: Vec<_> = black_box(&issues)
                .iter()
                .filter(|i| i.is_active())
                .collect();
        });
    });

    group.bench_function("filter_map_ids", |b| {
        b.iter(|| {
            let _ids: Vec<_> = black_box(&issues)
                .iter()
                .filter(|i| i.is_active())
                .map(|i| i.id.as_str().to_string())
                .collect();
        });
    });

    group.bench_function("map_titles", |b| {
        b.iter(|| {
            let _titles: Vec<_> = black_box(&issues)
                .iter()
                .map(|i| i.title.as_str().to_string())
                .collect();
        });
    });

    group.bench_function("chunk_and_process", |b| {
        b.iter(|| {
            let _chunks: Vec<_> = black_box(&issues)
                .iter()
                .chunks(10)
                .into_iter()
                .map(|chunk| chunk.filter(|i| i.is_active()).count())
                .collect();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Comparison with raw structs
// ============================================================================

fn bench_aggregate_vs_raw(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregate_vs_raw");

    // Aggregate approach (with validation)
    group.bench_function("aggregate_with_validation", |b| {
        b.iter(|| {
            let _issue = Issue::new(black_box("test-1"), black_box("Test Issue"));
        });
    });

    // Raw tuple approach (no validation, for comparison)
    group.bench_function("raw_tuple_no_validation", |b| {
        b.iter(|| {
            let _issue = (black_box("test-1"), black_box("Test Issue"));
        });
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_issue_new,
    bench_issue_builder,
    bench_issue_field_updates,
    bench_issue_query_methods,
    bench_aggregate_clone,
    bench_bead_operations,
    bench_bulk_issue_operations,
    bench_validation_overhead,
    bench_aggregate_iterators,
    bench_aggregate_vs_raw
);

criterion_main!(benches);
