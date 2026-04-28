//! Checkpoint command handler - save and restore full session state snapshots.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): CheckpointOptions, CheckpointAction, CheckpointOutput, CheckpointInfo
//!   (inert, serializable)
//! - **Actions** (`actions.rs`): run_checkpoint, run_create, run_restore, run_list (I/O operations)
//! - **Handlers** (`handlers.rs`): handle_checkpoint, handle_undo, handle_revert, handle_recover,
//!   handle_rollback (CLI entry points adapted from isolate)
//!
//! # CLI Usage
//!
//! ```text
//! scp checkpoint create                         # Create checkpoint
//! scp checkpoint create -d "before refactor"    # Create with description
//! scp checkpoint restore chk-abc123             # Restore a checkpoint
//! scp checkpoint list                           # List all checkpoints
//! ```

pub mod actions;
pub mod data;
pub mod handlers;

pub use actions::run_checkpoint;
pub use data::{
    generate_checkpoint_id, CheckpointAction, CheckpointInfo, CheckpointOptions, CheckpointOutput,
    OutputFormat,
};
pub use handlers::{
    handle_checkpoint, handle_recover, handle_revert, handle_rollback, handle_undo,
};
