<<<<<<< HEAD
pub mod storage;

=======
#![allow(clippy::module_inception)]
pub mod receipt_store;
pub mod storage;

pub use receipt_store::ReceiptStore;
>>>>>>> polecat/beta
pub use storage::SnapshotStore;
