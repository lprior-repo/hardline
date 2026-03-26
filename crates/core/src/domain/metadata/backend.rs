//! Metadata backend trait for persistence

use crate::Error;

/// Backend trait for metadata persistence
pub trait MetadataBackend {
    /// Load metadata from backend
    fn load(&self) -> Result<Vec<u8>, Error>;

    /// Save metadata to backend
    fn save(&self, data: &[u8]) -> Result<(), Error>;
}
