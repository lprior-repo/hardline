//! JSON OUTPUT documentation for command help
//! These strings document the `SchemaEnvelope` structure used in JSON output

// Note: We avoid glob re-exports to prevent name collisions between
// ai_contracts (semantic command contracts) and response_types (JSON schema docs).
// Each function is explicitly re-exported with its intended name.

// AI contracts (semantic command descriptions) - PRIMARY exports
pub use super::json_docs::ai_contracts::add;
pub use super::json_docs::ai_contracts::done;
pub use super::json_docs::ai_contracts::spawn;
pub use super::json_docs::ai_contracts_part2::remove;
pub use super::json_docs::ai_contracts_part2::sync;

// Response types (JSON schema documentation) - suffixed exports
pub use super::json_docs::response_types::clean;
pub use super::json_docs::response_types::config;
pub use super::json_docs::response_types::diff;
pub use super::json_docs::response_types::doctor;
pub use super::json_docs::response_types::focus;
pub use super::json_docs::response_types::introspect;
pub use super::json_docs::response_types::list;
pub use super::json_docs::response_types::status;

// System commands
pub use super::json_docs::system_commands::checkpoint;
pub use super::json_docs::system_commands::context;
pub use super::json_docs::system_commands::export;
pub use super::json_docs::system_commands::init;
pub use super::json_docs::system_commands::query;
