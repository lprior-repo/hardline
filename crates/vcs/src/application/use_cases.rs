//! VCS Use Cases

use std::{path::Path, sync::Arc};

use crate::{
    domain::{traits::VcsBackend, value_objects::VcsType},
    error::{Result, VcsError},
};

pub trait VcsService: Send + Sync {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>>;
    fn detect_vcs_type(&self, path: &Path) -> Option<VcsType>;
}

pub struct VcsServiceImpl;

impl Default for VcsServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl VcsServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl VcsService for VcsServiceImpl {
    fn detect_and_create_backend(&self, path: &Path) -> Result<Arc<dyn VcsBackend>> {
        match self.detect_vcs_type(path) {
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

    #[test]
    fn vcs_service_impl_new() {
        let _service = VcsServiceImpl::new();
    }

    #[test]
    fn vcs_service_impl_default() {
        let _service = VcsServiceImpl::default();
    }

    #[test]
    fn vcs_service_detect_type_nonexistent() {
        let service = VcsServiceImpl::new();
        assert!(service
            .detect_vcs_type(Path::new("/nonexistent/xyz"))
            .is_none());
    }

    #[test]
    fn vcs_service_detect_type_with_temp_dir() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let service = VcsServiceImpl::new();
        assert!(service.detect_vcs_type(dir.path()).is_none());
    }

    #[test]
    fn vcs_service_detect_type_with_git_dir() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let service = VcsServiceImpl::new();
        assert_eq!(service.detect_vcs_type(dir.path()), Some(VcsType::Git));
    }

    #[test]
    fn vcs_service_detect_and_create_backend_no_vcs() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let service = VcsServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_err());
        assert!(matches!(result, Err(VcsError::NotInitialized)));
    }

    #[test]
    fn vcs_service_detect_and_create_backend_git() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let service = VcsServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_ok());
    }

    // -- Proptests --

    proptest::proptest! {
        #[test]
        fn vcs_service_detect_never_panics(path in "[a-zA-Z0-9_/]{1,80}") {
            let service = VcsServiceImpl::new();
            // This should never panic, even for absurd paths
            let _ = service.detect_vcs_type(std::path::Path::new(&path));
        }
    }
}
