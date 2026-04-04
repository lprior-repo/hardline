//! Introspect command handler - Discover hardline capabilities.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): IntrospectOptions, CommandInfo, ArgumentInfo, FlagInfo,
//!   ExampleInfo, ErrorConditionInfo (inert, serializable)
//! - **Actions** (`actions.rs`): run_introspect (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp introspect                   # Show all capabilities
//! scp introspect add               # Show introspection for add command
//! scp introspect remove            # Show introspection for remove command
//! ```

pub mod actions;
pub mod data;

pub use actions::run_introspect;
pub use data::{
    known_commands, ArgumentInfo, CommandInfo, ErrorConditionInfo, ExampleInfo, FlagInfo,
    IntrospectOptions,
};
