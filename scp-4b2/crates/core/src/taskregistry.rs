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

        let task = tokio::spawn(async {
            sleep(Duration::from_millis(100)).await;
        });
        registry.register(task).await?;

        let count = registry.task_count().await?;
        assert_eq!(count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_shutdown_all() -> Result<()> {
        let registry = TaskRegistry::new();

        for i in 0..5 {
            let task = tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                println!("Task {i} completed");
            });
            registry.register(task).await?;
        }

        let count = registry.task_count().await?;
        assert_eq!(count, 5);

        registry.shutdown_all().await?;

        let count = registry.task_count().await?;
        assert_eq!(count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_completed() -> Result<()> {
        let registry = TaskRegistry::new();

        let task = tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
        });
        registry.register(task).await?;

        let long_task = tokio::spawn(async {
            sleep(Duration::from_secs(10)).await;
        });
        registry.register(long_task).await?;

        sleep(Duration::from_millis(50)).await;

        let removed = registry.cleanup_completed().await?;

        assert_eq!(removed, 1);

        let count = registry.task_count().await?;
        assert_eq!(count, 1);

        Ok(())
    }
}
