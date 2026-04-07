//! Workspace commands - main module with re-exports

pub mod branches;
pub mod commits;
pub mod completion;
pub mod lifecycle;
pub mod merge;
pub mod navigation;
pub mod operations;
pub mod types;
pub mod validators;

#[cfg(test)]
mod adversarial_tests;

pub use branches::*;
pub use commits::*;
pub use completion::*;
pub use lifecycle::*;
pub use merge::*;
pub use types::SyncOption;
