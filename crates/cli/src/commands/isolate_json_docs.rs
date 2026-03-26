//! JSON OUTPUT documentation for command help
//! These strings document the `SchemaEnvelope` structure used in JSON output

pub mod json_docs;

// Re-export for backward compatibility
pub use json_docs::response_types::*;
pub use json_docs::system_commands::*;
pub use json_docs::ai_contracts_part1::*;
pub use json_docs::ai_contracts_part2::*;
