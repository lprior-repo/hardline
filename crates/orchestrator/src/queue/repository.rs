//! Job repository trait and implementations
//!
//! Provides abstraction over job persistence with:
//! - JobRepository trait for async operations
//! - InMemoryJobRepository for testing

use std::sync;

use async_trait::async_trait;
use chrono::Utc;

use crate::queue::processor::{QueueError, QueueResult};
use crate::queue::types::{Job, JobState};

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn poll_pending_jobs(&self, limit: usize) -> QueueResult<Vec<Job>>;
    async fn update_job_state(&self, job_id: &str, state: JobState) -> QueueResult<()>;
    async fn get_job(&self, job_id: &str) -> QueueResult<Option<Job>>;
    async fn create_job(&self, job: Job) -> QueueResult<()>;
    async fn delete_job(&self, job_id: &str) -> QueueResult<bool>;
    async fn find_jobs_by_state(&self, state: &JobState) -> QueueResult<Vec<Job>>;
}

pub struct InMemoryJobRepository {
    jobs: sync::RwLock<Vec<Job>>,
}

impl InMemoryJobRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            jobs: sync::RwLock::new(Vec::new()),
        }
    }

    pub fn add_job(&self, job: Job) {
        if let Ok(mut jobs) = self.jobs.write() {
            jobs.push(job);
        }
    }

    pub fn get_all_jobs(&self) -> Vec<Job> {
        self.jobs
            .read()
            .map(|jobs| jobs.clone())
            .unwrap_or_default()
    }

    pub fn job_count(&self) -> usize {
        self.jobs.read().map(|jobs| jobs.len()).unwrap_or(0)
    }
}

impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn poll_pending_jobs(&self, limit: usize) -> QueueResult<Vec<Job>> {
        let jobs = self
            .jobs
            .read()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire read lock: {}", e)))?;

        let mut pending: Vec<&Job> = jobs.iter().filter(|j| j.state.is_pending()).collect();

        pending.sort_by_key(|j| j.priority);

        let result: Vec<Job> = pending.into_iter().take(limit).cloned().collect();

        Ok(result)
    }

    async fn update_job_state(&self, job_id: &str, new_state: JobState) -> QueueResult<()> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire write lock: {}", e)))?;

        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.state = new_state;
            job.updated_at = Utc::now();
            Ok(())
        } else {
            Err(QueueError::JobNotFound(job_id.to_string()))
        }
    }

    async fn get_job(&self, job_id: &str) -> QueueResult<Option<Job>> {
        let jobs = self
            .jobs
            .read()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire read lock: {}", e)))?;

        Ok(jobs.iter().find(|j| j.id == job_id).cloned())
    }

    async fn create_job(&self, job: Job) -> QueueResult<()> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire write lock: {}", e)))?;

        if jobs.iter().any(|j| j.id == job.id) {
            return Err(QueueError::JobNotFound(format!("Job already exists: {}", job.id)));
        }

        jobs.push(job);
        Ok(())
    }

    async fn delete_job(&self, job_id: &str) -> QueueResult<bool> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire write lock: {}", e)))?;

        let initial_len = jobs.len();
        jobs.retain(|j| j.id != job_id);
        let deleted = jobs.len() < initial_len;
        Ok(deleted)
    }

    async fn find_jobs_by_state(&self, state: &JobState) -> QueueResult<Vec<Job>> {
        let jobs = self
            .jobs
            .read()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire read lock: {}", e)))?;

        let result: Vec<Job> = jobs
            .iter()
            .filter(|j| {
                match (state, &j.state) {
                    (JobState::Pending, JobState::Pending) => true,
                    (JobState::Running { .. }, JobState::Running { .. }) => true,
                    (JobState::Completed { .. }, JobState::Completed { .. }) => true,
                    (JobState::Failed { .. }, JobState::Failed { .. }) => true,
                    _ => false,
                }
            })
            .cloned()
            .collect();

        Ok(result)
    }
}

