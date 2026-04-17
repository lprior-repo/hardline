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

    pub fn add_job(&self, job: Job) -> QueueResult<()> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|e| QueueError::Repository(format!("Failed to acquire write lock: {}", e)))?;
        jobs.push(job);
        Ok(())
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
}

pub fn sort_jobs_by_priority(jobs: &mut [Job]) {
    jobs.sort_by_key(|j| j.priority);
}
