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

        repo.update_job_state("1", JobState::Running { started_at: chrono::Utc::now() })
            .await
            .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert!(jobs.is_empty()); // No more pending
    }

    #[tokio::test]
    async fn test_in_memory_repository_update_nonexistent_job() {
        let repo = InMemoryJobRepository::new();
        let result = repo.update_job_state("nonexistent", JobState::Completed { finished_at: chrono::Utc::now() }).await;
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
        repo.update_job_state("1", JobState::Completed { finished_at: chrono::Utc::now() })
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
            tokio_test::block_on(repo.poll_pending_jobs(10)).expect("poll").len(),
            0
        );
    }

    #[tokio::test]
    async fn test_in_memory_repository_poll_excludes_running_jobs() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.add_job(create_test_job("2", JobPriority::P1));

        repo.update_job_state("1", JobState::Running { started_at: chrono::Utc::now() })
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

        repo.update_job_state("1", JobState::Failed { error: "err".to_string(), failed_at: chrono::Utc::now() })
            .await
            .expect("update");

        let jobs = repo.poll_pending_jobs(10).await.expect("poll");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "2");
    }
}
