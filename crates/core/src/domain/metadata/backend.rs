//! Metadata backend trait for persistence

use crate::Error;

/// Backend trait for metadata persistence
pub trait MetadataBackend {
    /// Load metadata from backend
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata cannot be loaded.
    fn load(&self) -> Result<Vec<u8>, Error>;

    /// Save metadata to backend
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata cannot be saved.
    fn save(&self, data: &[u8]) -> Result<(), Error>;
}
