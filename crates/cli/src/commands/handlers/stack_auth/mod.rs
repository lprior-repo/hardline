//! Stack auth command handler - Forge auth, token storage.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error enums
//! - **Calc** (`calc.rs`): Pure functions for auth resolution
//! - **Actions** (`actions.rs`): I/O operations using filesystem and gh CLI
//!
//! Ported from stax `commands/auth.rs` (132 lines) to hardline's functional architecture.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::{
    check_gh_cli_available, get_auth_status, get_saved_token, print_auth_status, run_auth,
};
pub use calc::{
    determine_auth_resolution_order, normalize_token, resolve_active_source, should_use_gh_cli,
    token_source_description, validate_token,
};
pub use data::{AuthError, AuthOptions, AuthResult, AuthSource, AuthStatus, ForgeType};
