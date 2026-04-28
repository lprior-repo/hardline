#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

// Re-export modules for public API
// Re-export parsing functions for tests
#[cfg(test)]
pub use parsing::{parse_bead_row, parse_datetime, parse_status};
#[cfg(test)]
pub use query::enable_wal_mode;
pub use query::query_beads;
pub use schema::ensure_schema;
pub use write::{delete_bead, insert_bead, update_bead};

// Module declarations
pub(crate) mod parsing;
mod query;
mod schema;
mod validation;
mod write;

#[cfg(test)]
mod delete_tests;
#[cfg(test)]
mod insert_tests;
#[cfg(test)]
mod update_tests;
