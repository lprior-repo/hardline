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
    use crate::error::Error;
    use crate::error_vcs::VcsErrorKind;
    use std::path::PathBuf;

    fn find_repo_root(start: &Path) -> PathBuf {
        start
            .ancestors()
            .find(|dir| dir.join(".git").exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| start.to_path_buf())
    }

    fn repo_root() -> PathBuf {
        find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    fn init_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        dir
    }

    fn init_real_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("git init");
        assert!(output.status.success(), "git init should succeed");
        let output = std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .expect("git config email");
        assert!(output.status.success());
        let output = std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .expect("git config name");
        assert!(output.status.success());
        dir
    }

    fn commit_file(repo: &Path, filename: &str, content: &str) {
        let file_path = repo.join(filename);
        std::fs::write(&file_path, content).expect("write file");
        let add = std::process::Command::new("git")
            .args(["add", filename])
            .current_dir(repo)
            .output()
            .expect("git add");
        assert!(add.status.success(), "git add should succeed");
        let commit = std::process::Command::new("git")
            .args(["commit", "-m", &format!("add {filename}")])
            .current_dir(repo)
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit should succeed");
    }

    // ========================================================================
    // Service creation and initialization
    // ========================================================================

    #[test]
    fn given_new_when_called_then_creates_service() {
        let _service = VcsIntegrationServiceImpl::new();
    }

    #[test]
    fn given_default_when_called_then_same_behavior_as_new() {
        let from_new = VcsIntegrationServiceImpl::new();
        let from_default = VcsIntegrationServiceImpl::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let new_result = from_new.get_status(dir.path());
        let default_result = from_default.get_status(dir.path());
        assert_eq!(new_result.is_err(), default_result.is_err());
    }

    #[test]
    fn given_factory_fn_when_called_then_returns_service() {
        let _service = create_vcs_integration_service();
    }

    #[test]
    fn given_factory_fn_when_used_as_dyn_trait_then_works() {
        let service: Box<dyn VcsIntegrationService> = Box::new(VcsIntegrationServiceImpl::new());
        let dir = init_git_repo();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn given_service_when_boxed_then_arc_backend_send_sync() {
        let service: Arc<dyn VcsIntegrationService> = Arc::new(VcsIntegrationServiceImpl::new());
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<VcsIntegrationServiceImpl>();
        let dir = init_git_repo();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let _arc: Arc<dyn VcsBackend> = backend;
    }

    #[test]
    fn given_multiple_services_when_created_independently_then_all_functional() {
        let s1 = VcsIntegrationServiceImpl::new();
        let s2 = VcsIntegrationServiceImpl::default();
        let dir = init_git_repo();
        let r1 = s1.detect_and_create_backend(dir.path());
        let r2 = s2.detect_and_create_backend(dir.path());
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    // ========================================================================
    // detect_and_create_backend — success paths
    // ========================================================================

    #[test]
    fn given_git_dir_when_detect_then_returns_arc_backend() {
        let dir = init_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let _: Arc<dyn VcsBackend> = backend;
    }

    #[test]
    fn given_this_repo_when_detect_then_succeeds() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok(), "detect should succeed for workspace root");
    }

    #[test]
    fn given_factory_when_detect_on_this_repo_then_succeeds() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok());
    }

    #[test]
    fn given_default_service_when_detect_on_this_repo_then_succeeds() {
        let service = VcsIntegrationServiceImpl::default();
        let result = service.detect_and_create_backend(&repo_root());
        assert!(result.is_ok());
    }

    #[test]
    fn given_real_git_repo_when_detect_then_backend_is_initialized() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let initialized = backend.is_initialized().expect("is_initialized");
        assert!(initialized);
    }

    #[test]
    fn given_multiple_backends_for_same_repo_then_both_initialized() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let b1 = service.detect_and_create_backend(dir.path()).expect("b1");
        let b2 = service.detect_and_create_backend(dir.path()).expect("b2");
        assert!(b1.is_initialized().expect("init1"));
        assert!(b2.is_initialized().expect("init2"));
    }

    // ========================================================================
    // detect_and_create_backend — error paths
    // ========================================================================

    #[test]
    fn given_empty_dir_when_detect_then_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn given_nonexistent_path_when_detect_then_err() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn given_root_path_when_detect_then_err() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(Path::new("/"));
        assert!(result.is_err());
    }

    #[test]
    fn given_tmp_when_detect_then_err() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn given_empty_path_when_detect_then_err() {
        let service = create_vcs_integration_service();
        let result = service.detect_and_create_backend(Path::new(""));
        assert!(result.is_err());
    }

    #[test]
    fn given_dir_with_gitignore_only_when_detect_then_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".gitignore")).expect("create .gitignore");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(
            result.is_err(),
            ".gitignore dir should not count as git repo"
        );
    }

    #[test]
    fn given_dir_with_git_module_only_when_detect_then_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".gitmodules")).expect("create .gitmodules");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_err());
    }

    // ========================================================================
    // Error translation — verify error codes and types
    // ========================================================================

    #[test]
    fn given_no_vcs_when_detect_then_error_code_is_vcs_not_initialized() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        if let Err(err) = result {
            assert_eq!(err.code(), "VCS_NOT_INITIALIZED");
        } else {
            panic!("should error");
        }
    }

    #[test]
    fn given_no_vcs_when_get_status_then_error_propagates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let err = service.get_status(dir.path()).expect_err("should error");
        assert_eq!(err.code(), "VCS_NOT_INITIALIZED");
    }

    #[test]
    fn given_no_vcs_when_list_branches_then_error_propagates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let err = service.list_branches(dir.path()).expect_err("should error");
        assert_eq!(err.code(), "VCS_NOT_INITIALIZED");
    }

    #[test]
    fn given_vcs_error_when_detect_then_error_has_context_map() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        if let Err(err) = result {
            let ctx = err.context_map().expect("context_map");
            assert_eq!(ctx["error_type"], "vcs_not_initialized");
        } else {
            panic!("should error");
        }
    }

    #[test]
    fn given_vcs_error_when_get_status_then_error_has_exit_code() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let err = service.get_status(dir.path()).expect_err("should error");
        assert_ne!(err.exit_code(), 0);
        assert_eq!(err.exit_code(), 30);
    }

    #[test]
    fn given_vcs_error_then_matches_vcs_variant() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        if let Err(err) = result {
            assert!(matches!(err, Error::Vcs(_)));
        } else {
            panic!("should error");
        }
    }

    #[test]
    fn given_vcs_error_then_inner_kind_is_not_initialized() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        if let Err(Error::Vcs(vcs_err)) = result {
            assert!(matches!(vcs_err.kind(), VcsErrorKind::NotInitialized));
        } else {
            panic!("expected Vcs error variant");
        }
    }

    // ========================================================================
    // get_status — integration with real git
    // ========================================================================

    #[test]
    fn given_real_repo_when_get_status_then_succeeds() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Clean);
    }

    #[test]
    fn given_real_repo_with_uncommitted_when_get_status_then_dirty() {
        let dir = init_real_git_repo();
        std::fs::write(dir.path().join("new.txt"), "content").expect("write");
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Dirty);
    }

    #[test]
    fn given_real_repo_after_commit_when_get_status_then_clean() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "hello.txt", "world");
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Clean);
    }

    #[test]
    fn given_real_repo_when_staged_file_then_dirty() {
        let dir = init_real_git_repo();
        std::fs::write(dir.path().join("staged.txt"), "staged").expect("write");
        let add = std::process::Command::new("git")
            .args(["add", "staged.txt"])
            .current_dir(dir.path())
            .output()
            .expect("git add");
        assert!(add.status.success());
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Dirty);
    }

    #[test]
    fn given_this_repo_when_get_status_then_succeeds() {
        let service = create_vcs_integration_service();
        let result = service.get_status(&repo_root());
        assert!(result.is_ok());
    }

    #[test]
    fn given_factory_when_get_status_on_real_repo_then_succeeds() {
        let dir = init_real_git_repo();
        let service = create_vcs_integration_service();
        let result = service.get_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn given_default_service_when_get_status_on_real_repo_then_succeeds() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::default();
        let result = service.get_status(dir.path());
        assert!(result.is_ok());
    }

    // ========================================================================
    // list_branches — integration with real git
    // ========================================================================

    #[test]
    fn given_real_repo_when_list_branches_then_has_main_or_master() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let branches = service.list_branches(dir.path()).expect("branches");
        let has_main = branches
            .iter()
            .any(|b| b.name == "main" || b.name == "master");
        assert!(has_main, "should have main or master branch");
    }

    #[test]
    fn given_real_repo_when_create_branch_then_list_includes_it() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        backend.create_branch("test-branch").expect("create branch");
        let branches = service.list_branches(dir.path()).expect("branches");
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"test-branch"), "should contain test-branch");
    }

    #[test]
    fn given_this_repo_when_list_branches_then_succeeds() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.list_branches(&repo_root());
        assert!(result.is_ok());
    }

    #[test]
    fn given_factory_when_list_branches_on_real_repo_then_succeeds() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = create_vcs_integration_service();
        let result = service.list_branches(dir.path());
        assert!(result.is_ok());
    }

    // ========================================================================
    // Error paths — get_status and list_branches
    // ========================================================================

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
    fn given_root_path_when_get_status_then_err() {
        let service = create_vcs_integration_service();
        let result = service.get_status(Path::new("/"));
        assert!(result.is_err());
    }

    #[test]
    fn given_root_path_when_list_branches_then_err() {
        let service = create_vcs_integration_service();
        let result = service.list_branches(Path::new("/"));
        assert!(result.is_err());
    }

    // ========================================================================
    // Concurrent VCS operations (Send + Sync)
    // ========================================================================

    #[test]
    fn given_shared_service_when_concurrent_detect_then_all_succeed() {
        let dir = init_real_git_repo();
        let service = Arc::new(VcsIntegrationServiceImpl::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let svc = Arc::clone(&service);
            let path = dir.path().to_path_buf();
            handles.push(std::thread::spawn(move || {
                svc.detect_and_create_backend(&path)
            }));
        }
        for handle in handles {
            let result = handle.join().expect("thread panicked");
            assert!(result.is_ok(), "concurrent detect should succeed");
        }
    }

    #[test]
    fn given_shared_service_when_concurrent_status_then_all_succeed() {
        let dir = init_real_git_repo();
        let service = Arc::new(VcsIntegrationServiceImpl::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let svc = Arc::clone(&service);
            let path = dir.path().to_path_buf();
            handles.push(std::thread::spawn(move || svc.get_status(&path)));
        }
        for handle in handles {
            let result = handle.join().expect("thread panicked");
            assert!(result.is_ok(), "concurrent status should succeed");
        }
    }

    #[test]
    fn given_shared_service_when_concurrent_list_branches_then_all_succeed() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = Arc::new(VcsIntegrationServiceImpl::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let svc = Arc::clone(&service);
            let path = dir.path().to_path_buf();
            handles.push(std::thread::spawn(move || svc.list_branches(&path)));
        }
        for handle in handles {
            let result = handle.join().expect("thread panicked");
            assert!(result.is_ok(), "concurrent list_branches should succeed");
        }
    }

    #[test]
    fn given_shared_service_when_concurrent_errors_then_all_get_vcs_not_initialized() {
        let service = Arc::new(VcsIntegrationServiceImpl::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let svc = Arc::clone(&service);
            handles.push(std::thread::spawn(move || {
                let dir = tempfile::tempdir().expect("tempdir");
                svc.get_status(dir.path())
            }));
        }
        for handle in handles {
            let result = handle.join().expect("thread panicked");
            let err = result.expect_err("should error");
            assert_eq!(err.code(), "VCS_NOT_INITIALIZED");
        }
    }

    #[test]
    fn given_service_in_arc_dyn_when_concurrent_then_safe() {
        let service: Arc<dyn VcsIntegrationService> = Arc::new(VcsIntegrationServiceImpl::new());
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let mut handles = vec![];
        for _ in 0..4 {
            let svc = Arc::clone(&service);
            let path = dir.path().to_path_buf();
            handles.push(std::thread::spawn(move || {
                let r1 = svc.detect_and_create_backend(&path);
                let r2 = svc.get_status(&path);
                let r3 = svc.list_branches(&path);
                (r1.is_ok(), r2.is_ok(), r3.is_ok())
            }));
        }
        for handle in handles {
            let (a, b, c) = handle.join().expect("thread panicked");
            assert!(a && b && c, "all concurrent ops should succeed");
        }
    }

    // ========================================================================
    // Backend delegation through service layer
    // ========================================================================

    #[test]
    fn given_backend_from_service_when_status_then_matches_service_status() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let service_status = service.get_status(dir.path()).expect("service status");
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let backend_status = backend.status().expect("backend status");
        assert_eq!(service_status, backend_status);
    }

    #[test]
    fn given_backend_from_service_when_list_branches_then_matches_service() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let service_branches = service.list_branches(dir.path()).expect("service branches");
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let backend_branches = backend.list_branches().expect("backend branches");
        assert_eq!(
            service_branches.len(),
            backend_branches.len(),
            "branch count should match"
        );
    }

    #[test]
    fn given_backend_from_service_when_current_branch_then_returns_name() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let branch = backend.current_branch().expect("current_branch");
        assert!(!branch.is_empty(), "branch name should not be empty");
    }

    #[test]
    fn given_backend_from_service_when_log_then_returns_commits() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "a.txt", "aaa");
        commit_file(dir.path(), "b.txt", "bbb");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let commits = backend.log(10).expect("log");
        assert_eq!(commits.len(), 2, "should have 2 commits");
    }

    #[test]
    fn given_backend_from_service_when_is_initialized_then_true() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        assert!(backend.is_initialized().expect("init"));
    }

    #[test]
    fn given_backend_from_service_when_repo_exists_then_true() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        assert!(backend.repo_exists(dir.path().to_str().expect("path")));
    }

    #[test]
    fn given_backend_from_service_when_repo_status_then_has_fields() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "file.txt", "content");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let status = backend.repo_status().expect("repo_status");
        assert!(status.clean);
        assert!(status.branch.is_some());
        assert!(status.commit_id.is_some());
        assert!(!status.has_conflicts);
        assert!(status.uncommitted_files.is_empty());
    }

    // ========================================================================
    // Workspace context — various path patterns
    // ========================================================================

    #[test]
    fn given_path_with_spaces_when_detect_then_succeeds() {
        let dir = tempfile::Builder::new()
            .prefix("path with spaces")
            .tempdir()
            .expect("tempdir with spaces");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_ok(), "should handle paths with spaces");
    }

    #[test]
    fn given_deeply_nested_git_dir_when_detect_then_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let deep = dir.path().join("a").join("b").join("c").join("d");
        std::fs::create_dir_all(deep.join(".git")).expect("create nested .git");
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(&deep);
        assert!(result.is_ok());
    }

    #[test]
    fn given_relative_path_when_detect_then_err_because_no_cwd_match() {
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn given_path_with_unicode_when_init_real_git_repo_then_works() {
        let dir = tempfile::Builder::new()
            .prefix("über-répo-日本語")
            .tempdir()
            .expect("tempdir");
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .expect("git init");
        assert!(output.status.success());
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn given_symlink_to_git_dir_when_detect_then_follows_link() {
        let dir = init_git_repo();
        let link_dir = tempfile::tempdir().expect("tempdir for link");
        let link = link_dir.path().join("repo-link");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path(), &link).expect("symlink");
        }
        let service = VcsIntegrationServiceImpl::new();
        let result = service.detect_and_create_backend(&link);
        assert!(result.is_ok(), "should follow symlink to git dir");
    }

    // ========================================================================
    // Idempotency and consistency
    // ========================================================================

    #[test]
    fn given_same_repo_when_detect_twice_then_both_succeed() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let r1 = service.detect_and_create_backend(dir.path());
        let r2 = service.detect_and_create_backend(dir.path());
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[test]
    fn given_same_repo_when_status_twice_then_consistent() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let s1 = service.get_status(dir.path()).expect("status1");
        let s2 = service.get_status(dir.path()).expect("status2");
        assert_eq!(s1, s2);
    }

    #[test]
    fn given_same_repo_when_branches_twice_then_consistent() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let b1 = service.list_branches(dir.path()).expect("branches1");
        let b2 = service.list_branches(dir.path()).expect("branches2");
        assert_eq!(b1.len(), b2.len());
    }

    #[test]
    fn given_new_service_each_call_when_detect_then_all_succeed() {
        let dir = init_real_git_repo();
        for _ in 0..5 {
            let service = VcsIntegrationServiceImpl::new();
            assert!(service.detect_and_create_backend(dir.path()).is_ok());
        }
    }

    // ========================================================================
    // Backend operations through service (integration)
    // ========================================================================

    #[test]
    fn given_real_repo_when_create_and_switch_branch_then_succeeds() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        backend.create_branch("feature-x").expect("create branch");
        backend.switch_branch("feature-x").expect("switch branch");
        let current = backend.current_branch().expect("current branch");
        assert_eq!(current, "feature-x");
    }

    #[test]
    fn given_real_repo_when_commit_then_returns_commit_id() {
        let dir = init_real_git_repo();
        std::fs::write(dir.path().join("file.txt"), "content").expect("write");
        let add = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .expect("git add");
        assert!(add.status.success());
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let commit_id = backend.commit("test commit").expect("commit");
        assert!(!commit_id.as_str().is_empty());
    }

    #[test]
    fn given_real_repo_when_multiple_commits_then_log_increases() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "a.txt", "aaa");
        commit_file(dir.path(), "b.txt", "bbb");
        commit_file(dir.path(), "c.txt", "ccc");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let commits = backend.log(10).expect("log");
        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn given_real_repo_when_modify_file_then_status_dirty() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "tracked.txt", "original");
        std::fs::write(dir.path().join("tracked.txt"), "modified").expect("modify");
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Dirty);
    }

    #[test]
    fn given_real_repo_when_delete_file_then_status_dirty() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "to-delete.txt", "content");
        std::fs::remove_file(dir.path().join("to-delete.txt")).expect("delete");
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Dirty);
    }

    #[test]
    fn given_real_repo_when_untracked_file_then_status_dirty() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        std::fs::write(dir.path().join("untracked.txt"), "new").expect("write");
        let service = VcsIntegrationServiceImpl::new();
        let status = service.get_status(dir.path()).expect("status");
        assert_eq!(status, VcsStatus::Dirty);
    }

    #[test]
    fn given_real_repo_when_repo_status_dirty_then_uncommitted_files_nonempty() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "initial.txt", "init");
        std::fs::write(dir.path().join("dirty.txt"), "dirty content").expect("write");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let status = backend.repo_status().expect("repo_status");
        assert!(!status.clean);
        assert!(!status.uncommitted_files.is_empty());
    }

    #[test]
    fn given_real_repo_when_diff_between_commits_then_succeeds() {
        let dir = init_real_git_repo();
        commit_file(dir.path(), "a.txt", "version 1");
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let first_id = {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir.path())
                .output()
                .expect("rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        std::fs::write(dir.path().join("a.txt"), "version 2").expect("write");
        let add = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .expect("add");
        assert!(add.status.success());
        let commit = std::process::Command::new("git")
            .args(["commit", "-m", "update a.txt"])
            .current_dir(dir.path())
            .output()
            .expect("commit");
        assert!(commit.status.success());
        let second_id = {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir.path())
                .output()
                .expect("rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        let from = crate::vcs::CommitId::from_unchecked(&first_id);
        let to = crate::vcs::CommitId::from_unchecked(&second_id);
        let diff = backend.diff(&from, &to).expect("diff");
        assert!(!diff.is_empty());
    }

    // ========================================================================
    // Workspace operations (unimplemented in Git)
    // ========================================================================

    #[test]
    fn given_backend_from_service_when_create_workspace_then_err_unimplemented() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let result = backend.create_workspace("test");
        assert!(result.is_err());
    }

    #[test]
    fn given_backend_from_service_when_switch_workspace_then_err_unimplemented() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let result = backend.switch_workspace("test");
        assert!(result.is_err());
    }

    #[test]
    fn given_backend_from_service_when_list_workspaces_then_err_unimplemented() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let result = backend.list_workspaces();
        assert!(result.is_err());
    }

    #[test]
    fn given_backend_from_service_when_delete_workspace_then_err_unimplemented() {
        let dir = init_real_git_repo();
        let service = VcsIntegrationServiceImpl::new();
        let backend = service
            .detect_and_create_backend(dir.path())
            .expect("backend");
        let result = backend.delete_workspace("test");
        assert!(result.is_err());
    }

    // ========================================================================
    // Send + Sync compile-time verification
    // ========================================================================

    #[test]
    fn given_service_impl_then_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VcsIntegrationServiceImpl>();
    }

    #[test]
    fn given_service_impl_then_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VcsIntegrationServiceImpl>();
    }

    #[test]
    fn given_arc_backend_then_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Arc<dyn VcsBackend>>();
    }

    #[test]
    fn given_arc_backend_then_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Arc<dyn VcsBackend>>();
    }

    #[test]
    fn given_arc_service_then_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Arc<dyn VcsIntegrationService>>();
    }

    #[test]
    fn given_arc_service_then_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Arc<dyn VcsIntegrationService>>();
    }

    #[test]
    fn given_boxed_service_then_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn VcsIntegrationService>>();
    }

    // ========================================================================
    // Property-based tests
    // ========================================================================

    proptest::proptest! {
        #[test]
        fn proptest_detect_err_on_random_non_git_dirs(prefix in "[a-zA-Z0-9_/-]{0,20}") {
            let dir = tempfile::tempdir().expect("tempdir");
            let service = VcsIntegrationServiceImpl::new();
            let path = dir.path().join(&prefix);
            let result = service.detect_and_create_backend(&path);
            assert!(result.is_err(), "non-git path should always err");
        }

        #[test]
        fn proptest_git_dir_always_detects(subpath in "[a-zA-Z0-9_]{1,10}") {
            let dir = tempfile::tempdir().expect("tempdir");
            let git_dir = dir.path().join(&subpath);
            std::fs::create_dir_all(git_dir.join(".git")).expect("create .git");
            let service = VcsIntegrationServiceImpl::new();
            let result = service.detect_and_create_backend(&git_dir);
            assert!(result.is_ok(), "git dir should always detect");
        }

        #[test]
        fn proptest_error_code_consistent_for_non_git(path_str in "/[a-zA-Z0-9_/-]{0,30}") {
            let service = VcsIntegrationServiceImpl::new();
            let path = PathBuf::from(&path_str);
            if let Err(err) = service.detect_and_create_backend(&path) {
                assert_eq!(err.code(), "VCS_NOT_INITIALIZED");
                assert_ne!(err.exit_code(), 0);
            }
        }
    }
}
