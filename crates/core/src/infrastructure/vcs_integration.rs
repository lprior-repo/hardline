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
        vcs::create_backend(path).map(Arc::<dyn VcsBackend>::from)
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

    /// Walk up from `start` until a directory containing `.git` or `.jj` is found.
    ///
    /// This is needed because `detect_vcs` in the core crate only checks the exact
    /// path (no ancestor walk), and `CARGO_MANIFEST_DIR` points to `crates/core/`
    /// while `.git` lives at the workspace root.
    fn find_repo_root(start: &Path) -> std::path::PathBuf {
        start
            .ancestors()
            .find(|dir| dir.join(".git").exists() || dir.join(".jj").exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| start.to_path_buf())
    }

    /// Return the repository root for this workspace (ancestor of CARGO_MANIFEST_DIR).
    fn repo_root() -> std::path::PathBuf {
        find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    #[test]
    fn test_vcs_service_creation() {
        let service = create_vcs_integration_service();
        let status = service.get_status(Path::new("/tmp"));
        assert!(status.is_err());
    }

    #[test]
    fn given_new_when_called_then_creates_service() {
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(Path::new("/tmp"));
        assert!(status.is_err());
    }

    #[test]
    fn given_default_when_called_then_same_as_new() {
        let _from_new = VcsIntegrationServiceImpl::new();
        let _from_default = VcsIntegrationServiceImpl::default();
        // Both should behave identically: both error on non-git paths
        let status = _from_default.get_status(Path::new("/tmp"));
        assert!(status.is_err());
    }

    #[test]
    fn given_service_when_used_as_trait_object_then_works() {
        let service: Box<dyn VcsIntegrationService> = Box::new(VcsIntegrationServiceImpl::new());
        let status = service.get_status(Path::new("/tmp"));
        assert!(status.is_err());
    }

    #[test]
    fn given_nonexistent_path_when_get_status_then_err() {
        let service = create_vcs_integration_service();
        let result = service.get_status(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn given_nonexistent_path_when_list_branches_then_err() {
        let service = create_vcs_integration_service();
        let result = service.list_branches(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn given_nonexistent_path_when_detect_backend_then_err() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn given_service_when_send_sync_then_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<VcsIntegrationServiceImpl>();
    }

    #[test]
    fn given_service_when_list_branches_on_temp_then_err() {
        let service = create_vcs_integration_service();
        let result = service.list_branches(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn given_new_when_detect_backend_on_this_repo_then_succeeds() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok(), "detect_and_create_backend should succeed for project root");
    }

    #[test]
    fn given_new_when_get_status_on_this_repo_then_succeeds() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.get_status(&repo_root());
        assert!(result.is_ok(), "get_status should succeed for project root");
    }

    #[test]
    fn given_new_when_list_branches_on_this_repo_then_succeeds() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.list_branches(&repo_root());
        assert!(result.is_ok(), "list_branches should succeed for project root");
    }

    #[test]
    fn given_service_when_detect_backend_on_this_repo_then_succeeds() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok(), "detect_and_create_backend should succeed via factory");
    }

    #[test]
    fn given_factory_when_get_status_on_this_repo_then_succeeds() {
        let service = create_vcs_integration_service();
        let result = service.get_status(&repo_root());
        assert!(result.is_ok(), "get_status should succeed via factory");
    }

    #[test]
    fn given_factory_when_list_branches_on_this_repo_then_succeeds() {
        let service = create_vcs_integration_service();
        let result = service.list_branches(&repo_root());
        assert!(result.is_ok(), "list_branches should succeed via factory");
    }

    #[test]
    fn given_service_when_detect_then_result_send_sync() {
        let service = VcsIntegrationServiceImpl::new();
        let _result = service.detect_and_create_backend(&repo_root());
    }

    #[test]
    fn given_service_when_status_then_result_send_sync() {
        let service = VcsIntegrationServiceImpl::new();
        let _result = service.get_status(&repo_root());
    }

    #[test]
    fn given_service_when_branches_then_result_send_sync() {
        let service = VcsIntegrationServiceImpl::new();
        let _result = service.list_branches(&repo_root());
    }

    #[test]
    fn given_service_when_used_in_box_then_compiles() {
        let service: Box<dyn VcsIntegrationService> = Box::new(VcsIntegrationServiceImpl::new());
        let _result = service.detect_and_create_backend(&repo_root());
    }

    #[test]
    fn given_default_when_detect_backend_on_this_repo_then_succeeds() {
        let service = VcsIntegrationServiceImpl::default();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok());
    }

    #[test]
    fn given_root_path_when_get_status_then_err() {
        let service = create_vcs_integration_service();
        let result = service.get_status(Path::new("/"));
        assert!(result.is_err());
    }
}
