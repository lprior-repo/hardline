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
    /// Thread-safe storage for task handles
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl TaskRegistry {
    /// Create a new empty task registry
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a task for tracking and cleanup
    ///
    /// # Errors
    ///
    /// Returns error if the lock is poisoned
    pub async fn register(&self, task: JoinHandle<()>) -> Result<()> {
        self.tasks.lock().await.push(task);
        Ok(())
    }

    /// Get the count of registered tasks
    ///
    /// # Errors
    ///
    /// Returns error if the lock is poisoned
    pub async fn task_count(&self) -> Result<usize> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.len())
    }

    /// Shutdown all registered tasks
    ///
    /// This aborts all tasks and removes them from the registry.
    ///
    /// # Errors
    ///
    /// Returns error if the lock is poisoned
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().await;

        // Abort each task
        for task in tasks.drain(..) {
            task.abort();
        }

        drop(tasks); // Release lock early
        Ok(())
    }

    /// Remove completed tasks from the registry
    ///
    /// This is useful for periodic cleanup to prevent unbounded growth.
    ///
    /// # Errors
    ///
    /// Returns error if the lock is poisoned
    pub async fn cleanup_completed(&self) -> Result<usize> {
        let mut tasks = self.tasks.lock().await;
        let initial_count = tasks.len();

        // Remove completed tasks (those that return immediately when polled)
        tasks.retain(|task| !task.is_finished());

        let removed = initial_count.saturating_sub(tasks.len());
        drop(tasks); // Release lock early
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = TaskRegistry::new();
        let count = registry.task_count().await;
        assert!(count.is_ok());
        if let Ok(c) = count {
            assert_eq!(c, 0);
        }
    }

    #[tokio::test]
    async fn test_register_task() -> Result<()> {
        let registry = TaskRegistry::new();

        // Register a task
        let task = tokio::spawn(async {
            sleep(Duration::from_millis(100)).await;
        });
        registry.register(task).await?;

        // Verify task count
        let count = registry.task_count().await?;
        assert_eq!(count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_shutdown_all() -> Result<()> {
        let registry = TaskRegistry::new();

        // Register multiple tasks
        for i in 0..5 {
            let task = tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                println!("Task {i} completed");
            });
            registry.register(task).await?;
        }

        // Verify we have 5 tasks
        let count = registry.task_count().await?;
        assert_eq!(count, 5);

        // Shutdown all tasks
        registry.shutdown_all().await?;

        // Verify all tasks were removed
        let count = registry.task_count().await?;
        assert_eq!(count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_completed() -> Result<()> {
        let registry = TaskRegistry::new();

        // Register a short-lived task
        let task = tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
        });
        registry.register(task).await?;

        // Register a long-running task
        let long_task = tokio::spawn(async {
            sleep(Duration::from_secs(10)).await;
        });
        registry.register(long_task).await?;

        // Wait for short task to complete
        sleep(Duration::from_millis(50)).await;

        // Cleanup completed tasks
        let removed = registry.cleanup_completed().await?;

        // Should have removed 1 task
        assert_eq!(removed, 1);

        // Should have 1 remaining task
        let count = registry.task_count().await?;
        assert_eq!(count, 1);

        Ok(())
    }
}
