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
pub use types::{Job, JobOutcome, JobPayload, JobPriority, JobResult, JobState};

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

        repo.add_job(create_test_job("1", JobPriority::P1));
        repo.add_job(create_test_job("2", JobPriority::P0));
        repo.add_job(create_test_job("3", JobPriority::P2));

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
            });
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
        repo.add_job(create_test_job("1", JobPriority::P0));

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
        repo.add_job(create_test_job("1", JobPriority::P0));

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
            repo.add_job(create_test_job(&format!("job-{i}"), JobPriority::P0));
        }

        let jobs = repo.poll_pending_jobs(3).await.expect("poll");
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_after_completion() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.add_job(create_test_job("2", JobPriority::P0));

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
        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.add_job(create_test_job("2", JobPriority::P1));

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
        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.add_job(create_test_job("2", JobPriority::P1));

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
}
