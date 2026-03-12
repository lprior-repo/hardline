#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod storage;
pub mod error;

pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{SnapshotError, Result};
