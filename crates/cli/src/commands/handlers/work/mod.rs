//! Work command handler - Unified workflow start for AI agents.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): WorkOptions, WorkOutput, EnvVar (inert, serializable)
//! - **Actions** (`actions.rs`): run_work, output_existing_workspace (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp work my-session                    # Create and enter workspace
//! scp work my-session --bead scp-123     # Associate with a bead
//! scp work my-session --no-agent         # Skip agent registration
//! scp work my-session --idempotent       # Succeed if session exists
//! scp work my-session --dry-run          # Preview without creating
//! ```

pub mod actions;
pub mod data;

pub use actions::run_work;
pub use data::{EnvVar, WorkMode, WorkOptions, WorkOutput};
