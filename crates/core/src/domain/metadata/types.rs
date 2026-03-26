//! Metadata types and error definitions

use std::error::Error;
use std::fmt;

use crate::dag::BranchId;
use crate::Error as CoreError;

/// Error types for metadata operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataError {
    /// Branch not found in metadata
    BranchNotFound(BranchId),
    /// Branch already exists in metadata
    BranchAlreadyExists(BranchId),
    /// Parent branch not found
    ParentNotFound(BranchId),
    /// Setting parent would create circular reference
    CircularReference(BranchId),
    /// Backend operation failed
    Backend(String),
    /// Metadata is corrupted
    Corrupted,
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BranchNotFound(id) => write!(f, "Branch not found: {}", id),
            Self::BranchAlreadyExists(id) => write!(f, "Branch already exists: {}", id),
            Self::ParentNotFound(id) => write!(f, "Parent not found: {}", id),
            Self::CircularReference(id) => {
                write!(f, "Circular reference would be created for branch {}", id)
            }
            Self::Backend(msg) => write!(f, "Backend error: {}", msg),
            Self::Corrupted => write!(f, "Metadata corrupted"),
        }
    }
}

impl Error for MetadataError {}

impl From<MetadataError> for CoreError {
    fn from(err: MetadataError) -> Self {
        match err {
            MetadataError::BranchNotFound(id) => {
                CoreError::NotFound(format!("Branch not found: {}", id))
            }
            MetadataError::BranchAlreadyExists(id) => {
                CoreError::InvalidState(format!("Branch already exists: {}", id))
            }
            MetadataError::ParentNotFound(id) => {
                CoreError::NotFound(format!("Parent not found: {}", id))
            }
            MetadataError::CircularReference(id) => CoreError::InvalidState(format!(
                "Circular reference would be created for branch {}",
                id
            )),
            MetadataError::Backend(msg) => {
                CoreError::InvalidState(format!("Metadata backend error: {}", msg))
            }
            MetadataError::Corrupted => CoreError::InvalidState("Metadata corrupted".to_string()),
        }
    }
}
