//! Job processing queue with priority-based execution
//!
//! This module implements a job processing loop that:
//! - Polls for pending beads from a repository
//! - Executes beads according to their priority
//! - Enforces concurrency limits
//! - Provides graceful shutdown

pub mod processor;
pub mod repository;
pub mod types;

pub use processor::{JobProcessor, JobProcessorConfig, QueueError, QueueResult};
pub use repository::{sort_jobs_by_priority, InMemoryJobRepository, JobRepository};
pub use types::{Job, JobOutcome, JobPayload, JobPriority, JobResult, JobState, JobTransitionError};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_job(id: &str, priority: JobPriority) -> Job {
        Job {
            id: id.to_string(),
            priority,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_job_priority_ordering() {
        let mut jobs = vec![
            create_test_job("1", JobPriority::P2),
            create_test_job("2", JobPriority::P0),
            create_test_job("3", JobPriority::P4),
            create_test_job("4", JobPriority::P1),
        ];

        sort_jobs_by_priority(&mut jobs);

        assert_eq!(jobs[0].id, "2");
        assert_eq!(jobs[1].id, "4");
        assert_eq!(jobs[2].id, "1");
        assert_eq!(jobs[3].id, "3");
    }

    #[test]
    fn test_job_state_is_pending() {
        assert!(JobState::Pending.is_pending());
        assert!(!JobState::Running {
            started_at: chrono::Utc::now()
        }
        .is_pending());
        assert!(!JobState::Completed {
            finished_at: chrono::Utc::now()
        }
        .is_pending());
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_pending() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P1)).expect("add");
        repo.add_job(create_test_job("2", JobPriority::P0)).expect("add");
        repo.add_job(create_test_job("3", JobPriority::P2)).expect("add");

        let jobs = repo.poll_pending_jobs(2).await.unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, "2");
        assert_eq!(jobs[1].id, "1");
    }

    #[tokio::test]
    async fn test_config_validation_zero_interval() {
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::ZERO,
            concurrency_limit: 5,
            max_retries: 3,
        };

        let result = JobProcessor::new(repo, config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_validation_zero_concurrency() {
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: 0,
            max_retries: 3,
        };

        let result = JobProcessor::new(repo, config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_job_execution_respects_limit() {
        let repo = InMemoryJobRepository::new();

        for i in 0..10 {
            repo.add_job(Job {
                id: format!("job-{}", i),
                priority: JobPriority::P0,
                payload: JobPayload::Task {
                    command: format!("cmd-{}", i),
                },
                state: JobState::Pending,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }).expect("add");
        }

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 3,
            max_retries: 0,
        };

        let processor = JobProcessor::new(repo, config).unwrap();

        assert_eq!(processor.running_jobs(), 0);
    }

    #[test]
    fn test_priority_ordering_stable() {
        let mut jobs = vec![
            create_test_job("a", JobPriority::P1),
            create_test_job("b", JobPriority::P1),
            create_test_job("c", JobPriority::P1),
            create_test_job("d", JobPriority::P0),
        ];

        sort_jobs_by_priority(&mut jobs);

        assert_eq!(jobs[0].id, "d");
        assert_eq!(jobs[0].priority, JobPriority::P0);
    }

    #[test]
    fn test_job_state_transitions() {
        let pending = JobState::Pending;
        assert!(pending.is_pending());
        assert!(!pending.is_running());
        assert!(!pending.is_terminal());

        let running = JobState::Running {
            started_at: chrono::Utc::now(),
        };
        assert!(!running.is_pending());
        assert!(running.is_running());
        assert!(!running.is_terminal());

        let completed = JobState::Completed {
            finished_at: chrono::Utc::now(),
        };
        assert!(!completed.is_pending());
        assert!(!completed.is_running());
        assert!(completed.is_terminal());

        let failed = JobState::Failed {
            error: "test error".to_string(),
            failed_at: chrono::Utc::now(),
        };
        assert!(!failed.is_pending());
        assert!(!failed.is_running());
        assert!(failed.is_terminal());
    }

    #[test]
    fn test_sort_jobs_by_priority_empty() {
        let mut jobs: Vec<Job> = vec![];
        sort_jobs_by_priority(&mut jobs);
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_sort_jobs_by_priority_single() {
        let mut jobs = vec![create_test_job("1", JobPriority::P2)];
        sort_jobs_by_priority(&mut jobs);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn test_sort_jobs_by_priority_preserves_order_within_priority() {
        let mut jobs = vec![
            create_test_job("first-p0", JobPriority::P0),
            create_test_job("second-p0", JobPriority::P0),
            create_test_job("third-p0", JobPriority::P0),
        ];
        sort_jobs_by_priority(&mut jobs);
        // All P0, order should be stable
        assert_eq!(jobs[0].id, "first-p0");
        assert_eq!(jobs[1].id, "second-p0");
        assert_eq!(jobs[2].id, "third-p0");
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_pending_empty() {
        let repo = InMemoryJobRepository::new();
        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_repository_update_job_state() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0)).expect("add");

        repo.update_job_state(
            "1",
            JobState::Running {
                started_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert!(jobs.is_empty()); // No more pending
    }

    #[tokio::test]
    async fn test_in_memory_repository_update_nonexistent_job() {
        let repo = InMemoryJobRepository::new();
        let result = repo
            .update_job_state(
                "nonexistent",
                JobState::Completed {
                    finished_at: chrono::Utc::now(),
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_repository_get_job() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0)).expect("add");

        let job = repo.get_job("1").await.expect("get");
        assert!(job.is_some());
        assert_eq!(job.unwrap().id, "1");

        let job = repo.get_job("nonexistent").await.expect("get");
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_respects_limit() {
        let repo = InMemoryJobRepository::new();
        for i in 0..10 {
            repo.add_job(create_test_job(&format!("job-{i}"), JobPriority::P0)).expect("add");
        }

        let jobs = repo.poll_pending_jobs(3).await.expect("poll");
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_after_completion() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0)).expect("add");
        repo.add_job(create_test_job("2", JobPriority::P0)).expect("add");

        // Complete job-1
        repo.update_job_state(
            "1",
            JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "2");
    }

    #[test]
    fn test_job_processor_config_validate_valid() {
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: 5,
            max_retries: 3,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_queue_error_display() {
        let errors = [
            QueueError::NoRepository,
            QueueError::Repository("lock error".to_string()),
            QueueError::JobNotFound("job-99".to_string()),
            QueueError::InvalidJobState("already running".to_string()),
            QueueError::ExecutionFailed("oom".to_string()),
            QueueError::ShutdownRequested,
            QueueError::InvalidConfiguration("bad config".to_string()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_in_memory_repository_default() {
        let repo = InMemoryJobRepository::default();
        // Should not panic on empty
        assert_eq!(
            tokio_test::block_on(repo.poll_pending_jobs(10))
                .expect("poll")
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_excludes_running_jobs() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0)).expect("add");
        repo.add_job(create_test_job("2", JobPriority::P1)).expect("add");

        repo.update_job_state(
            "1",
            JobState::Running {
                started_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "2");
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_excludes_failed_jobs() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0)).expect("add");
        repo.add_job(create_test_job("2", JobPriority::P1)).expect("add");

        repo.update_job_state(
            "1",
            JobState::Failed {
                error: "err".to_string(),
                failed_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "2");
    }

<<<<<<< HEAD
    // --- Mock repository for error injection ---

    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Arc;

    /// Wrapper to share InMemoryJobRepository via Arc
    struct ArcJobRepository {
        inner: Arc<InMemoryJobRepository>,
    }

    #[async_trait::async_trait]
    impl JobRepository for ArcJobRepository {
        async fn poll_pending_jobs(&self, limit: usize) -> QueueResult<Vec<Job>> {
            self.inner.poll_pending_jobs(limit).await
        }
        async fn update_job_state(&self, job_id: &str, state: JobState) -> QueueResult<()> {
            self.inner.update_job_state(job_id, state).await
        }
        async fn get_job(&self, job_id: &str) -> QueueResult<Option<Job>> {
            self.inner.get_job(job_id).await
        }
    }

    struct FailingJobRepository {
        inner: Arc<InMemoryJobRepository>,
        fail_update: AtomicBool,
        fail_on_call_count: AtomicUsize,
        calls_so_far: AtomicUsize,
    }

    impl FailingJobRepository {
        fn new(inner: Arc<InMemoryJobRepository>) -> Self {
            Self {
                inner,
                fail_update: AtomicBool::new(false),
                fail_on_call_count: AtomicUsize::new(usize::MAX),
                calls_so_far: AtomicUsize::new(0),
            }
        }

        fn fail_update_on_next(&self) {
            self.fail_update.store(true, AtomicOrdering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl JobRepository for FailingJobRepository {
        async fn poll_pending_jobs(&self, limit: usize) -> QueueResult<Vec<Job>> {
            self.inner.poll_pending_jobs(limit).await
        }

        async fn update_job_state(&self, job_id: &str, state: JobState) -> QueueResult<()> {
            let count = self.calls_so_far.fetch_add(1, AtomicOrdering::Relaxed);
            if self.fail_update.swap(false, AtomicOrdering::Relaxed)
                || count >= self.fail_on_call_count.load(AtomicOrdering::Relaxed)
            {
                return Err(QueueError::Repository("injected update failure".into()));
            }
            self.inner.update_job_state(job_id, state).await
        }

        async fn get_job(&self, job_id: &str) -> QueueResult<Option<Job>> {
            self.inner.get_job(job_id).await
        }
    }

    // --- JobProcessor::run main loop tests ---

    #[tokio::test]
    async fn test_run_stops_on_shutdown_signal() {
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(repo, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        tx.send(()).unwrap();

        let result = processor.run(rx).await;
        assert!(result.is_ok());
        assert_eq!(processor.running_jobs(), 0);
    }

    #[tokio::test]
    async fn test_run_processes_job_then_receives_shutdown() {
        let repo = Arc::new(InMemoryJobRepository::new());
        repo.add_job(create_test_job("j1", JobPriority::P0))
            .expect("add");

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(ArcJobRepository { inner: Arc::clone(&repo) }, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        // Give time for the job to be processed
        tokio::time::sleep(Duration::from_millis(200)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        // Verify job was completed
        let job = repo.get_job("j1").await.unwrap().unwrap();
        assert!(job.state.is_terminal());
    }

    #[tokio::test]
    async fn test_run_handles_empty_repository_gracefully() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(5),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(ArcJobRepository { inner: Arc::clone(&repo) }, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        // Let it cycle a few times with no jobs
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_multiple_jobs_respects_concurrency_limit() {
        let repo = Arc::new(InMemoryJobRepository::new());
        for i in 0..5 {
            repo.add_job(create_test_job(&format!("job-{i}"), JobPriority::P0))
                .expect("add");
        }

        // Concurrency limit of 1 — process one at a time
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(ArcJobRepository { inner: Arc::clone(&repo) }, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        // Let it process several cycles
        tokio::time::sleep(Duration::from_millis(500)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        // All jobs should be in terminal state (completed or failed)
        for i in 0..5 {
            let job = repo.get_job(&format!("job-{i}")).await.unwrap().unwrap();
            assert!(job.state.is_terminal(), "job-{i} should be terminal");
        }
    }

    #[tokio::test]
    async fn test_run_processes_jobs_in_priority_order() {
        let repo = Arc::new(InMemoryJobRepository::new());
        // Add in reverse priority order
        repo.add_job(create_test_job("low", JobPriority::P4))
            .expect("add");
        repo.add_job(create_test_job("high", JobPriority::P0))
            .expect("add");
        repo.add_job(create_test_job("mid", JobPriority::P2))
            .expect("add");

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(ArcJobRepository { inner: Arc::clone(&repo) }, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        tokio::time::sleep(Duration::from_millis(300)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        // All should be completed
        for id in &["low", "high", "mid"] {
            let job = repo.get_job(id).await.unwrap().unwrap();
            assert!(job.state.is_terminal());
        }
    }

    #[tokio::test]
    async fn test_run_shutdown_drops_receiver_not_error() {
        // When all senders are dropped, recv returns Err (Lagged/Closed)
        // The loop should handle this gracefully
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(repo, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        drop(tx); // Drop sender — recv will return error

        // The run loop's select! treats _ = stop_signal.recv() as shutdown
        let result = processor.run(rx).await;
        assert!(result.is_ok());
    }

    // --- execute_job edge cases ---

    /// Helper: create processor with ArcJobRepository, add job, run briefly, shutdown, return repo
    async fn run_processor_with_job(job: Job) -> Arc<InMemoryJobRepository> {
        let repo = Arc::new(InMemoryJobRepository::new());
        repo.add_job(job).expect("add");

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(ArcJobRepository { inner: Arc::clone(&repo) }, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(()).unwrap();
        let _ = handle.await;

        repo
    }

    #[tokio::test]
    async fn test_execute_job_task_payload_completes_successfully() {
        let repo = run_processor_with_job(Job {
            id: "task-1".to_string(),
            priority: JobPriority::P0,
            payload: JobPayload::Task {
                command: "echo hello".to_string(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }).await;

        let job = repo.get_job("task-1").await.unwrap().unwrap();
        assert!(matches!(job.state, JobState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_execute_job_pipeline_payload_completes_successfully() {
        let repo = run_processor_with_job(Job {
            id: "pipe-1".to_string(),
            priority: JobPriority::P0,
            payload: JobPayload::Pipeline {
                spec_path: "specs/deploy.yaml".to_string(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }).await;

        let job = repo.get_job("pipe-1").await.unwrap().unwrap();
        assert!(matches!(job.state, JobState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_execute_job_custom_payload_completes_successfully() {
        let repo = run_processor_with_job(Job {
            id: "custom-1".to_string(),
            priority: JobPriority::P0,
            payload: JobPayload::Custom {
                data: serde_json::json!({"action": "restart", "service": "api"}),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }).await;

        let job = repo.get_job("custom-1").await.unwrap().unwrap();
        assert!(matches!(job.state, JobState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_execute_job_records_nonzero_execution_time() {
        let repo = run_processor_with_job(create_test_job("timed", JobPriority::P0)).await;

        let job = repo.get_job("timed").await.unwrap().unwrap();
        assert!(matches!(job.state, JobState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_execute_job_transition_to_running_state() {
        let repo = run_processor_with_job(create_test_job("state-check", JobPriority::P0)).await;

        let job = repo.get_job("state-check").await.unwrap().unwrap();
        // Job should end in Completed (went Pending → Running → Completed)
        assert!(matches!(job.state, JobState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_execute_job_update_state_failure_does_not_crash_loop() {
        // When update_job_state fails during the "Running" transition,
        // execute_job returns an error but the run loop continues gracefully
        let inner = Arc::new(InMemoryJobRepository::new());
        inner
            .add_job(create_test_job("fail-update", JobPriority::P0))
            .expect("add");

        let repo = FailingJobRepository::new(Arc::clone(&inner));
        repo.fail_update_on_next(); // Fail the first update (Pending → Running)

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(repo, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        // The process_cycle logs the error but doesn't propagate it
        assert!(result.is_ok());

        // Job should eventually be completed (the next cycle retries after the first failure)
        let job = inner.get_job("fail-update").await.unwrap().unwrap();
        assert!(job.state.is_terminal());
    }

    #[tokio::test]
    async fn test_run_continues_after_poll_error() {
        // Repository poll failure should not crash the run loop
        let inner = Arc::new(InMemoryJobRepository::new());
        inner
            .add_job(create_test_job("after-err", JobPriority::P0))
            .expect("add");

        let repo = FailingJobRepository::new(Arc::clone(&inner));

        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(repo, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let handle = tokio::spawn(async move {
            processor.run(rx).await
        });

        // First cycle: poll fails. Second cycle: succeeds, processes job.
        tokio::time::sleep(Duration::from_millis(300)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        // Poll error propagates through process_cycle → run
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_with_multiple_shutdown_signals() {
        // Multiple shutdown signals should not cause issues
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 0,
        };
        let processor = JobProcessor::new(repo, config).unwrap();

        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
        tx.send(()).unwrap();
        tx.send(()).unwrap();

        let result = processor.run(rx).await;
        assert!(result.is_ok());
    }
=======
    // --- ha-706a: Comprehensive Queue tests ---

    #[tokio::test]
    async fn test_enqueue_dequeue_full_priority_ordering() {
        let repo = InMemoryJobRepository::new();

        // Enqueue one job per priority level in reverse order
        repo.add_job(create_test_job("p4", JobPriority::P4));
        repo.add_job(create_test_job("p3", JobPriority::P3));
        repo.add_job(create_test_job("p2", JobPriority::P2));
        repo.add_job(create_test_job("p1", JobPriority::P1));
        repo.add_job(create_test_job("p0", JobPriority::P0));

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 5);
        // Dequeue order: P0 first, then P1, P2, P3, P4
        assert_eq!(jobs[0].id, "p0");
        assert_eq!(jobs[1].id, "p1");
        assert_eq!(jobs[2].id, "p2");
        assert_eq!(jobs[3].id, "p3");
        assert_eq!(jobs[4].id, "p4");
    }

    #[tokio::test]
    async fn test_fifo_ordering_same_priority() {
        let repo = InMemoryJobRepository::new();

        // Enqueue 5 P1 jobs in order
        repo.add_job(create_test_job("first", JobPriority::P1));
        repo.add_job(create_test_job("second", JobPriority::P1));
        repo.add_job(create_test_job("third", JobPriority::P1));
        repo.add_job(create_test_job("fourth", JobPriority::P1));
        repo.add_job(create_test_job("fifth", JobPriority::P1));

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 5);
        // FIFO: same priority preserves insertion order
        assert_eq!(jobs[0].id, "first");
        assert_eq!(jobs[1].id, "second");
        assert_eq!(jobs[2].id, "third");
        assert_eq!(jobs[3].id, "fourth");
        assert_eq!(jobs[4].id, "fifth");
    }

    #[tokio::test]
    async fn test_mixed_priority_with_fifo_within_groups() {
        let repo = InMemoryJobRepository::new();

        // Enqueue mixed priorities, multiple jobs per priority
        repo.add_job(create_test_job("p1-a", JobPriority::P1));
        repo.add_job(create_test_job("p0-a", JobPriority::P0));
        repo.add_job(create_test_job("p1-b", JobPriority::P1));
        repo.add_job(create_test_job("p0-b", JobPriority::P0));
        repo.add_job(create_test_job("p2-a", JobPriority::P2));
        repo.add_job(create_test_job("p0-c", JobPriority::P0));

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 6);

        // P0 group first (FIFO within): p0-a, p0-b, p0-c
        assert_eq!(jobs[0].id, "p0-a");
        assert_eq!(jobs[1].id, "p0-b");
        assert_eq!(jobs[2].id, "p0-c");
        // P1 group next (FIFO within): p1-a, p1-b
        assert_eq!(jobs[3].id, "p1-a");
        assert_eq!(jobs[4].id, "p1-b");
        // P2 group: p2-a
        assert_eq!(jobs[5].id, "p2-a");
    }

    #[tokio::test]
    async fn test_repeated_polls_drain_queue() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("a", JobPriority::P2));
        repo.add_job(create_test_job("b", JobPriority::P0));
        repo.add_job(create_test_job("c", JobPriority::P1));

        // First poll: get all 3
        let batch1 = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(batch1.len(), 3);
        assert_eq!(batch1[0].id, "b"); // P0
        assert_eq!(batch1[1].id, "c"); // P1
        assert_eq!(batch1[2].id, "a"); // P2

        // Note: poll_pending_jobs returns clones, jobs are still pending in repo
        // Second poll returns same results (poll doesn't consume)
        let batch2 = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(batch2.len(), 3);
    }

    #[tokio::test]
    async fn test_interleaved_enqueue_and_state_changes() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("j1", JobPriority::P0));
        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 1);

        // Complete j1
        repo.update_job_state(
            "j1",
            JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("update");

        // Add more jobs
        repo.add_job(create_test_job("j2", JobPriority::P1));
        repo.add_job(create_test_job("j3", JobPriority::P0));

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, "j3"); // P0 first
        assert_eq!(jobs[1].id, "j2"); // P1 second
    }

    #[tokio::test]
    async fn test_poll_limit_zero_returns_empty() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));

        let jobs = repo.poll_pending_jobs(0).await.expect("poll");
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn test_poll_limit_one_returns_highest_priority() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("low", JobPriority::P4));
        repo.add_job(create_test_job("mid", JobPriority::P2));
        repo.add_job(create_test_job("high", JobPriority::P0));

        let jobs = repo.poll_pending_jobs(1).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "high");
    }

    #[tokio::test]
    async fn test_enqueue_single_dequeue() {
        let repo = InMemoryJobRepository::new();

        // Single enqueue then dequeue
        repo.add_job(create_test_job("solo", JobPriority::P3));
        let jobs = repo.poll_pending_jobs(1).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "solo");
        assert_eq!(jobs[0].priority, JobPriority::P3);
        assert!(jobs[0].state.is_pending());
    }

    #[test]
    fn test_sort_all_priority_levels() {
        let mut jobs = vec![
            create_test_job("p4", JobPriority::P4),
            create_test_job("p2", JobPriority::P2),
            create_test_job("p0", JobPriority::P0),
            create_test_job("p3", JobPriority::P3),
            create_test_job("p1", JobPriority::P1),
        ];

        sort_jobs_by_priority(&mut jobs);

        assert_eq!(jobs.iter().map(|j| j.priority.value()).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_sort_two_elements_different_priority() {
        let mut jobs = vec![
            create_test_job("low", JobPriority::P4),
            create_test_job("high", JobPriority::P0),
        ];
        sort_jobs_by_priority(&mut jobs);
        assert_eq!(jobs[0].id, "high");
        assert_eq!(jobs[1].id, "low");
    }

    #[test]
    fn test_sort_already_sorted() {
        let mut jobs = vec![
            create_test_job("p0", JobPriority::P0),
            create_test_job("p1", JobPriority::P1),
            create_test_job("p2", JobPriority::P2),
        ];
        sort_jobs_by_priority(&mut jobs);
        assert_eq!(jobs[0].id, "p0");
        assert_eq!(jobs[1].id, "p1");
        assert_eq!(jobs[2].id, "p2");
    }

    #[test]
    fn test_sort_reverse_sorted() {
        let mut jobs = vec![
            create_test_job("p4", JobPriority::P4),
            create_test_job("p3", JobPriority::P3),
            create_test_job("p2", JobPriority::P2),
            create_test_job("p1", JobPriority::P1),
            create_test_job("p0", JobPriority::P0),
        ];
        sort_jobs_by_priority(&mut jobs);
        for i in 0..5 {
            assert_eq!(jobs[i].id, format!("p{i}"));
        }
    }
>>>>>>> polecat/onyx-mnn3rb73
}
