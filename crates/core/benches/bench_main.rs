mod bench_queue;
mod bench_types;
mod bench_dag;

fn main() {
    criterion::Criterion::default()
        .configure_from_args()
        .run();
}
