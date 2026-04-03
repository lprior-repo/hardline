//! Rename command handler - Rename a workspace/session.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): RenameOptions, RenameOutput (inert, serializable)
//! - **Actions** (`actions.rs`): run_rename (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace rename old-name new-name        # Rename session
//! scp workspace rename old-name new-name --dry-run  # Preview
//! ```

pub mod actions;
pub mod data;

pub use actions::run_rename;
pub use data::{RenameOptions, RenameOutput};
