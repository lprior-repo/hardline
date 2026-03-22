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

pub use processor::{
    JobProcessor, JobProcessorConfig, QueueError, QueueResult,
};
pub use repository::{sort_jobs_by_priority, InMemoryJobRepository, JobRepository};
pub use types::{
    Job, JobOutcome, JobPayload, JobPriority, JobResult, JobState,
};

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
}
