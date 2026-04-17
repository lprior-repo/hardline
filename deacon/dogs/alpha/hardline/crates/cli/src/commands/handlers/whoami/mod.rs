//! Whoami command handler - Show current agent identity.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): WhoamiOptions, WhoamiOutput (inert, serializable)
//! - **Actions** (`actions.rs`): run_whoami, build_identity (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp whoami                  # Show simple identity
//! scp whoami --json           # Output as JSON
//! ```

pub mod actions;
pub mod data;

pub use actions::{build_identity, run_whoami};
pub use data::{WhoamiOptions, WhoamiOutput};
