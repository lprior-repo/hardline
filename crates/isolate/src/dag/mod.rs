//! Branch DAG — directed acyclic graph of branch relationships.
//!
//! Organized as Data / Calc / Actions:
//! - **Data** (`types`, `data`): inert value types and the `BranchDag` struct
//! - **Calc** (`calc`): pure operations — add/remove, traversal, topological sort
//! - **Actions**: none (pure domain, no I/O)

mod calc;
mod data;
#[cfg(test)]
mod tests;
pub mod types;

pub use data::BranchDag;
pub use types::{BranchId, DagError};
