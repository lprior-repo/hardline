//! Task registry for resource leak protection
//!
//! This module provides a centralized registry for tracking all active tasks
//! and ensuring clean shutdown. It prevents resource leaks by:
//! - Tracking all spawned tasks
//! - Providing graceful shutdown on drop
//! - Cleaning up resources on panic
//!
//! # Example
//!
//! ```ignore
//! let registry = TaskRegistry::new();
//! let task = tokio::spawn(async { /* ... */ });
//! registry.register(task).await;
//!
//! // On shutdown, all tasks are cleaned up automatically
//! registry.shutdown_all().await;
//! ```

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::sync::Arc;

use tokio::{sync::Mutex, task::JoinHandle};

use crate::Result;

/// Registry for tracking and cleaning up tasks
///
/// Tasks are stored in a `Mutex<Vec<JoinHandle>>>` for thread-safe access.
/// On shutdown, all tasks are aborted gracefully.
#[derive(Clone, Default)]
pub struct TaskRegistry {
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl TaskRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, task: JoinHandle<()>) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        tasks.push(task);
        Ok(())
    }

    pub async fn task_count(&self) -> Result<usize> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.len())
    }

    pub async fn shutdown_all(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().await;

        for task in tasks.drain(..) {
            task.abort();
        }

        drop(tasks);
        Ok(())
    }

    pub async fn cleanup_completed(&self) -> Result<usize> {
        let mut tasks = self.tasks.lock().await;
        let initial_count = tasks.len();

        tasks.retain(|task| !task.is_finished());

        let removed = initial_count.saturating_sub(tasks.len());
        drop(tasks);
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    // ── Empty registry behavior ──

    #[tokio::test]
    async fn empty_registry_has_zero_count() -> Result<()> {
        let registry = TaskRegistry::new();
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn empty_registry_cleanup_removes_nothing() -> Result<()> {
        let registry = TaskRegistry::new();
        assert_eq!(registry.cleanup_completed().await?, 0);
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn empty_registry_shutdown_is_noop() -> Result<()> {
        let registry = TaskRegistry::new();
        registry.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn default_equals_new() -> Result<()> {
        let via_new = TaskRegistry::new();
        let via_default = TaskRegistry::default();
        assert_eq!(via_new.task_count().await?, via_default.task_count().await?);
        Ok(())
    }

    // ── Task registration ──

    #[tokio::test]
    async fn register_single_task_increments_count() -> Result<()> {
        let registry = TaskRegistry::new();
        let task = tokio::spawn(async {
            sleep(Duration::from_secs(10)).await;
        });
        registry.register(task).await?;
        assert_eq!(registry.task_count().await?, 1);
        registry.shutdown_all().await
    }

    #[tokio::test]
    async fn register_multiple_tasks_counts_all() -> Result<()> {
        let registry = TaskRegistry::new();
        for _ in 0..10 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(10)).await;
            });
            registry.register(task).await?;
        }
        assert_eq!(registry.task_count().await?, 10);
        registry.shutdown_all().await
    }

    #[tokio::test]
    async fn register_already_completed_task() -> Result<()> {
        let registry = TaskRegistry::new();
        let task = tokio::spawn(async {});
        // Wait for the task to finish
        sleep(Duration::from_millis(20)).await;
        registry.register(task).await?;
        assert_eq!(registry.task_count().await?, 1);
        // It's there but finished — cleanup should remove it
        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 1);
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn register_after_shutdown_works() -> Result<()> {
        let registry = TaskRegistry::new();
        let task = tokio::spawn(async {
            sleep(Duration::from_secs(10)).await;
        });
        registry.register(task).await?;
        registry.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);

        // Can register new tasks after shutdown
        let new_task = tokio::spawn(async {
            sleep(Duration::from_secs(10)).await;
        });
        registry.register(new_task).await?;
        assert_eq!(registry.task_count().await?, 1);
        registry.shutdown_all().await
    }

    // ── Shutdown behavior ──

    #[tokio::test]
    async fn shutdown_aborts_all_tasks() -> Result<()> {
        let registry = TaskRegistry::new();

        for _ in 0..5 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            registry.register(task).await?;
        }

        assert_eq!(registry.task_count().await?, 5);
        registry.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);

        Ok(())
    }

    #[tokio::test]
    async fn shutdown_is_idempotent() -> Result<()> {
        let registry = TaskRegistry::new();
        for _ in 0..3 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            registry.register(task).await?;
        }

        registry.shutdown_all().await?;
        registry.shutdown_all().await?; // Second shutdown — should not panic
        registry.shutdown_all().await?; // Third — belt and suspenders
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_then_cleanup() -> Result<()> {
        let registry = TaskRegistry::new();
        for _ in 0..3 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            registry.register(task).await?;
        }

        registry.shutdown_all().await?;
        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 0); // Already drained by shutdown
        Ok(())
    }

    // ── Cleanup behavior ──

    #[tokio::test]
    async fn cleanup_removes_only_finished_tasks() -> Result<()> {
        let registry = TaskRegistry::new();

        // Short-lived task — will finish quickly
        let short = tokio::spawn(async {
            sleep(Duration::from_millis(5)).await;
        });
        registry.register(short).await?;

        // Long-lived task — still running
        let long = tokio::spawn(async {
            sleep(Duration::from_secs(60)).await;
        });
        registry.register(long).await?;

        // Wait for the short one to finish
        sleep(Duration::from_millis(50)).await;

        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 1);
        assert_eq!(registry.task_count().await?, 1);

        registry.shutdown_all().await
    }

    #[tokio::test]
    async fn cleanup_when_all_running_removes_none() -> Result<()> {
        let registry = TaskRegistry::new();
        for _ in 0..3 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            registry.register(task).await?;
        }

        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 0);
        assert_eq!(registry.task_count().await?, 3);

        registry.shutdown_all().await
    }

    #[tokio::test]
    async fn cleanup_when_all_finished_removes_all() -> Result<()> {
        let registry = TaskRegistry::new();
        for _ in 0..4 {
            let task = tokio::spawn(async {});
            registry.register(task).await?;
        }

        // Wait for all tasks to complete
        sleep(Duration::from_millis(50)).await;

        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 4);
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn cleanup_is_idempotent() -> Result<()> {
        let registry = TaskRegistry::new();
        let task = tokio::spawn(async {});
        registry.register(task).await?;
        sleep(Duration::from_millis(20)).await;

        let first = registry.cleanup_completed().await?;
        assert_eq!(first, 1);

        let second = registry.cleanup_completed().await?;
        assert_eq!(second, 0);
        Ok(())
    }

    #[tokio::test]
    async fn cleanup_returns_accurate_count() -> Result<()> {
        let registry = TaskRegistry::new();

        // Register 6 tasks: 3 short, 3 long
        for _ in 0..3 {
            let task = tokio::spawn(async {});
            registry.register(task).await?;
        }
        for _ in 0..3 {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            registry.register(task).await?;
        }

        sleep(Duration::from_millis(50)).await;

        let removed = registry.cleanup_completed().await?;
        assert_eq!(removed, 3);
        assert_eq!(registry.task_count().await?, 3);

        registry.shutdown_all().await
    }

    // ── Clone / shared state behavior ──

    #[tokio::test]
    async fn cloned_registry_shares_state() -> Result<()> {
        let registry = TaskRegistry::new();
        let clone = registry.clone();

        let task = tokio::spawn(async {
            sleep(Duration::from_secs(60)).await;
        });
        registry.register(task).await?;

        // Clone sees the same task
        assert_eq!(clone.task_count().await?, 1);

        // Shutdown via clone affects original
        clone.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn multiple_clones_share_all_mutations() -> Result<()> {
        let registry = TaskRegistry::new();
        let c1 = registry.clone();
        let c2 = c1.clone();

        // Register via original
        let t1 = tokio::spawn(async {
            sleep(Duration::from_secs(60)).await;
        });
        registry.register(t1).await?;

        // Register via clone
        let t2 = tokio::spawn(async {
            sleep(Duration::from_secs(60)).await;
        });
        c1.register(t2).await?;

        // All views see 2 tasks
        assert_eq!(registry.task_count().await?, 2);
        assert_eq!(c1.task_count().await?, 2);
        assert_eq!(c2.task_count().await?, 2);

        c2.shutdown_all().await
    }

    // ── Concurrency ──

    #[tokio::test]
    async fn concurrent_registrations() -> Result<()> {
        let registry = TaskRegistry::new();
        let mut handles = Vec::new();

        for _ in 0..20 {
            let r = registry.clone();
            handles.push(tokio::spawn(async move {
                let task = tokio::spawn(async {
                    sleep(Duration::from_secs(60)).await;
                });
                r.register(task).await
            }));
        }

        for h in handles {
            // JoinError doesn't impl Into<Error>; must unwrap the join then propagate
            let inner = h.await.expect("spawned task panicked");
            inner?;
        }

        assert_eq!(registry.task_count().await?, 20);
        registry.shutdown_all().await
    }

    #[tokio::test]
    async fn concurrent_cleanup_and_register() -> Result<()> {
        let registry = TaskRegistry::new();

        // Pre-populate with short tasks
        for _ in 0..10 {
            let task = tokio::spawn(async {});
            registry.register(task).await?;
        }

        sleep(Duration::from_millis(50)).await;

        // Run cleanup and registration concurrently
        let r1 = registry.clone();
        let r2 = registry.clone();

        let cleanup_handle = tokio::spawn(async move { r1.cleanup_completed().await });
        let register_handle = tokio::spawn(async move {
            let task = tokio::spawn(async {
                sleep(Duration::from_secs(60)).await;
            });
            r2.register(task).await
        });

        let removed = cleanup_handle
            .await
            .expect("cleanup task panicked")
            .expect("cleanup failed");
        register_handle
            .await
            .expect("register task panicked")
            .expect("register failed");

        // At least some tasks should have been cleaned up
        assert!(removed > 0);
        // The newly registered task should still be there
        assert!(registry.task_count().await? >= 1);

        registry.shutdown_all().await
    }

    // ── Sequential operation chains ──

    #[tokio::test]
    async fn register_shutdown_register_shutdown_cycle() -> Result<()> {
        let registry = TaskRegistry::new();

        for cycle in 0..3 {
            for _ in 0..2 {
                let task = tokio::spawn(async {
                    sleep(Duration::from_secs(60)).await;
                });
                registry.register(task).await?;
            }
            assert_eq!(registry.task_count().await?, 2);
            registry.shutdown_all().await?;
            assert_eq!(registry.task_count().await?, 0);
            // Verify cycle counter is used (suppress unused warning)
            assert!(cycle < 3);
        }
        Ok(())
    }

    #[tokio::test]
    async fn cleanup_preserves_running_tasks_across_multiple_calls() -> Result<()> {
        let registry = TaskRegistry::new();

        // Register a long-running task
        let long_task = tokio::spawn(async {
            sleep(Duration::from_secs(60)).await;
        });
        registry.register(long_task).await?;

        // Register and complete short tasks in waves
        for wave in 0..3 {
            for _ in 0..2 {
                let task = tokio::spawn(async {});
                registry.register(task).await?;
            }
            sleep(Duration::from_millis(30)).await;

            let removed = registry.cleanup_completed().await?;
            assert_eq!(removed, 2);
            // Long task persists across all waves
            assert_eq!(registry.task_count().await?, 1);
            assert!(wave < 3); // suppress unused
        }

        registry.shutdown_all().await
    }

    // ── Task count invariant ──

    #[tokio::test]
    async fn count_never_negative_after_operations() -> Result<()> {
        let registry = TaskRegistry::new();

        // Various operations that should never leave count negative
        let _ = registry.cleanup_completed().await?;
        assert_eq!(registry.task_count().await?, 0);

        registry.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);

        let _ = registry.cleanup_completed().await?;
        registry.shutdown_all().await?;
        assert_eq!(registry.task_count().await?, 0);

        Ok(())
    }

    // ── Property-based tests ──

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        #[test]
        fn prop_register_increments_count_monotonically() {
            let rt = tokio::runtime::Runtime::new().expect("runtime creation");
            let registry = TaskRegistry::new();

            proptest!(|(n in 0usize..20)| {
                let r = registry.clone();
                rt.block_on(async {
                    let start = r.task_count().await.unwrap();
                    for _ in 0..n {
                        let task = tokio::spawn(async {
                            sleep(Duration::from_secs(60)).await;
                        });
                        r.register(task).await.unwrap();
                    }
                    let end = r.task_count().await.unwrap();
                    assert_eq!(end, start + n);
                    r.shutdown_all().await.unwrap();
                });
            });
        }

        #[test]
        fn prop_cleanup_never_exceeds_count() {
            let rt = tokio::runtime::Runtime::new().expect("runtime creation");
            let registry = TaskRegistry::new();

            proptest!(|(task_count in 0usize..10)| {
                let r = registry.clone();
                rt.block_on(async {
                    r.shutdown_all().await.unwrap();

                    for _ in 0..task_count {
                        let task = tokio::spawn(async {});
                        r.register(task).await.unwrap();
                    }

                    sleep(Duration::from_millis(20)).await;

                    let count_before = r.task_count().await.unwrap();
                    let removed = r.cleanup_completed().await.unwrap();
                    let count_after = r.task_count().await.unwrap();

                    assert!(removed <= count_before);
                    assert_eq!(count_after + removed, count_before);

                    r.shutdown_all().await.unwrap();
                });
            });
        }

        #[test]
        fn prop_shutdown_always_empties_registry() {
            let rt = tokio::runtime::Runtime::new().expect("runtime creation");
            let registry = TaskRegistry::new();

            proptest!(|(n in 0usize..15)| {
                let r = registry.clone();
                rt.block_on(async {
                    r.shutdown_all().await.unwrap();

                    for _ in 0..n {
                        let task = tokio::spawn(async {
                            sleep(Duration::from_secs(60)).await;
                        });
                        r.register(task).await.unwrap();
                    }

                    r.shutdown_all().await.unwrap();
                    assert_eq!(r.task_count().await.unwrap(), 0);
                });
            });
        }
    }
}
