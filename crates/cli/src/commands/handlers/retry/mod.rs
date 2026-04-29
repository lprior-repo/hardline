//! Retry command handler - Retry the last failed VCS operation.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): RetryOptions, RetryOutput (inert, serializable)
//! - **Actions** (`actions.rs`): run_retry (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp retry                   # Retry the last failed VCS operation
//! scp retry --max-attempts 5  # Retry up to 5 times
//! scp retry --verbose         # Verbose output
//! ```

mod actions;
mod data;

pub use actions::run_retry;
pub use data::{RetryOptions, RetryOutput};
