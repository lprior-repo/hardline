use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scp_core::lock::{LockManager, MemLockManager};
use scp_core::queue::{MemQueue, Priority, QueueItem, QueueManager, QueueSource};
use std::sync::Arc;

fn bench_queue_enqueue(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_enqueue");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
                let queue = MemQueue::new(lock);
                for i in 0..size {
                    let item = QueueItem {
                        id: format!("item-{}", i),
                        branch: format!("branch-{}", i),
                        source: QueueSource::Direct,
                        priority: Priority::Normal,
                        status: scp_core::queue::QueueStatus::Pending,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        attempt_count: 0,
                        last_error: None,
                    };
                    queue.enqueue(black_box(item)).ok();
                }
            });
        });
    }
    group.finish();
}

fn bench_queue_dequeue(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_dequeue");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
                let queue = MemQueue::new(lock);
                for i in 0..size {
                    let item = QueueItem {
                        id: format!("item-{}", i),
                        branch: format!("branch-{}", i),
                        source: QueueSource::Direct,
                        priority: Priority::Normal,
                        status: scp_core::queue::QueueStatus::Pending,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        attempt_count: 0,
                        last_error: None,
                    };
                    queue.enqueue(item).ok();
                }
                while queue.dequeue().ok().flatten().is_some() {}
            });
        });
    }
    group.finish();
}

fn bench_queue_list_pending(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_list_pending");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
                let queue = MemQueue::new(lock);
                for i in 0..size {
                    let mut item = QueueItem::direct(format!("branch-{}", i));
                    if i % 3 == 0 {
                        item.status = scp_core::queue::QueueStatus::Processing;
                    }
                    queue.enqueue(item).ok();
                }
                black_box(queue.list_pending()).ok();
            });
        });
    }
    group.finish();
}

fn bench_queue_priority_order(c: &mut Criterion) {
    c.bench_function("queue_priority_order", |b| {
        b.iter(|| {
            let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
            let queue = MemQueue::new(lock);

            for priority in [
                Priority::Low,
                Priority::Normal,
                Priority::High,
                Priority::Critical,
            ] {
                for i in 0..25 {
                    let mut item = QueueItem::direct(format!("branch-{}-{}", priority as u8, i));
                    item.priority = priority;
                    queue.enqueue(item).ok();
                }
            }
            black_box(queue.list_pending()).ok();
        });
    });
}

criterion_group!(
    benches,
    bench_queue_enqueue,
    bench_queue_dequeue,
    bench_queue_list_pending,
    bench_queue_priority_order
);
criterion_main!(benches);
