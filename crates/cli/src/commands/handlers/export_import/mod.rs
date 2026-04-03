//! Export/Import command handler - Export and import session configurations.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): ExportOptions, ImportOptions, ExportResult, ImportResult,
//!   ExportedSession (inert, serializable)
//! - **Actions** (`actions.rs`): run_export, run_import (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace export                    # Export all sessions
//! scp workspace export my-session         # Export specific session
//! scp workspace export -o backup.json     # Export to file
//! scp workspace import backup.json        # Import sessions
//! scp workspace import backup.json --force  # Overwrite existing
//! ```

pub mod actions;
pub mod data;

pub use actions::{run_export, run_import};
pub use data::{ExportOptions, ImportOptions};
