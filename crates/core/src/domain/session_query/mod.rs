//! Session list query and filtering
//!
//! Provides functional filtering and sorting for session lists using:
//! - Value objects for filter criteria
//! - Iterator pipelines with `itertools` and `tap::Pipe`
//! - Railway-oriented error handling with `Result<T, E>`
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionFilter` - value object for filter criteria
//! - `SessionSort` - sort field and direction
//! - `filter_sessions()` - pure function for filtering
//! - `sort_sessions()` - pure function for sorting
//! - `apply_query()` - compose filter + sort + paginate

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![cfg_attr(test, allow(clippy::todo))]
#![cfg_attr(test, allow(clippy::unimplemented))]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_clone)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod session_filter;
pub mod session_query_ops;
pub mod session_query_repo;
pub mod session_query_types;
pub mod session_sort;

// Tests module
#[cfg(test)]
mod session_query_tests;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use session_filter::SessionFilter;
pub use session_query_ops::{apply_query, filter_sessions, paginate_sessions, sort_sessions};
pub use session_query_repo::SessionRepositoryExt;
pub use session_query_types::SessionQuery;
pub use session_sort::{SessionSort, SessionSortField, SortDirection};
