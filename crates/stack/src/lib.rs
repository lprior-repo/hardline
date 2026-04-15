#![allow(dead_code)]
#![allow(clippy::missing_errors_doc)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod engine;
pub mod error;
pub mod github;

pub use domain::entities::{PrInfo, PrState, Stack, StackBranch};
pub use domain::value_objects::{BranchName, CiCheckHistory, CiRunRecord, CiStatus};
pub use error::{Result, StackError};
