//! VCS Use Cases

use crate::domain::traits::VcsBackend;
use crate::domain::value_objects::VcsType;
use crate::error::{Result, VcsError};
use std::path::Path;
use std::sync::Arc;

pub trait VcsService: Send + Sync {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>>;
    fn detect_vcs_type(&self, path: &Path) -> Option<VcsType>;
}

pub struct VcsServiceImpl;

impl VcsServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl VcsService for VcsServiceImpl {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>> {
        match self.detect_vcs_type(path) {
            Some(VcsType::Jujutsu) => {
                use crate::infrastructure::JjBackend;
                Ok(Arc::new(JjBackend::new_from_path(path)) as Arc<dyn VcsBackend>)
            }
            Some(VcsType::Git) => {
                use crate::infrastructure::GitBackend;
                Ok(Arc::new(GitBackend::new_from_path(path)) as Arc<dyn VcsBackend>)
            }
            None => Err(VcsError::NotInitialized),
        }
    }

    fn detect_vcs_type(&self, path: &Path) -> Option<VcsType> {
        VcsType::detect(path)
    }
}

pub fn create_vcs_service() -> impl VcsService {
    VcsServiceImpl::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcs_service_creation() {
        let service = create_vcs_service();
        let vcs_type = service.detect_vcs_type(Path::new("/tmp"));
        assert!(vcs_type.is_none());
    }
}
