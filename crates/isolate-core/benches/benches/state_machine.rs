#![allow(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Benchmark state machine transition performance.
//!
//! This benchmark ensures that state transitions are zero-cost abstractions.
//! State machines should compile down to simple enum swaps with minimal overhead.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use isolate_core::{
    beads::{Issue, IssueState},
    domain::aggregates::bead::Bead,
};

// ============================================================================
// FIXTURES
// ============================================================================

/// Create a test issue for benchmarks
fn test_issue() -> Issue {
    Issue::new("test-1", "Test Issue").expect("valid issue")
}

/// Create a test bead for benchmarks
fn test_bead() -> Bead {
    let id = isolate_core::domain::identifiers::BeadId::parse("test-bead-1").expect("valid id");
    Bead::new(id, "Test Bead", None::<String>).expect("bead created")
}

// All possible state transitions for IssueState
fn issue_state_transitions() -> Vec<(IssueState, IssueState)> {
    use chrono::Utc;

    vec![
        (IssueState::Open, IssueState::InProgress),
        (IssueState::Open, IssueState::Blocked),
        (IssueState::Open, IssueState::Deferred),
        (
            IssueState::Open,
            IssueState::Closed {
                closed_at: Utc::now(),
            },
        ),
        (IssueState::InProgress, IssueState::Open),
        (IssueState::InProgress, IssueState::Blocked),
        (IssueState::InProgress, IssueState::Deferred),
        (
            IssueState::InProgress,
            IssueState::Closed {
                closed_at: Utc::now(),
            },
        ),
        (IssueState::Blocked, IssueState::Open),
        (IssueState::Blocked, IssueState::InProgress),
        (IssueState::Blocked, IssueState::Deferred),
        (
            IssueState::Blocked,
            IssueState::Closed {
                closed_at: Utc::now(),
            },
        ),
        (IssueState::Deferred, IssueState::Open),
        (IssueState::Deferred, IssueState::InProgress),
        (IssueState::Deferred, IssueState::Blocked),
        (
            IssueState::Deferred,
            IssueState::Closed {
                closed_at: Utc::now(),
            },
        ),
    ]
}

// ============================================================================
// BENCHMARKS: IssueState::transition_to
// ============================================================================

fn bench_issue_state_transition(c: &mut Criterion) {
    let transitions = issue_state_transitions();

    let mut group = c.benchmark_group("issue_state_transition");

    for (idx, (from, to)) in transitions.into_iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("transition", idx),
            &(from, to),
            |b, (from, to)| {
                b.iter(|| {
                    let _result = black_box(from).transition_to(black_box(*to));
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Issue::transition_to
// ============================================================================

fn bench_issue_transition_to(c: &mut Criterion) {
    use chrono::Utc;

    let mut group = c.benchmark_group("issue_transition_to");

    // Common workflow transitions
    group.bench_function("open_to_in_progress", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            let _ = issue.transition_to(black_box(IssueState::InProgress));
        });
    });

    group.bench_function("in_progress_to_blocked", |b| {
        let mut issue = test_issue();
        issue.transition_to(IssueState::InProgress).expect("valid");
        b.iter(|| {
            let _ = issue.transition_to(black_box(IssueState::Blocked));
        });
    });

    group.bench_function("blocked_to_in_progress", |b| {
        let mut issue = test_issue();
        issue.transition_to(IssueState::Blocked).expect("valid");
        b.iter(|| {
            let _ = issue.transition_to(black_box(IssueState::InProgress));
        });
    });

    group.bench_function("close", |b| {
        let mut issue = test_issue();
        b.iter(|| {
            issue.close();
        });
    });

    group.bench_function("close_with_time", |b| {
        let mut issue = test_issue();
        let closed_at = Utc::now();
        b.iter(|| {
            issue.close_with_time(black_box(closed_at));
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Bead state transitions
// ============================================================================

fn bench_bead_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("bead_state_transitions");

    // Test all bead state transition methods
    group.bench_function("bead_open_to_in_progress", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).start();
        });
    });

    group.bench_function("bead_open_to_blocked", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).block();
        });
    });

    group.bench_function("bead_open_to_deferred", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).defer();
        });
    });

    group.bench_function("bead_close", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).close();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: State query methods
// ============================================================================

