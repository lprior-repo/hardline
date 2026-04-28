//! Query command handler - Structured session querying with filters.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): QueryOptions, QueryOutput, SessionInfo, SessionStatus (inert,
//!   serializable types + pure computation)
//! - **Actions** (`actions.rs`): run_query (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp query session-exists my-session       # Check if session exists
//! scp query sessions --status active        # List active sessions
//! scp query session-info my-session         # Get session details
//! scp query blockers                        # Show blocked sessions
//! ```

pub mod actions;
pub mod data;

pub use actions::run_query;
pub use data::{QueryOptions, QueryOutput};
