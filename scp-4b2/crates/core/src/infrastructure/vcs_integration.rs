//! VCS Integration Infrastructure Service

use crate::error::Result;
use crate::vcs::{self, Branch, VcsBackend, VcsStatus};
use std::path::Path;
use std::sync::Arc;

pub trait VcsIntegrationService: Send + Sync {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>>;
    fn get_status(&self, path: &Path) -> Result<VcsStatus>;
    fn list_branches(&self, path: &Path) -> Result<Vec<Branch>>;
}

pub struct VcsIntegrationServiceImpl;

impl VcsIntegrationServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl VcsIntegrationService for VcsIntegrationServiceImpl {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>> {
        vcs::create_backend(path).map(|b| Arc::<dyn VcsBackend>::from(b))
    }

    fn get_status(&self, path: &Path) -> Result<VcsStatus> {
        let backend = self.detect_and_create_backend(path)?;
        backend.status()
    }

    fn list_branches(&self, path: &Path) -> Result<Vec<Branch>> {
        let backend = self.detect_and_create_backend(path)?;
        backend.list_branches()
    }
}

impl Default for VcsIntegrationServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_vcs_integration_service() -> impl VcsIntegrationService {
    VcsIntegrationServiceImpl::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcs_service_creation() {
        let service = create_vcs_integration_service();
        let status = service.get_status(Path::new("/tmp"));
        assert!(status.is_err());
    }
}
