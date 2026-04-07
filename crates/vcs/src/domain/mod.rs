//! VCS Domain Layer - Pure domain types and traits
//!
//! This module contains:
//! - Domain entities (Commit, Branch, Workspace, Ops types)
//! - VCS trait definitions
//! - Value objects
//!
//! No I/O, no external dependencies - pure business logic.

pub mod entities;
pub mod traits;
pub mod value_objects;

pub use entities::{Branch, Commit, Workspace};
pub use entities::ops::{
    LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry,
};
pub use traits::VcsBackend;
pub use value_objects::{VcsStatus, VcsType};
