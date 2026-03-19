mod bench_dag;
mod bench_queue;
mod bench_types;

fn main() {
    criterion::Criterion::default().configure_from_args().run();
}
