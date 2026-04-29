#![allow(clippy::module_inception)]
#![allow(dead_code)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod error;
pub mod storage;

pub use domain::{
    receipt::{LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry},
    receipt_calc::{can_redo, can_undo, has_remote_changes, modified_branch_count},
    snapshot::{Snapshot, SnapshotId, SnapshotType},
};
pub use error::{Result, SnapshotError};
pub use storage::ReceiptStore;
