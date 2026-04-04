//! Schema command handler - Show JSON Schema definitions for AI agents.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): SchemaOptions, SchemaInfo, SchemaListOutput, AllSchemasOutput
//!   (inert, serializable)
//! - **Actions** (`actions.rs`): run_schema, resolve_schema (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp schema                   # List available schemas
//! scp schema --list            # List available schemas
//! scp schema --all             # Dump all schema definitions
//! scp schema add-response      # Show schema for add-response
//! ```

pub mod actions;
pub mod data;

pub use actions::run_schema;
pub use data::{AllSchemasOutput, SchemaInfo, SchemaListOutput, SchemaMode, SchemaOptions};
