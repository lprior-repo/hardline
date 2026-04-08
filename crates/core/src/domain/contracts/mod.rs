//! # Design-by-Contract Annotations
//!
//! This module provides contract annotations for domain functions using the `dbc` crate (contracts).
//!
//! # Usage
//!
//! ```rust
//! use scp_core::domain::contracts::{ensures, requires};
//!
//! #[requires(x > 0, "x must be positive")]
//! #[ensures(ret > 0, "result must be positive")]
//! pub fn safe_div(x: i32, y: i32) -> i32 {
//!     x / y
//! }
//!
//! # fn main() { safe_div(4, 2); }
//! ```
//!
//! # Categories
//!
//! - **Preconditions** (`#[requires]`): Constraints on input arguments
//! - **Postconditions** (`#[ensures]`): Guarantees on return values
//! - **Invariants** (`#[invariant]`): Constraints on struct state

pub use dbc::ensures;
pub use dbc::invariant;
pub use dbc::requires;
