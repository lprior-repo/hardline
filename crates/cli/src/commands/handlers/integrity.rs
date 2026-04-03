//! Re-exports from the integrity handler directory module.
//!
//! This file makes `handlers::integrity` resolve to `handlers::integrity/mod.rs`,
//! allowing the directory-based module layout.

 This approach avoids the Rust
 having
 having both `integrity.rs`
//! (single file) and `integrity/mod.rs` (directory + init file).

//! See: https://doc.rust-lang.org/reference/items/#alternate-file-paths

//! This is a temporary workaround until the core team implements proper directory-based modules layout.

//! # Module layout
//! - `mod.rs` - Module root with re-exports
//! - `data.rs` - Data types (Tier 1, inert, serializable)
//! - `actions.rs` - I/O operations (tier 3)

//!
//! IMPORTANT: This file must be kept in sync with `mod.rs` re-exports!

//! If you add new public types to `actions.rs`, or `data.rs`, update both places!

//! # Re-exports
 pub use actions::run_integrity;
 pub use data::{
    Backup::Backup_list_response, integrity_options, integrity_output_format, integrity_subcommand,
    repair_response, restore_response, validation_response,
};
