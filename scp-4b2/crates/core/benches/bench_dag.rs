use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scp_core::dag::{BranchDag, BranchId};

fn bench_dag_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_create");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut dag = BranchDag::new();
                for i in 0..size {
                    let id = BranchId::new(format!("branch-{}", i));
                    let parents = if i == 0 {
                        vec![BranchId::new("trunk".to_string())]
                    } else {
                        vec![BranchId::new(format!("branch-{}", i - 1))]
                    };
                    dag.add_branch(black_box(id), black_box(parents)).ok();
                }
            });
        });
    }
    group.finish();
}

fn bench_dag_add_branch(c: &mut Criterion) {
    c.bench_function("dag_add_branch", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();
            for i in 0..100 {
                let id = BranchId::new(format!("branch-{}", i));
                let parents = if i == 0 {
                    vec![BranchId::new("trunk".to_string())]
                } else {
                    vec![BranchId::new(format!("branch-{}", i - 1))]
                };
                dag.add_branch(black_box(id), black_box(parents)).ok();
            }
        });
    });
}

fn bench_dag_linear_chain(c: &mut Criterion) {
    c.bench_function("dag_linear_chain", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();
            for i in 0..100 {
                let id = BranchId::new(format!("b{}", i));
                let parent = if i == 0 {
                    BranchId::new("trunk".to_string())
                } else {
                    BranchId::new(format!("b{}", i - 1))
                };
                dag.add_branch(black_box(id), black_box(vec![parent])).ok();
            }
            black_box(&dag);
        });
    });
}

fn bench_dag_wide_branching(c: &mut Criterion) {
    c.bench_function("dag_wide_branching", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();
            // Branch from trunk 100 times
            for i in 0..100 {
                let id = BranchId::new(format!("feature-{}", i));
                let parent = BranchId::new("trunk".to_string());
                dag.add_branch(black_box(id), black_box(vec![parent])).ok();
            }
            black_box(&dag);
        });
    });
}

fn bench_dag_descendants(c: &mut Criterion) {
    c.bench_function("dag_descendants", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();

            // Create a tree structure
            for i in 0..50 {
                let id = BranchId::new(format!("branch-{}", i));
                let parent = if i == 0 {
                    BranchId::new("trunk".to_string())
                } else {
                    BranchId::new(format!("branch-{}", (i - 1) / 2))
                };
                dag.add_branch(black_box(id), black_box(vec![parent])).ok();
            }

            // Benchmark descendants query on trunk
            black_box(dag.descendants(&BranchId::new("trunk".to_string()))).ok();
        });
    });
}

fn bench_dag_ancestors(c: &mut Criterion) {
    c.bench_function("dag_ancestors", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();

            // Create a linear chain
            for i in 0..50 {
                let id = BranchId::new(format!("branch-{}", i));
                let parent = if i == 0 {
                    BranchId::new("trunk".to_string())
                } else {
                    BranchId::new(format!("branch-{}", i - 1))
                };
                dag.add_branch(black_box(id), black_box(vec![parent])).ok();
            }

            // Benchmark ancestors query on last branch
            black_box(dag.ancestors(&BranchId::new("branch-49".to_string()))).ok();
        });
    });
}

fn bench_dag_topological_sort(c: &mut Criterion) {
    c.bench_function("dag_topological_sort", |b| {
        b.iter(|| {
            let mut dag = BranchDag::new();

            // Create a more complex DAG
            for i in 0..50 {
                let id = BranchId::new(format!("branch-{}", i));
                let parents = if i == 0 {
                    vec![BranchId::new("trunk".to_string())]
                } else if i < 10 {
                    vec![BranchId::new("trunk".to_string())]
                } else {
                    vec![
                        BranchId::new(format!("branch-{}", (i - 1) * 2 / 3)),
                        BranchId::new(format!("branch-{}", i - 1)),
                    ]
                };
                dag.add_branch(black_box(id), black_box(parents)).ok();
            }

            black_box(dag.topological_order()).ok();
        });
    });
}

criterion_group!(
    benches,
    bench_dag_create,
    bench_dag_add_branch,
    bench_dag_linear_chain,
    bench_dag_wide_branching,
    bench_dag_descendants,
    bench_dag_ancestors,
    bench_dag_topological_sort
);
criterion_main!(benches);
