//! Bookmark command handler - manage Git branch bookmarks.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): BookmarkOptions, BookmarkSubcommand, BookmarkOutput,
//!   BookmarkInfo, response types (inert, serializable)
//! - **Actions** (`actions.rs`): run_bookmark, run_create, run_list, run_delete,
//!   run_track (I/O operations delegating to Git)
//!
//! # CLI Usage
//!
//! ```text
//! scp bookmark create feature-auth         # Create a new bookmark
//! scp bookmark create feature-auth --push  # Create and push to remote
//! scp bookmark list                        # List bookmarks
//! scp bookmark list --all                  # List all including remotes
//! scp bookmark delete old-feature          # Delete a bookmark
//! scp bookmark track main                  # Track remote bookmark on origin
//! scp bookmark track main --remote upstream # Track on specific remote
//! ```

pub mod actions;
pub mod data;

pub use actions::run_bookmark;
pub use data::{
    BookmarkCreateOutput, BookmarkDeleteOutput, BookmarkInfo, BookmarkListOutput, BookmarkOptions,
    BookmarkOutput, BookmarkSubcommand, BookmarkTrackOutput, parse_branch_list,
};
