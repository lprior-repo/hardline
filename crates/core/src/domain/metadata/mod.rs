//! Metadata domain module - Branch parent-child relationship storage

pub mod backend;
pub mod constructors;
pub mod entities;
pub mod getters;
pub mod operations;
pub mod serialization;
pub mod types;

// Re-export commonly used types
pub use backend::MetadataBackend;
pub use entities::StackMetadata;
pub use types::MetadataError;

// Re-export BranchId for convenience
pub use crate::dag::BranchId;
