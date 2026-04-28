//! Job processor with priority-based execution
//!
//! Provides:
//! - JobProcessor: Main processor with concurrency control
//! - JobProcessorConfig: Configuration for the processor

use std::time::Duration;

use chrono::Utc;
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::queue::{
    repository::JobRepository,
    types::{Job, JobOutcome, JobResult, JobState},
};

#[derive(Debug, Clone, Error)]
pub enum QueueError {
    #[error("No repository configured")]
    NoRepository,

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Invalid job state: {0}")]
    InvalidJobState(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Shutdown requested")]
    ShutdownRequested,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

pub type QueueResult<T> = std::result::Result<T, QueueError>;

#[derive(Debug, Clone)]
pub struct JobProcessorConfig {
    pub poll_interval: Duration,
    pub concurrency_limit: usize,
    pub max_retries: u32,
}

impl JobProcessorConfig {
    pub fn validate(&self) -> QueueResult<()> {
        if self.poll_interval.is_zero() {
            return Err(QueueError::InvalidConfiguration(
                "Poll interval must be non-zero".into(),
            ));
        }
        if self.concurrency_limit == 0 {
            return Err(QueueError::InvalidConfiguration(
                "Concurrency limit must be at least 1".into(),
            ));
        }
        Ok(())
    }
}

pub struct JobProcessor<R: JobRepository> {
    repository: R,
    config: JobProcessorConfig,
    semaphore: Semaphore,
    running_count: std::sync::atomic::AtomicUsize,
}

impl<R: JobRepository> JobProcessor<R> {
    pub fn new(repository: R, config: JobProcessorConfig) -> QueueResult<Self> {
        config.validate()?;
        let concurrency_limit = config.concurrency_limit;
        Ok(Self {
            repository,
            config,
            semaphore: Semaphore::new(concurrency_limit),
            running_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    #[must_use]
    pub fn config(&self) -> &JobProcessorConfig {
        &self.config
    }

    #[must_use]
    pub fn running_jobs(&self) -> usize {
        self.running_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn poll_once(&self) -> QueueResult<Option<Job>> {
        let jobs = self.repository.poll_pending_jobs(1).await?;
        Ok(jobs.into_iter().next())
    }

    async fn execute_job(&self, mut job: Job) -> QueueResult<JobResult> {
        let started_at = Utc::now();
        job.state = JobState::Running { started_at };

        self.repository
            .update_job_state(&job.id, job.state.clone())
            .await?;

        let exec_start = std::time::Instant::now();
        let outcome = self.execute_job_body(&job).await;
        let execution_time_ms = exec_start.elapsed().as_millis() as u64;

        let final_state = match &outcome {
            Ok(_) => JobState::Completed {
                finished_at: Utc::now(),
            },
            Err(e) => JobState::Failed {
                error: e.to_string(),
                failed_at: Utc::now(),
            },
        };

        self.repository
            .update_job_state(&job.id, final_state)
            .await?;

        Ok(JobResult {
            job_id: job.id,
            outcome: outcome?,
            execution_time_ms,
        })
    }

    async fn execute_job_body(&self, job: &Job) -> QueueResult<JobOutcome> {
        match &job.payload {
            super::types::JobPayload::Pipeline { spec_path } => {
                info!(
                    "Executing pipeline job: {} with spec: {}",
                    job.id, spec_path
                );
                Ok(JobOutcome::Success)
            }
            super::types::JobPayload::Task { command } => {
                info!("Executing task job: {} with command: {}", job.id, command);
                Ok(JobOutcome::Success)
            }
            super::types::JobPayload::Custom { data } => {
                info!("Executing custom job: {} with data: {:?}", job.id, data);
                Ok(JobOutcome::Success)
            }
        }
    }

    pub async fn run(
        &self,
        mut stop_signal: tokio::sync::broadcast::Receiver<()>,
    ) -> QueueResult<()> {
        info!(
            "Starting job processor with poll_interval={:?}, concurrency_limit={}",
            self.config.poll_interval, self.config.concurrency_limit
        );

        loop {
            tokio::select! {
                _ = stop_signal.recv() => {
                    info!("Shutdown signal received, stopping job processor");
                    break;
                }
                _ = tokio::time::sleep(self.config.poll_interval) => {
                    self.process_cycle().await?;
                }
            }
        }

        info!(
            "Job processor stopped. Final running jobs: {}",
            self.running_jobs()
        );
        Ok(())
    }

    async fn process_cycle(&self) -> QueueResult<()> {
        let Some(job) = self.poll_once().await? else {
            debug!("No pending jobs found");
            return Ok(());
        };

        let _permit = self.semaphore.acquire().await.map_err(|e| {
            QueueError::ExecutionFailed(format!("Failed to acquire semaphore: {}", e))
        })?;

        let running = self
            .running_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!("Starting job {}, running count: {}", job.id, running + 1);

        let result = self.execute_job(job).await;

        self.running_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

        if let Err(e) = result {
            error!("Job execution failed: {}", e);
        }

        Ok(())
    }
}
