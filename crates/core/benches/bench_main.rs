mod bench_dag;
mod bench_queue;
mod bench_types;

criterion_main!(
    bench_dag::benches,
    bench_queue::benches,
    bench_types::benches
);
