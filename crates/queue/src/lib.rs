#![allow(dead_code, unused_imports, unknown_lints)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod infrastructure;

pub use domain::{InMemoryQueueRepository, QueueRepository};
pub use domain::{Queue, QueueEntry, QueueStatus, MAX_PRIORITY};
pub use domain::{QueueEntryId, SessionName};
pub use domain::{ValidationError, ValidationResult};

pub use error::{MigrationError, MigrationResult};
pub use infrastructure::{
    rollback_migration, run_migrations, verify_migration, MigrationError as InfraMigrationError,
};
pub mod error;
