//! Bookmark command handler - manage Git branch bookmarks.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): BookmarkOptions, BookmarkSubcommand, BookmarkOutput, BookmarkInfo,
//!   response types (inert, serializable)
//! - **Actions** (`actions.rs`): run_bookmark, run_create, run_list, run_delete, run_track (I/O
//!   operations delegating to Git)
//! - **Handler** (`mod.rs`): handle_bookmark (CLI adapter parsing subcommands)
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

use clap::ArgMatches;
use scp_core::Result;

pub mod actions;
pub mod data;

pub use actions::run_bookmark;
pub use data::{
    parse_branch_list, BookmarkCreateOutput, BookmarkDeleteOutput, BookmarkInfo,
    BookmarkListOutput, BookmarkOptions, BookmarkOutput, BookmarkSubcommand, BookmarkTrackOutput,
};

// Re-export get_format for handler use
use super::json_format::get_format;

/// Handle bookmark subcommands: list, create, delete, track.
///
/// Ported from ~/src/isolate/crates/isolate/src/cli/handlers/bookmark.rs
///
/// # Errors
///
/// Returns errors from the underlying `run_bookmark` function for invalid
/// bookmark names, not found bookmarks, or Git command failures.
#[allow(clippy::unnecessary_wraps)]
pub fn handle_bookmark(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", list_m)) => {
            let show_all = list_m.get_flag("all");
            let _format = get_format(list_m);
            let opts = BookmarkOptions {
                subcommand: BookmarkSubcommand::List { show_all },
            };
            run_bookmark(&opts).map(|_output| ())
        }
        Some(("create", create_m)) => {
            let name = create_m
                .get_one::<String>("name")
                .ok_or_else(|| scp_core::Error::validation_error("Bookmark name is required"))?
                .clone();
            let push = create_m.get_flag("push");
            let _format = get_format(create_m);
            let opts = BookmarkOptions {
                subcommand: BookmarkSubcommand::Create { name, push },
            };
            run_bookmark(&opts).map(|_output| ())
        }
        Some(("delete", delete_m)) => {
            let name = delete_m
                .get_one::<String>("name")
                .ok_or_else(|| scp_core::Error::validation_error("Bookmark name is required"))?
                .clone();
            let _format = get_format(delete_m);
            let opts = BookmarkOptions {
                subcommand: BookmarkSubcommand::Delete { name },
            };
            run_bookmark(&opts).map(|_output| ())
        }
        Some(("track", track_m)) => {
            let name = track_m
                .get_one::<String>("name")
                .ok_or_else(|| scp_core::Error::validation_error("Bookmark name is required"))?
                .clone();
            let remote = track_m.get_one::<String>("remote").cloned();
            let _format = get_format(track_m);
            let opts = BookmarkOptions {
                subcommand: BookmarkSubcommand::Track { name, remote },
            };
            run_bookmark(&opts).map(|_output| ())
        }
        _ => Err(scp_core::Error::validation_error(
            "Subcommand required: list, create, delete, or track",
        )),
    }
}
