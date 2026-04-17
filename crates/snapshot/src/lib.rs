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

<<<<<<< HEAD
<<<<<<< HEAD
pub use application::{CleanupReport, SnapshotService};
pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{Result, SnapshotError};
pub use storage::SnapshotStore;

// Re-export receipt types for backward compatibility with downstream crates.
// These will be removed in a future version — consumers should depend on scp-receipt directly.
#[deprecated(
    since = "0.5.0",
    note = "Use scp-receipt crate directly instead"
)]
pub use scp_receipt::{
    can_redo, can_undo, has_remote_changes, modified_branch_count, LocalRefEntry, OpError,
    OpKind, OpReceipt, OpStatus, PlanSummary, ReceiptError, ReceiptStore, RemoteRefEntry,
};
=======
pub use domain::receipt::{
    LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry,
};
pub use domain::receipt_calc::{can_redo, can_undo, has_remote_changes, modified_branch_count};
pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{Result, SnapshotError};
pub use storage::ReceiptStore;
>>>>>>> polecat/beta
=======
pub use domain::receipt::{
    LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry,
};
pub use domain::receipt_calc::{can_redo, can_undo, has_remote_changes, modified_branch_count};
pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{Result, SnapshotError};
pub use storage::ReceiptStore;
>>>>>>> polecat/theta
