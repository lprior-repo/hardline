#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;

pub use domain::{Queue, QueueEntry, QueueStatus, MAX_PRIORITY};
pub use domain::{QueueEntryId, SessionName};
pub use domain::{ValidationError, ValidationResult};
pub use domain::{QueueRepository, InMemoryQueueRepository};
