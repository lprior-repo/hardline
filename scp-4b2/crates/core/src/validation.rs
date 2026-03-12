//! Domain validation layer - Pure functions for business rule enforcement
//!
//! This module provides:
//! - `domain` - Pure validation functions with no I/O
//! - `infrastructure` - I/O validation functions for filesystem checks

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod infrastructure;

pub use crate::Error as IdentifierError;
