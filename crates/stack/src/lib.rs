#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod github;
pub mod engine;
pub mod error;

pub use domain::entities::{Stack, StackBranch, BranchName, PrInfo, PrState};
pub use error::{StackError, Result};
