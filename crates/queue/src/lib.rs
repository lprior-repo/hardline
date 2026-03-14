#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod infrastructure;

pub use domain::{Queue, QueueEntry, QueueStatus, MAX_PRIORITY};
pub use domain::{QueueEntryId, SessionName};
pub use domain::{ValidationError, ValidationResult};
pub use domain::{QueueRepository, InMemoryQueueRepository};

pub use infrastructure::{run_migrations, verify_migration, rollback_migration, MigrationError};
