//! Queue-internal error helpers.
//!
//! Public error types live in [`crate::error_queue`]. This module provides
//! convenience functions used by queue operation implementations.

use crate::error::Error;

pub(crate) fn lock_failed(context: &str, e: impl std::fmt::Display) -> Error {
    Error::invalid_state(format!("{}: {}", context, e))
}
