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
pub mod infrastructure;

pub use domain::entities::{PrInfo, PrState, Stack, StackBranch};
pub use domain::value_objects::BranchName;
pub use engine::restack::{
    build_all_restack_plans, build_restack_plan, calculate_depth, infer_scope, scope_branches,
    RestackPlan, RestackScope, RestackStep,
};
pub use engine::transactional_engine::{TransactionConfig, TransactionalStackOps};
pub use error::{Result, StackError};
