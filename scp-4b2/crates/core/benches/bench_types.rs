use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scp_core::types::{SessionId, SessionName};

fn bench_session_name_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_name_parse");

    let valid_names = [
        "feature-branch",
        "session-123",
        "my_awesome_feature",
        "a",
        "a".repeat(64).as_str(),
    ];

    for name in valid_names.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(name.len()), name, |b, &name| {
            b.iter(|| black_box(SessionName::parse(name)).ok());
        });
    }

    group.finish();
}

fn bench_session_name_invalid(c: &mut Criterion) {
    let invalid_names = [
        "",
        "123-start-with-number",
        "has@invalid#chars!",
        "a b c",
        "a".repeat(65).as_str(),
    ];

    c.bench_function("session_name_invalid", |b| {
        b.iter(|| {
            for name in invalid_names.iter() {
                black_box(SessionName::parse(name)).ok();
            }
        });
    });
}

fn bench_session_id_parse(c: &mut Criterion) {
    let valid_ids = ["abc123", "session-uuid-1234", "a", "abc-def-ghi-jkl"];

    c.bench_function("session_id_parse", |b| {
        b.iter(|| {
            for id in valid_ids.iter() {
                black_box(SessionId::parse(id)).ok();
            }
        });
    });
}

fn bench_session_id_invalid(c: &mut Criterion) {
    let invalid_ids = ["", "has spaces", "special@chars!", "has_underscore"];

    c.bench_function("session_id_invalid", |b| {
        b.iter(|| {
            for id in invalid_ids.iter() {
                black_box(SessionId::parse(id)).ok();
            }
        });
    });
}

fn bench_session_name_display(c: &mut Criterion) {
    let names: Vec<SessionName> = (0..100)
        .map(|i| SessionName::parse(format!("session-{}", i)).unwrap())
        .collect();

    c.bench_function("session_name_display", |b| {
        b.iter(|| {
            for name in names.iter() {
                black_box(format!("{}", name));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_session_name_parse,
    bench_session_name_invalid,
    bench_session_id_parse,
    bench_session_id_invalid,
    bench_session_name_display
);
criterion_main!(benches);
