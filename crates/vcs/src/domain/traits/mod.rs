//! VCS Backend Trait - Domain contract for VCS operations

use crate::{
    domain::{
        entities::{Branch, Commit, Workspace},
        value_objects::VcsStatus,
    },
    error::Result,
};

pub trait VcsBackend: Send + Sync {
    fn current_branch(&self) -> Result<String>;

    fn list_branches(&self) -> Result<Vec<Branch>>;

    fn create_branch(&self, name: &str) -> Result<()>;

    fn switch_branch(&self, name: &str) -> Result<()>;

    fn push(&self) -> Result<()>;

    fn pull(&self) -> Result<()>;

    fn rebase(&self, onto: &str) -> Result<()>;

    fn merge(&self, branch: &str) -> Result<()>;

    fn log(&self, limit: usize) -> Result<Vec<Commit>>;

    fn status(&self) -> Result<VcsStatus>;

    fn is_initialized(&self) -> Result<bool>;

    fn create_workspace(&self, name: &str) -> Result<()>;

    fn switch_workspace(&self, name: &str) -> Result<()>;

    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    fn delete_workspace(&self, name: &str) -> Result<()>;

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()>;

    fn merge_workspace(&self, name: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stub VCSBackend for testing that the trait can be implemented
    struct StubVcsBackend;

    impl VcsBackend for StubVcsBackend {
        fn current_branch(&self) -> Result<String> {
            Ok("main".to_string())
        }
        fn list_branches(&self) -> Result<Vec<Branch>> {
            Ok(vec![Branch::new("main".to_string(), true, None)])
        }
        fn create_branch(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn switch_branch(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn push(&self) -> Result<()> {
            Ok(())
        }
        fn pull(&self) -> Result<()> {
            Ok(())
        }
        fn rebase(&self, _onto: &str) -> Result<()> {
            Ok(())
        }
        fn merge(&self, _branch: &str) -> Result<()> {
            Ok(())
        }
        fn log(&self, _limit: usize) -> Result<Vec<Commit>> {
            Ok(vec![])
        }
        fn status(&self) -> Result<VcsStatus> {
            Ok(VcsStatus::Clean)
        }
        fn is_initialized(&self) -> Result<bool> {
            Ok(true)
        }
        fn create_workspace(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn switch_workspace(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn list_workspaces(&self) -> Result<Vec<Workspace>> {
            Ok(vec![])
        }
        fn delete_workspace(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn fork_workspace(&self, _source: &str, _target: &str) -> Result<()> {
            Ok(())
        }
        fn merge_workspace(&self, _name: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn stub_backend_current_branch() {
        let stub = StubVcsBackend;
        assert_eq!(stub.current_branch().expect("ok"), "main");
    }

    #[test]
    fn stub_backend_list_branches() {
        let stub = StubVcsBackend;
        let branches = stub.list_branches().expect("ok");
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn stub_backend_status() {
        let stub = StubVcsBackend;
        assert_eq!(stub.status().expect("ok"), VcsStatus::Clean);
    }

    #[test]
    fn stub_backend_is_initialized() {
        let stub = StubVcsBackend;
        assert!(stub.is_initialized().expect("ok"));
    }

    #[test]
    fn stub_backend_log_empty() {
        let stub = StubVcsBackend;
        let log = stub.log(10).expect("ok");
        assert!(log.is_empty());
    }

    #[test]
    fn stub_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StubVcsBackend>();
    }

    #[test]
    fn stub_backend_operations_return_ok() {
        let stub = StubVcsBackend;
        assert!(stub.create_branch("test").is_ok());
        assert!(stub.switch_branch("test").is_ok());
        assert!(stub.push().is_ok());
        assert!(stub.pull().is_ok());
        assert!(stub.rebase("main").is_ok());
        assert!(stub.merge("feature").is_ok());
    }
}
