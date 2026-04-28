//! # Design-by-Contract Annotations
//!
//! This module provides contract annotations for domain functions using the `dbc` crate
//! (contracts).
//!
//! # Usage
//!
//! ```rust
//! use scp_core::domain::contracts::requires;
//!
//! #[requires(x > 0, "x must be positive")]
//! pub fn safe_div(x: i32, y: i32) -> i32 {
//!     x / y
//! }
//! ```
//!
//! # Categories
//!
//! - **Preconditions** (`#[requires]`): Constraints on input arguments
//! - **Postconditions** (`#[ensures]`): Guarantees on return values
//! - **Invariants** (`#[invariant]`): Constraints on struct state

pub use dbc::{ensures, invariant, requires};
