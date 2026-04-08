//! Gitoxide repository module
//!
//! Pure gitoxide implementation — no CLI spawning where possible.
//! Some operations (diff, status details) fall back to git CLI
//! where gix does not yet expose the required APIs.

pub mod branch;
pub mod commit;
pub mod diff;
pub mod merge;
pub mod rebase;
pub mod refs;
pub mod remote;
pub mod repository;
pub mod stash;
pub mod status;
pub mod tag;
pub mod worktree;