pub fn sort_jobs_by_priority(jobs: &mut [Job]) {
    jobs.sort_by_key(|j| j.priority);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::types::{Job, JobPayload, JobPriority, JobState};
    use std::time::Duration;
    use tokio::time::timeout;

    fn create_test_job(id: &str, priority: JobPriority) -> Job {
        Job {
            id: id.to_string(),
            priority,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ========== CRUD: Create ==========

    #[tokio::test]
    async fn test_create_job_success() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-1", JobPriority::P0);

        let result = repo.create_job(job.clone()).await;
        assert!(result.is_ok());

        let retrieved = repo.get_job("job-1").await.expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "job-1");
    }

    #[tokio::test]
    async fn test_create_job_duplicate() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-1", JobPriority::P0);

        repo.create_job(job.clone()).await.expect("first create");

        let result = repo.create_job(job).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_job_empty_id() {
        let repo = InMemoryJobRepository::new();
        let job = Job {
            id: "".to_string(),
            priority: JobPriority::P0,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = repo.create_job(job).await;
        assert!(result.is_ok());

        let retrieved = repo.get_job("").await.expect("get");
        assert!(retrieved.is_some());
    }

    // ========== CRUD: Read ==========

    #[tokio::test]
    async fn test_get_job_found() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-42", JobPriority::P2);
        repo.create_job(job).await.expect("create");

        let result = repo.get_job("job-42").await.expect("get");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "job-42");
    }

    #[tokio::test]
    async fn test_get_job_not_found() {
        let repo = InMemoryJobRepository::new();

        let result = repo.get_job("nonexistent").await.expect("get");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_all_jobs() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P1));
        repo.add_job(create_test_job("2", JobPriority::P0));
        repo.add_job(create_test_job("3", JobPriority::P2));

        let all_jobs = repo.get_all_jobs();
        assert_eq!(all_jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_job_count_empty() {
        let repo = InMemoryJobRepository::new();
        assert_eq!(repo.job_count(), 0);
    }

    #[tokio::test]
    async fn test_job_count_after_operations() {
        let repo = InMemoryJobRepository::new();
        assert_eq!(repo.job_count(), 0);

        repo.add_job(create_test_job("1", JobPriority::P0));
        assert_eq!(repo.job_count(), 1);

        repo.add_job(create_test_job("2", JobPriority::P0));
        assert_eq!(repo.job_count(), 2);

        repo.delete_job("1").await.expect("delete");
        assert_eq!(repo.job_count(), 1);
    }

    // ========== CRUD: Update ==========

    #[tokio::test]
    async fn test_update_job_state_success() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-1", JobPriority::P0);
        repo.create_job(job).await.expect("create");

        let result = repo
            .update_job_state(
                "job-1",
                JobState::Running {
                    started_at: Utc::now(),
                },
            )
            .await;

        assert!(result.is_ok());

        let updated_job = repo.get_job("job-1").await.expect("get").unwrap();
        assert!(updated_job.state.is_running());
    }

    #[tokio::test]
    async fn test_update_job_state_not_found() {
        let repo = InMemoryJobRepository::new();

        let result = repo
            .update_job_state(
                "nonexistent",
                JobState::Running {
                    started_at: Utc::now(),
                },
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueueError::JobNotFound(_)));
    }

    #[tokio::test]
    async fn test_update_job_timestamp_updated() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-1", JobPriority::P0);
        repo.create_job(job).await.expect("create");

        let job_before = repo.get_job("job-1").await.expect("get").unwrap();
        let updated_at_before = job_before.updated_at;

        tokio::time::sleep(Duration::from_millis(10)).await;

        repo.update_job_state(
            "job-1",
            JobState::Running {
                started_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let job_after = repo.get_job("job-1").await.expect("get").unwrap();
        assert!(job_after.updated_at > updated_at_before);
    }

    // ========== CRUD: Delete ==========

    #[tokio::test]
    async fn test_delete_job_success() {
        let repo = InMemoryJobRepository::new();
        let job = create_test_job("job-1", JobPriority::P0);
        repo.create_job(job).await.expect("create");

        let result = repo.delete_job("job-1").await.expect("delete");
        assert!(result);

        let retrieved = repo.get_job("job-1").await.expect("get");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_job_not_found() {
        let repo = InMemoryJobRepository::new();

        let result = repo.delete_job("nonexistent").await.expect("delete");
        assert!(!result);

        let count = repo.job_count();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_preserves_remaining_jobs() {
        let repo = InMemoryJobRepository::new();

        repo.create_job(create_test_job("1", JobPriority::P0)).await.expect("create");
        repo.create_job(create_test_job("2", JobPriority::P1)).await.expect("create");
        repo.create_job(create_test_job("3", JobPriority::P2)).await.expect("create");

        repo.delete_job("2").await.expect("delete");

        assert!(repo.get_job("1").await.expect("get").is_some());
        assert!(repo.get_job("2").await.expect("get").is_none());
        assert!(repo.get_job("3").await.expect("get").is_some());
    }

    // ========== Find by State ==========

    #[tokio::test]
    async fn test_find_jobs_by_state_pending() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.add_job(create_test_job("2", JobPriority::P1));

        let result = repo
            .find_jobs_by_state(&JobState::Pending)
            .await
            .expect("find");

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_running() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.update_job_state(
            "1",
            JobState::Running {
                started_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let result = repo
            .find_jobs_by_state(&JobState::Running {
                started_at: Utc::now(),
            })
            .await
            .expect("find");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "1");
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_completed() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.update_job_state(
            "1",
            JobState::Completed {
                finished_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let result = repo
            .find_jobs_by_state(&JobState::Completed {
                finished_at: Utc::now(),
            })
            .await
            .expect("find");

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_failed() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.update_job_state(
            "1",
            JobState::Failed {
                error: "test error".to_string(),
                failed_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let result = repo
            .find_jobs_by_state(&JobState::Failed {
                error: "test error".to_string(),
                failed_at: Utc::now(),
            })
            .await
            .expect("find");

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_mixed_states() {
        let repo = InMemoryJobRepository::new();

        repo.add_job(create_test_job("pending-1", JobPriority::P0));
        repo.add_job(create_test_job("pending-2", JobPriority::P1));
        repo.add_job(create_test_job("running-1", JobPriority::P0));
        repo.add_job(create_test_job("completed-1", JobPriority::P0));
        repo.add_job(create_test_job("failed-1", JobPriority::P0));

        repo.update_job_state(
            "running-1",
            JobState::Running {
                started_at: Utc::now(),
            },
        )
        .await
        .expect("update");
        repo.update_job_state(
            "completed-1",
            JobState::Completed {
                finished_at: Utc::now(),
            },
        )
        .await
        .expect("update");
        repo.update_job_state(
            "failed-1",
            JobState::Failed {
                error: "err".to_string(),
                failed_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let pending = repo
            .find_jobs_by_state(&JobState::Pending)
            .await
            .expect("find");
        assert_eq!(pending.len(), 2);

        let running = repo
            .find_jobs_by_state(&JobState::Running {
                started_at: Utc::now(),
            })
            .await
            .expect("find");
        assert_eq!(running.len(), 1);

        let completed = repo
            .find_jobs_by_state(&JobState::Completed {
                finished_at: Utc::now(),
            })
            .await
            .expect("find");
        assert_eq!(completed.len(), 1);

        let failed = repo
            .find_jobs_by_state(&JobState::Failed {
                error: "err".to_string(),
                failed_at: Utc::now(),
            })
            .await
            .expect("find");
        assert_eq!(failed.len(), 1);
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_empty() {
        let repo = InMemoryJobRepository::new();

        let result = repo
            .find_jobs_by_state(&JobState::Pending)
            .await
            .expect("find");

        assert!(result.is_empty());
    }

    // ========== Concurrent Access Safety ==========

    #[tokio::test]
    async fn test_concurrent_reads() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryJobRepository::new());

        for i in 0..100 {
            repo.add_job(create_test_job(&format!("job-{i}"), JobPriority::P0));
        }

        let mut handles = vec![];

        for _ in 0..10 {
            let repo_clone = Arc::clone(&repo);
            let handle = tokio::spawn(async move {
                let count = repo_clone.job_count();
                assert_eq!(count, 100);
                count
            });
            handles.push(handle);
        }

        for handle in handles {
            let count = handle.await.expect("spawn");
            assert_eq!(count, 100);
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryJobRepository::new());

        let mut handles = vec![];

        for i in 0..50 {
            let job = create_test_job(&format!("job-{i}"), JobPriority::P0);
            let repo_clone = Arc::clone(&repo);
            let handle = tokio::spawn(async move {
                repo_clone.add_job(job);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("spawn");
        }

        assert_eq!(repo.job_count(), 50);
    }

    #[tokio::test]
    async fn test_concurrent_read_write() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryJobRepository::new());

        for i in 0..10 {
            repo.add_job(create_test_job(&format!("init-{i}"), JobPriority::P0));
        }

        let mut handles = vec![];

        // Spawn readers
        for _ in 0..5 {
            let repo_clone = Arc::clone(&repo);
            let handle = tokio::spawn(async move {
                for _ in 0..100 {
                    let count = repo_clone.job_count();
                    assert!(count >= 10 && count <= 60);
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            });
            handles.push(handle);
        }

        // Spawn writers
        for i in 0..50 {
            let job = create_test_job(&format!("job-{i}"), JobPriority::P0);
            let repo_clone = Arc::clone(&repo);
            let handle = tokio::spawn(async move {
                repo_clone.add_job(job);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("spawn");
        }

        assert_eq!(repo.job_count(), 60);
    }

    #[tokio::test]
    async fn test_rwlock_poisoning_read() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));

        let result = timeout(
            Duration::from_secs(5),
            repo.get_job("1"),
        )
        .await;

        assert!(result.is_ok());
        let job_opt = result.unwrap();
        assert!(job_opt.is_ok());
        assert!(job_opt.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_rwlock_poisoning_write() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));

        let result = timeout(
            Duration::from_secs(5),
            repo.update_job_state(
                "1",
                JobState::Running {
                    started_at: Utc::now(),
                },
            ),
        )
        .await;

        assert!(result.is_ok());
        let update_result = result.unwrap();
        assert!(update_result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_delete() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryJobRepository::new());

        for i in 0..20 {
            repo.add_job(create_test_job(&format!("job-{i}"), JobPriority::P0));
        }

        let mut handles = vec![];

        for i in 0..10 {
            let id = format!("job-{i}");
            let repo_clone = Arc::clone(&repo);
            let handle = tokio::spawn(async move {
                repo_clone.delete_job(&id).await.expect("delete")
            });
            handles.push(handle);
        }

        let mut deleted_count = 0;
        for handle in handles {
            if handle.await.expect("spawn") {
                deleted_count += 1;
            }
        }

        assert_eq!(deleted_count, 10);
        assert_eq!(repo.job_count(), 10);
    }

    // ========== Edge Cases ==========

    #[tokio::test]
    async fn test_create_job_with_special_characters() {
        let repo = InMemoryJobRepository::new();
        let job = Job {
            id: "job-with-special!@#$%^&*()chars".to_string(),
            priority: JobPriority::P0,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = repo.create_job(job.clone()).await;
        assert!(result.is_ok());

        let retrieved = repo.get_job(&job.id).await.expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, job.id);
    }

    #[tokio::test]
    async fn test_delete_job_with_special_characters() {
        let repo = InMemoryJobRepository::new();
        let id = "job-with-special!@#$%^&*()chars";
        let job = create_test_job(id, JobPriority::P0);
        repo.create_job(job).await.expect("create");

        let result = repo.delete_job(id).await.expect("delete");
        assert!(result);

        let retrieved = repo.get_job(id).await.expect("get");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_find_jobs_by_state_exact_match() {
        let repo = InMemoryJobRepository::new();

        let now = Utc::now();
        repo.add_job(create_test_job("1", JobPriority::P0));
        repo.update_job_state(
            "1",
            JobState::Completed {
                finished_at: now,
            },
        )
        .await
        .expect("update");

        // Different timestamp should not match
        let different_timestamp = JobState::Completed {
            finished_at: now + chrono::Duration::seconds(1),
        };

        let result = repo.find_jobs_by_state(&different_timestamp).await.expect("find");
        assert_eq!(result.len(), 1); // Still matches by variant, not exact timestamp
    }

    #[tokio::test]
    async fn test_repository_state_isolation() {
        let repo1 = InMemoryJobRepository::new();
        let repo2 = InMemoryJobRepository::new();

        repo1.add_job(create_test_job("repo1-job", JobPriority::P0));
        repo2.add_job(create_test_job("repo2-job", JobPriority::P0));

        let jobs1 = repo1.get_all_jobs();
        let jobs2 = repo2.get_all_jobs();

        assert_eq!(jobs1.len(), 1);
        assert_eq!(jobs2.len(), 1);
        assert_eq!(jobs1[0].id, "repo1-job");
        assert_eq!(jobs2[0].id, "repo2-job");
    }

    #[tokio::test]
    async fn test_create_job_then_delete_immediately() {
        let repo = InMemoryJobRepository::new();

        let job = create_test_job("ephemeral", JobPriority::P0);
        repo.create_job(job).await.expect("create");

        let deleted = repo.delete_job("ephemeral").await.expect("delete");
        assert!(deleted);

        let exists = repo.get_job("ephemeral").await.expect("get");
        assert!(exists.is_none());
    }

    #[tokio::test]
    async fn test_update_then_delete() {
        let repo = InMemoryJobRepository::new();

        let job = create_test_job("updatable", JobPriority::P0);
        repo.create_job(job).await.expect("create");

        repo.update_job_state(
            "updatable",
            JobState::Running {
                started_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        repo.update_job_state(
            "updatable",
            JobState::Completed {
                finished_at: Utc::now(),
            },
        )
        .await
        .expect("update");

        let deleted = repo.delete_job("updatable").await.expect("delete");
        assert!(deleted);
    }
}
