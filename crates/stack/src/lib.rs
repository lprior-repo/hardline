#![allow(dead_code)]
#![allow(clippy::missing_errors_doc)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod commands;
pub mod domain;
pub mod engine;
pub mod error;
pub mod infrastructure;

pub use application::{
    GitHubClientTrait, MergeMethod, PrMergeStatus, StackRepository, VcsClientTrait,
};
pub use commands::{
    calculate_land_scope, calculate_merge_scope, wait_for_pr_ready, LandBranchInfo,
    MergeWhenReadyContext, MergeWhenReadyOptions, MergeWhenReadyScope, RemainingBranchInfo,
};
pub use domain::entities::{PrInfo, PrState, Stack, StackBranch};
<<<<<<< HEAD
<<<<<<< HEAD
pub use domain::land_status::LandStatus;
=======
>>>>>>> polecat/beta
=======
>>>>>>> polecat/theta
pub use domain::value_objects::{BranchName, CiCheckHistory, CiRunRecord, CiStatus};
pub use error::{Result, StackError};
