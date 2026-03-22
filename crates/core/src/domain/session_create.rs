//! Session creation module
//!
//! Provides validation logic for session creation following functional-rust patterns:
//! - Zero unwrap/panic/expect - all fallible via Result<T, E>
//! - Zero let mut - immutable by default
//! - Data → Calculations → Actions organization
//!
//! # Preconditions (P1-P7)
//!
//! | ID | Description | Type |
//! |----|-------------|------|
//! | P1 | `SessionName` not empty | Compile-time (`SessionName::parse`) |
//! | P2 | `SessionName` starts with letter | Compile-time (`SessionName::parse`) |
//! | P3 | `SessionName` alphanumeric/hyphen/underscore | Compile-time (`SessionName::parse`) |
//! | P4 | `SessionName` 1-63 chars | Compile-time (`SessionName::parse`) |
//! | P5 | Workspace path must exist | Runtime |
//! | P6 | Session name must be unique | Runtime |
//! | P7 | Max sessions limit | Runtime |
//!
//! # Postconditions (Q1-Q8)
//!
//! | ID | Description |
//! |----|-------------|
//! | Q1 | Session created with status Created |
//! | Q2 | Session.id is set correctly |
//! | Q3 | Session.name is set correctly |
//! | Q4 | Session.workspace_path is set correctly |
//! | Q5 | Session.branch is set correctly |
//! | Q6 | Session.created_at is set |
//! | Q7 | Session.updated_at is set |
//! | Q8 | Session.status is Created |

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// Re-export types from submodules
pub use session_create_creator::{create_session_entity, SessionCreator};
pub use session_create_errors::SessionCreateError;
pub use session_create_types::{SessionCreateInput, SessionCreateOutput, SessionLimits};
pub use session_create_validation::{
    validate_name_unique, validate_under_limit, validate_workspace_exists,
};

// Include tests
#[cfg(test)]
mod session_create_tests;
