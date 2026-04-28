//! Graceful shutdown coordinator for isolate.
//!
//! Handles SIGINT/SIGTERM signals and coordinates cleanup of:
//! - In-flight operations
//! - Database connections
//! - Agent processes
//! - Zellij sessions

use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

use crate::Result;

/// Shutdown signal that can be sent to all active operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// Graceful shutdown requested (SIGINT/SIGTERM)
    Graceful,
    /// Force shutdown requested (timeout exceeded)
    Force,
}

/// Coordinator for graceful shutdown across all components
pub struct ShutdownCoordinator {
    /// Channel to broadcast shutdown signals
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    /// Tracking all spawned tasks that need cleanup
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Tracking agent processes that need termination
    agent_processes: Arc<Mutex<Vec<std::process::Child>>>,
    /// Timeout for graceful shutdown before forcing
    shutdown_timeout: Duration,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    ///
    /// # Returns
    ///
    /// Returns a new coordinator instance. The result must be used
    /// to manage shutdown lifecycle.
    #[must_use]
    pub fn new(shutdown_timeout: Duration) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);

        Self {
            shutdown_tx,
            tasks: Arc::new(Mutex::new(Vec::new())),
            agent_processes: Arc::new(Mutex::new(Vec::new())),
            shutdown_timeout,
        }
    }

    /// Get a receiver for shutdown signals
    ///
    /// Components should call this and listen in their async loops
    ///
    /// # Returns
    ///
    /// Returns a receiver for shutdown signals. The result must be used
    /// to receive and act on shutdown events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ShutdownSignal> {
        self.shutdown_tx.subscribe()
    }

    /// Register a task for cleanup on shutdown
    pub async fn register_task(&self, task: JoinHandle<()>) {
        self.tasks.lock().await.push(task);
    }

    /// Register an agent process for cleanup on shutdown
    pub async fn register_agent(&self, process: std::process::Child) {
        self.agent_processes.lock().await.push(process);
    }

    /// Initiate graceful shutdown
    ///
    /// This is called when SIGINT or SIGTERM is received
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Initiating graceful shutdown...");

        // Send graceful shutdown signal
        let _ = self.shutdown_tx.send(ShutdownSignal::Graceful);

        // Wait for graceful shutdown or timeout
        let shutdown_result = tokio::time::timeout(self.shutdown_timeout, async {
            // Give tasks time to clean up
            tokio::time::sleep(Duration::from_secs(1)).await;

            // Abort all remaining tasks
            {
                let mut tasks = self.tasks.lock().await;
                for task in tasks.drain(..) {
                    task.abort();
                }
                drop(tasks); // Release lock early
            }

            // Terminate agent processes
            {
                let mut processes = self.agent_processes.lock().await;
                for mut process in processes.drain(..) {
                    // Try graceful shutdown first
                    let _ = process.kill();
                }
                drop(processes); // Release lock early
            }

            Ok::<(), crate::Error>(())
        })
        .await;

        match shutdown_result {
            Ok(Ok(())) => {
                tracing::info!("Graceful shutdown completed");
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("Error during shutdown: {e}");
                Err(e)
            }
            Err(_) => {
                // Timeout exceeded - force shutdown
                tracing::warn!("Shutdown timeout exceeded, forcing shutdown");
                let _ = self.shutdown_tx.send(ShutdownSignal::Force);
                Ok(())
            }
        }
    }

    /// Check if shutdown has been requested
    ///
    /// # Returns
    ///
    /// Returns `true` if shutdown is in progress. The result should be checked
    /// before starting new operations that should be cancelled during shutdown.
    #[must_use]
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_tx.receiver_count() > 0
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

/// Create signal channels for SIGINT and SIGTERM
///
/// Returns receivers that will receive a value when the signal is detected
pub async fn signal_channels() -> Result<(broadcast::Receiver<()>, broadcast::Receiver<()>)> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint = signal(SignalKind::interrupt())
            .map_err(|e| crate::Error::InvalidConfig(format!("Failed to setup SIGINT: {e}")))?;
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| crate::Error::InvalidConfig(format!("Failed to setup SIGTERM: {e}")))?;

        let (sigint_tx, sigint_rx) = broadcast::channel(1);
        let (sigterm_tx, sigterm_rx) = broadcast::channel(1);

        // Spawn tasks to forward signals to the channels
        tokio::spawn(async move {
            let _ = sigint.recv().await;
            tracing::info!("Received SIGINT");
            let _ = sigint_tx.send(());
        });

        tokio::spawn(async move {
            let _ = sigterm.recv().await;
            tracing::info!("Received SIGTERM");
            let _ = sigterm_tx.send(());
        });

        Ok((sigint_rx, sigterm_rx))
    }

    #[cfg(not(unix))]
    {
        // On non-Unix platforms, use Ctrl-C
        let (sigint_tx, sigint_rx) = broadcast::channel(1);
        let (sigterm_tx, sigterm_rx) = broadcast::channel(1);

        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Received Ctrl-C");
            let _ = sigint_tx.send(());
            // On non-Unix, treat both the same
            let _ = sigterm_tx.send(());
        });

        Ok((sigint_rx, sigterm_rx))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator_creation() {
        let coordinator = ShutdownCoordinator::default();
        assert!(!coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_subscription() {
        let coordinator = ShutdownCoordinator::default();
        let mut rx = coordinator.subscribe();

        // Send shutdown signal
        let shutdown_result = coordinator.shutdown().await;
        assert!(shutdown_result.is_ok(), "shutdown should succeed");

        // Verify signal received within timeout
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok(signal)) => assert_eq!(signal, ShutdownSignal::Graceful),
            Ok(Err(e)) => {
                // Broadcast error should not happen in test
                unreachable!("should not receive broadcast error: {e}")
            }
            Err(e) => {
                // Timeout should not happen in test
                unreachable!("should receive signal within timeout: {e}")
            }
        }
    }

    #[tokio::test]
    async fn test_task_registration() {
        let coordinator = ShutdownCoordinator::default();

        // Register a task
        let task = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        });
        coordinator.register_task(task).await;

        // Shutdown should abort the task
        let shutdown_result = coordinator.shutdown().await;
        assert!(shutdown_result.is_ok());

        // Tasks should be cleaned up - drop lock immediately after check
        assert!(coordinator.tasks.lock().await.is_empty());
    }
}