fn bench_state_query_methods(c: &mut Criterion) {
    use chrono::Utc;

    let mut group = c.benchmark_group("state_query_methods");

    // IssueState query methods
    group.bench_function("issue_state_is_active_open", |b| {
        let state = IssueState::Open;
        b.iter(|| {
            let _ = black_box(state).is_active();
        });
    });

    group.bench_function("issue_state_is_active_in_progress", |b| {
        let state = IssueState::InProgress;
        b.iter(|| {
            let _ = black_box(state).is_active();
        });
    });

    group.bench_function("issue_state_is_blocked", |b| {
        let state = IssueState::Blocked;
        b.iter(|| {
            let _ = black_box(state).is_blocked();
        });
    });

    group.bench_function("issue_state_is_closed", |b| {
        let state = IssueState::Closed {
            closed_at: Utc::now(),
        };
        b.iter(|| {
            let _ = black_box(state).is_closed();
        });
    });

    group.bench_function("issue_state_closed_at", |b| {
        let state = IssueState::Closed {
            closed_at: Utc::now(),
        };
        b.iter(|| {
            let _ = black_box(state).closed_at();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Aggregate query methods
// ============================================================================

fn bench_aggregate_query_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregate_query_methods");

    // Issue query methods
    group.bench_function("issue_is_active", |b| {
        let issue = test_issue();
        b.iter(|| {
            let _ = black_box(&issue).is_active();
        });
    });

    group.bench_function("issue_is_blocked", |b| {
        let mut issue = test_issue();
        issue.transition_to(IssueState::Blocked).expect("valid");
        b.iter(|| {
            let _ = black_box(&issue).is_blocked();
        });
    });

    group.bench_function("issue_is_closed", |b| {
        let mut issue = test_issue();
        issue.close();
        b.iter(|| {
            let _ = black_box(&issue).is_closed();
        });
    });

    group.bench_function("issue_has_parent", |b| {
        let issue = test_issue();
        b.iter(|| {
            let _ = black_box(&issue).has_parent();
        });
    });

    // Bead query methods
    group.bench_function("bead_is_open", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).is_open();
        });
    });

    group.bench_function("bead_is_in_progress", |b| {
        let bead = test_bead();
        let in_progress = bead.start().expect("valid");
        b.iter(|| {
            let _ = black_box(&in_progress).is_in_progress();
        });
    });

    group.bench_function("bead_validate_can_modify", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).validate_can_modify();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Common workflows
// ============================================================================

fn bench_common_workflows(c: &mut Criterion) {
    let mut group = c.benchmark_group("common_workflows");

    // Typical issue lifecycle
    group.bench_function("issue_lifecycle_open_to_closed", |b| {
        b.iter(|| {
            let mut issue = test_issue();
            issue.transition_to(IssueState::InProgress).expect("valid");
            issue.close();
            black_box(&issue);
        });
    });

    // Issue with blockers
    group.bench_function("issue_with_blockers", |b| {
        b.iter(|| {
            let mut issue = test_issue();
            issue
                .set_blocked_by(vec!["blocker-1".to_string(), "blocker-2".to_string()])
                .expect("valid");
            issue.transition_to(IssueState::Blocked).expect("valid");
            black_box(&issue);
        });
    });

    // Bead full lifecycle
    group.bench_function("bead_lifecycle", |b| {
        b.iter(|| {
            let bead = test_bead();
            let started = bead.start().expect("valid");
            let closed = started.close().expect("valid");
            black_box(&closed);
        });
    });

    // Multiple state transitions
    group.bench_function("issue_multiple_transitions", |b| {
        b.iter(|| {
            let mut issue = test_issue();
            issue.transition_to(IssueState::InProgress).expect("valid");
            issue.transition_to(IssueState::Blocked).expect("valid");
            issue.transition_to(IssueState::Deferred).expect("valid");
            issue.transition_to(IssueState::InProgress).expect("valid");
            issue.close();
            black_box(&issue);
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: State cloning overhead
// ============================================================================

fn bench_state_clone_overhead(c: &mut Criterion) {
    use chrono::Utc;

    let mut group = c.benchmark_group("state_clone_overhead");

    // IssueState is Copy (no clone overhead expected)
    group.bench_function("issue_state_copy_open", |b| {
        let state = IssueState::Open;
        b.iter(|| {
            let _ = black_box(state);
        });
    });

    group.bench_function("issue_state_copy_closed", |b| {
        let state = IssueState::Closed {
            closed_at: Utc::now(),
        };
        b.iter(|| {
            let _ = black_box(state);
        });
    });

    // Issue requires clone (should be cheap)
    group.bench_function("issue_clone", |b| {
        let issue = test_issue();
        b.iter(|| {
            let _ = black_box(&issue).clone();
        });
    });

    // Bead requires clone (should be cheap)
    group.bench_function("bead_clone", |b| {
        let bead = test_bead();
        b.iter(|| {
            let _ = black_box(&bead).clone();
        });
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_issue_state_transition,
    bench_issue_transition_to,
    bench_bead_state_transitions,
    bench_state_query_methods,
    bench_aggregate_query_methods,
    bench_common_workflows,
    bench_state_clone_overhead
);

criterion_main!(benches);
