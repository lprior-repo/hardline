#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod error;
pub mod storage;

pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{Result, SnapshotError};
