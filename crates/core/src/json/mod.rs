//! JSON output structures for AI-first CLI design
//!
//! This module provides consistent JSON output formats across all commands.

pub mod envelope;
pub mod envelope_array;
pub mod error_code;
pub mod error_mapping;
pub mod error_types;

pub mod hateoas;
pub mod helpers;
pub mod meta;
pub mod response;
pub mod schemas;
pub mod serializable;
pub mod serializers;

#[cfg(test)]
mod tests;

// Re-export commonly used types for convenience
pub use envelope::SchemaEnvelope;
pub use envelope_array::SchemaEnvelopeArray;
pub use error_code::ErrorCode;
pub use error_mapping::{classify_exit_code, map_error_to_parts};
pub use error_types::{ErrorDetail, JsonError, JsonSuccess};
pub use hateoas::{HateoasLink, RelatedResources};
pub use helpers::{
    error_with_available_sessions, output_json_parse_error, output_json_success, semantic_exit_code,
};
pub use meta::ResponseMeta;
pub use response::{FixRisk, Response, ResponseError, ResponseFix, ResponseMetadata};
pub use serializable::JsonSerializable;
pub use serializers::{
    ConfigSetOutput, ConfigValueOutput, DiffOutput, DiffStatOutput, FileDiffStatOutput,
    InitOutput, TemplateDeleteOutput, TemplateShowOutput,
};
