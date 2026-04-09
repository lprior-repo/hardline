//! CI status command handler.
//!
//! Provides CI check status monitoring and watch functionality.
//! Ported from stax commands/ci.rs, adapted to hardline architecture.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): CiCheckOptions, CiWatchOptions, CiSubcommand
//! - **Actions** (`actions.rs`): run_ci_check, run_ci_watch (I/O operations)

pub mod actions;
pub mod data;

pub use actions::{run_ci_check, run_ci_watch};
pub use data::{CiCheckOptions, CiCheckOutput, CiSubcommand, CiWatchOptions};
