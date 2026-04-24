use crate::domain::value_objects::Priority;
use crate::error::QueueError;

use super::entity::{QueueEntry, Pending};

pub struct QueueEntryBuilder {
    session_name: Option<String>,
    bead_id: Option<String>,
    priority: Priority,
}

impl QueueEntryBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_name: None,
            bead_id: None,
            priority: Priority::default(),
        }
    }

    #[must_use]
    pub fn with_session(mut self, session: &str) -> Self {
        self.session_name = Some(session.to_string());
        self
    }

    #[must_use]
    pub fn with_bead(mut self, bead_id: &str) -> Self {
        self.bead_id = Some(bead_id.to_string());
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_high_priority(mut self) -> Self {
        self.priority = Priority::high();
        self
    }

    #[must_use]
    pub fn with_low_priority(mut self) -> Self {
        self.priority = Priority::low();
        self
    }

    #[must_use]
    pub fn with_critical_priority(mut self) -> Self {
        self.priority = Priority::critical();
        self
    }

    pub fn enqueue(self) -> Result<QueueEntry<Pending>, QueueError> {
        let session = self
            .session_name
            .ok_or_else(|| QueueError::InvalidQueueEntryId("session name required".into()))?;
        QueueEntry::enqueue(session, self.bead_id, self.priority)
    }
}

impl Default for QueueEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub trait QueueDsl {
    fn enqueue_session(&mut self, session_name: &str) -> &mut Self;
    fn with_high_priority(&mut self) -> &mut Self;
    fn with_low_priority(&mut self) -> &mut Self;
    fn with_critical_priority(&mut self) -> &mut Self;
    fn execute(&mut self) -> Result<QueueEntry<Pending>, QueueError>;
}

impl QueueDsl for QueueEntryBuilder {
    fn enqueue_session(&mut self, session_name: &str) -> &mut Self {
        self.session_name = Some(session_name.to_string());
        self
    }

    fn with_high_priority(&mut self) -> &mut Self {
        self.priority = Priority::high();
        self
    }

    fn with_low_priority(&mut self) -> &mut Self {
        self.priority = Priority::low();
        self
    }

    fn with_critical_priority(&mut self) -> &mut Self {
        self.priority = Priority::critical();
        self
    }

    fn execute(&mut self) -> Result<QueueEntry<Pending>, QueueError> {
        let session = self
            .session_name
            .take()
            .ok_or_else(|| QueueError::InvalidQueueEntryId("session name required".into()))?;
        QueueEntry::enqueue(session, self.bead_id.clone(), self.priority)
    }
}
