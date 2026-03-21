#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

pub use super::job_id::{JobCreationError, JobId, QueueId};
pub use super::job_priority::Priority;
pub use super::job_status::{JobStatus, QueueError};
pub use super::payload::Payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    id: JobId,
    payload: Payload,
    priority: Priority,
    status: JobStatus,
    created_at: DateTime<Utc>,
}

impl Job {
    pub fn new(id: JobId, payload: Payload, priority: Priority) -> Result<Self, JobCreationError> {
        Ok(Self {
            id,
            payload,
            priority,
            status: JobStatus::Pending,
            created_at: Utc::now(),
        })
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn priority(&self) -> &Priority {
        &self.priority
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn transition_to(&self, new_status: JobStatus) -> Result<Job, QueueError> {
        self.status.transition(new_status).map(|status| Job {
            id: self.id.clone(),
            payload: self.payload.clone(),
            priority: self.priority,
            status,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobQueue {
    jobs: Vec<Job>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&self, job: Job) -> Self {
        let priority = job.priority.value();
        let mut jobs = self.jobs.clone();
        let insert_pos = jobs
            .iter()
            .rposition(|j| j.priority.value() < priority)
            .map(|p| p + 1)
            .unwrap_or(0);
        jobs.insert(insert_pos, job);
        Self { jobs }
    }

    pub fn dequeue(&self) -> (Self, Option<Job>) {
        if self.jobs.is_empty() {
            return (self.clone(), None);
        }
        let mut jobs = self.jobs.clone();
        let job = jobs.remove(0);
        (Self { jobs }, Some(job))
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn find(&self, id: &JobId) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id() == id)
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_creation_with_valid_inputs() {
        let id = JobId::new("j-1").unwrap();
        let payload = Payload::from_str(r#"{"key":"value"}"#).unwrap();
        let priority = Priority::new(100).unwrap();
        let job = Job::new(id, payload, priority);
        assert!(job.is_ok());
        let job = job.unwrap();
        assert_eq!(job.id().as_str(), "j-1");
        assert_eq!(job.status(), JobStatus::Pending);
    }

    #[test]
    fn job_transition_to_processing() {
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let job = job.transition_to(JobStatus::Processing).unwrap();
        assert_eq!(job.status(), JobStatus::Processing);
    }

    #[test]
    fn job_transition_to_completed() {
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let job = job.transition_to(JobStatus::Processing).unwrap();
        let job = job.transition_to(JobStatus::Completed).unwrap();
        assert_eq!(job.status(), JobStatus::Completed);
    }

    #[test]
    fn job_transition_to_failed() {
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let job = job.transition_to(JobStatus::Processing).unwrap();
        let job = job.transition_to(JobStatus::Failed).unwrap();
        assert_eq!(job.status(), JobStatus::Failed);
    }

    #[test]
    fn job_queue_enqueue_and_dequeue() {
        let queue = JobQueue::new();
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let queue = queue.enqueue(job);
        assert_eq!(queue.len(), 1);
        let (queue, dequeued) = queue.dequeue();
        assert!(dequeued.is_some());
        assert!(queue.is_empty());
    }

    #[test]
    fn job_queue_dequeue_empty_returns_none() {
        let queue = JobQueue::new();
        let (queue, dequeued) = queue.dequeue();
        assert!(dequeued.is_none());
        assert!(queue.is_empty());
    }

    #[test]
    fn job_queue_priority_ordering() {
        let queue = JobQueue::new();
        let job1 = Job::new(
            JobId::new("j-high").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(10).unwrap(),
        )
        .unwrap();
        let job2 = Job::new(
            JobId::new("j-low").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let queue = queue.enqueue(job2);
        let queue = queue.enqueue(job1);
        let (_, dequeued) = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id().as_str(), "j-high");
    }
}
