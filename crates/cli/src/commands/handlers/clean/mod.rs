//! Clean command handler - Remove stale workspace sessions.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): CleanOptions, CleanOutput (inert, serializable)
//! - **Actions** (`actions.rs`): run_clean (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! hardline clean                   # Remove stale sessions
//! hardline clean --dry-run         # List stale sessions without removing
//! hardline clean --verbose         # Show detailed output
//! hardline clean --force           # Skip confirmation (non-interactive)
//! ```

pub mod actions;
pub mod data;

pub use actions::run_clean;
pub use data::{CleanOptions, CleanOutput};
