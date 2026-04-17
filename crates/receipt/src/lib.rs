#![allow(clippy::module_inception)]
#![allow(dead_code)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod error;
pub mod storage;

pub use domain::receipt::{
    LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry,
};
pub use domain::receipt_calc::{can_redo, can_undo, has_remote_changes, modified_branch_count};
pub use error::{ReceiptError, Result};
pub use storage::ReceiptStore;
