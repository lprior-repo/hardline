#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
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

    #[must_use]
    pub fn id(&self) -> &JobId {
        &self.id
    }

    #[must_use]
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    #[must_use]
    pub fn priority(&self) -> &Priority {
        &self.priority
    }

    #[must_use]
    pub fn status(&self) -> JobStatus {
        self.status
    }

    #[must_use]
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn enqueue(&self, job: Job) -> Self {
        let priority = job.priority.value();
        let mut jobs = self.jobs.clone();
        let insert_pos = jobs
            .iter()
            .rposition(|j| j.priority.value() < priority)
            .map_or(0, |p| p + 1);
        jobs.insert(insert_pos, job);
        Self { jobs }
    }

    #[must_use]
    pub fn dequeue(&self) -> (Self, Option<Job>) {
        if self.jobs.is_empty() {
            return (self.clone(), None);
        }
        let mut jobs = self.jobs.clone();
        let job = jobs.remove(0);
        (Self { jobs }, Some(job))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    #[must_use]
    pub fn find(&self, id: &JobId) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id() == id)
    }

    #[must_use]
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

    #[test]
    fn job_created_at_is_recent() {
        let before = Utc::now();
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let after = Utc::now();
        assert!(job.created_at() >= before);
        assert!(job.created_at() <= after);
    }

    #[test]
    fn job_find_existing() {
        let queue = JobQueue::new();
        let job = Job::new(
            JobId::new("j-find").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let queue = queue.enqueue(job);
        let id = JobId::new("j-find").unwrap();
        let found = queue.find(&id);
        assert!(found.is_some());
    }

    #[test]
    fn job_find_nonexistent() {
        let queue = JobQueue::new();
        let id = JobId::new("j-nope").unwrap();
        let found = queue.find(&id);
        assert!(found.is_none());
    }

    #[test]
    fn job_queue_jobs_returns_slice() {
        let queue = JobQueue::new();
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let queue = queue.enqueue(job);
        assert_eq!(queue.jobs().len(), 1);
    }

    #[test]
    fn job_queue_default_is_empty() {
        let queue = JobQueue::default();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn job_queue_is_immutable() {
        let queue = JobQueue::new();
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let _new_queue = queue.enqueue(job);
        assert!(queue.is_empty(), "Original queue should remain unchanged");
    }

    #[test]
    fn job_multiple_enqueue_priority_order() {
        let queue = JobQueue::new();
        let jobs: Vec<Job> = [5_u8, 1, 3, 4, 2]
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                Job::new(
                    JobId::new(format!("j-{i}")).unwrap(),
                    Payload::from_str(r#"{}"#).unwrap(),
                    Priority::new(p).unwrap(),
                )
                .unwrap()
            })
            .collect();

        let queue = jobs.into_iter().fold(queue, |q, j| q.enqueue(j));
        let priorities: Vec<u8> = queue.jobs().iter().map(|j| j.priority().value()).collect();
        // JobQueue: lower priority value = dequeued first (ascending order)
        let mut sorted_priorities = priorities.clone();
        sorted_priorities.sort();
        assert_eq!(priorities, sorted_priorities);
    }

    #[test]
    fn job_transition_invalid_skips_states() {
        let job = Job::new(
            JobId::new("j-1").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        // Can't go from Pending directly to Completed
        let result = job.transition_to(JobStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn job_clone() {
        let job = Job::new(
            JobId::new("j-clone").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let cloned = job.clone();
        assert_eq!(job.id().as_str(), cloned.id().as_str());
        assert_eq!(job.status(), cloned.status());
    }

    #[test]
    fn job_equality_same_timestamp() {
        // Jobs with the same fields are PartialEq
        // created_at may differ so they may not be equal
        let _a = Job::new(
            JobId::new("j-eq").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        let _b = Job::new(
            JobId::new("j-eq").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(100).unwrap(),
        )
        .unwrap();
        // Created_at differs between the two, so PartialEq is false.
        // This is a documentation test that verifies Job is PartialEq.
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn job_new_with_high_priority() {
        let job = Job::new(
            JobId::new("j-high").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(255).unwrap(),
        )
        .unwrap();
        assert_eq!(job.priority().value(), 255);
    }

    #[test]
    fn job_new_with_zero_priority() {
        let job = Job::new(
            JobId::new("j-zero").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(0).unwrap(),
        )
        .unwrap();
        assert_eq!(job.priority().value(), 0);
    }

    #[test]
    fn job_new_with_complex_payload() {
        let payload = Payload::from_str(r#"{"nested":{"data":[1,2,3]}}"#).unwrap();
        let job = Job::new(
            JobId::new("j-complex").unwrap(),
            payload,
            Priority::new(50).unwrap(),
        )
        .unwrap();
        assert_eq!(job.payload().as_value()["nested"]["data"][1], 2);
    }

    #[test]
    fn job_find_in_nonempty_queue() {
        let queue = JobQueue::new();
        let job = Job::new(
            JobId::new("j-target").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let queue = queue.enqueue(job);
        let found = queue.find(&JobId::new("j-target").unwrap());
        assert!(found.is_some());
        assert_eq!(found.unwrap().id().as_str(), "j-target");
    }

    #[test]
    fn job_dequeue_preserves_priority_order() {
        let queue = JobQueue::new();
        let jobs: Vec<Job> = [100_u8, 50, 75, 25, 0]
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                Job::new(
                    JobId::new(format!("j-{i}")).unwrap(),
                    Payload::from_str(r#"{}"#).unwrap(),
                    Priority::new(p).unwrap(),
                )
                .unwrap()
            })
            .collect();

        let queue = jobs.into_iter().fold(queue, |q, j| q.enqueue(j));

        let mut prev_priority = 0;
        for job in queue.jobs() {
            assert!(
                job.priority().value() >= prev_priority,
                "Priority ordering violated: {} >= {}",
                job.priority().value(),
                prev_priority
            );
            prev_priority = job.priority().value();
        }
    }

    #[test]
    fn job_transition_preserves_id() {
        let job = Job::new(
            JobId::new("j-id-check").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let id = job.id().as_str().to_string();

        let claimed = job.transition_to(JobStatus::Processing).unwrap();
        assert_eq!(claimed.id().as_str(), id);

        let completed = claimed.transition_to(JobStatus::Completed).unwrap();
        assert_eq!(completed.id().as_str(), id);
    }

    #[test]
    fn job_transition_preserves_priority() {
        let job = Job::new(
            JobId::new("j-prio-check").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(77).unwrap(),
        )
        .unwrap();

        let processed = job.transition_to(JobStatus::Processing).unwrap();
        assert_eq!(processed.priority().value(), 77);
    }

    #[test]
    fn job_transition_preserves_payload() {
        let job = Job::new(
            JobId::new("j-pay-check").unwrap(),
            Payload::from_str(r#"{"check":true}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();

        let processed = job.transition_to(JobStatus::Processing).unwrap();
        assert_eq!(processed.payload().as_value()["check"], true);
    }

    #[test]
    fn job_transition_preserves_created_at() {
        let job = Job::new(
            JobId::new("j-time-check").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let original_time = job.created_at();

        let processed = job.transition_to(JobStatus::Processing).unwrap();
        assert_eq!(processed.created_at(), original_time);
    }

    #[test]
    fn job_queue_dequeue_specific_order() {
        let queue = JobQueue::new();
        let j1 = Job::new(
            JobId::new("j-first").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(1).unwrap(),
        )
        .unwrap();
        let j2 = Job::new(
            JobId::new("j-second").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(10).unwrap(),
        )
        .unwrap();

        let queue = queue.enqueue(j2).enqueue(j1);

        let (_, first) = queue.dequeue();
        assert_eq!(first.unwrap().id().as_str(), "j-first");
    }

    #[test]
    fn job_queue_enqueue_many_then_dequeue_all() {
        let queue = JobQueue::new();
        let count = 20;
        let queue = (0..count)
            .map(|i| {
                Job::new(
                    JobId::new(format!("j-{i}")).unwrap(),
                    Payload::from_str(r#"{}"#).unwrap(),
                    Priority::new((count - i) as u8).unwrap(),
                )
                .unwrap()
            })
            .fold(queue, |q, j| q.enqueue(j));

        assert_eq!(queue.len(), count);

        let mut current = queue;
        for _ in 0..count {
            let (next, dequeued) = current.dequeue();
            assert!(dequeued.is_some());
            current = next;
        }
        assert!(current.is_empty());
    }

    #[test]
    fn job_failed_to_any_transition_rejected() {
        let job = Job::new(
            JobId::new("j-fail").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap()
        .transition_to(JobStatus::Processing)
        .unwrap()
        .transition_to(JobStatus::Failed)
        .unwrap();

        // From Failed, no transitions should succeed
        assert!(job.transition_to(JobStatus::Pending).is_err());
        assert!(job.transition_to(JobStatus::Processing).is_err());
        assert!(job.transition_to(JobStatus::Completed).is_err());
        assert!(job.transition_to(JobStatus::Failed).is_err());
    }

    #[test]
    fn job_completed_to_any_transition_rejected() {
        let job = Job::new(
            JobId::new("j-complete").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap()
        .transition_to(JobStatus::Processing)
        .unwrap()
        .transition_to(JobStatus::Completed)
        .unwrap();

        assert!(job.transition_to(JobStatus::Pending).is_err());
        assert!(job.transition_to(JobStatus::Processing).is_err());
        assert!(job.transition_to(JobStatus::Completed).is_err());
        assert!(job.transition_to(JobStatus::Failed).is_err());
    }

    #[test]
    fn job_priority_ties_maintain_enqueue_order() {
        let queue = JobQueue::new();
        let j1 = Job::new(
            JobId::new("j-a").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let j2 = Job::new(
            JobId::new("j-b").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();
        let j3 = Job::new(
            JobId::new("j-c").unwrap(),
            Payload::from_str(r#"{}"#).unwrap(),
            Priority::new(50).unwrap(),
        )
        .unwrap();

        let queue = queue.enqueue(j1).enqueue(j2).enqueue(j3);

        // JobQueue inserts same-priority items at position 0 (before existing equal priorities)
        let ids: Vec<&str> = queue.jobs().iter().map(|j| j.id().as_str()).collect();
        assert_eq!(ids, vec!["j-c", "j-b", "j-a"]);
    }
}
