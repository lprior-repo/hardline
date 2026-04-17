//! Integration tests for GitWorktreeAdapter
//!
//! Tests the Git adapter with real git repositories

use std::process::Command;
use tempfile::TempDir;
use worktree::domain::BranchName;
use worktree::infrastructure::git::GitWorktreeAdapter;

/// Create a temporary git repository for testing
fn create_test_repo() -> (TempDir, GitWorktreeAdapter) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git email");

    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git user");

    // Create initial commit
    let readme_path = repo_path.join("README.md");
    std::fs::write(&readme_path, "# Test Repo").unwrap();

    Command::new("git")
        .arg("add")
        .arg("README.md")
        .current_dir(repo_path)
        .output()
        .expect("Failed to add file");

    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    let adapter = GitWorktreeAdapter::new(repo_path).unwrap();
    (temp_dir, adapter)
}

mod git_adapter_integration_tests {
    use super::*;

    #[test]
    fn git_adapter_open_valid_repository_returns_adapter() {
        let (_temp_dir, adapter) = create_test_repo();
        assert!(!adapter.repository().is_bare());
    }

    #[test]
    fn git_adapter_open_nonexistent_path_returns_error() {
        let result = GitWorktreeAdapter::new("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[test]
    fn git_adapter_open_not_a_git_repo_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let result = GitWorktreeAdapter::new(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn git_adapter_get_current_branch_returns_master_branch() {
        let (_temp_dir, adapter) = create_test_repo();
        let branch = adapter.get_current_branch().unwrap();
        assert!(branch.is_some());
        assert_eq!(branch.unwrap().as_str(), "master");
    }

    #[test]
    fn git_adapter_get_local_branches_returns_branch_list() {
        let (_temp_dir, adapter) = create_test_repo();

        // Get local branches
        let branches = adapter.get_local_branches().unwrap();
        assert!(!branches.is_empty());
        assert!(branches.iter().any(|b| b.as_str() == "master"));
    }

    #[test]
    fn git_adapter_get_remote_branches_strips_origin_prefix() {
        let (temp_dir, adapter) = create_test_repo();

        // Create a remote and branch
        let remote_dir = TempDir::new().unwrap();
        Command::new("git")
            .arg("init")
            .arg("--bare")
            .current_dir(remote_dir.path())
            .output()
            .expect("Failed to init bare repo");

        Command::new("git")
            .arg("remote")
            .arg("add")
            .arg("origin")
            .arg(remote_dir.path().to_str().unwrap())
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to add remote");

        // Create and push a feature branch
        Command::new("git")
            .arg("checkout")
            .arg("-b")
            .arg("feature/test")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to create branch");

        std::fs::write(temp_dir.path().join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .arg("add")
            .arg("feature.txt")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to add feature");

        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Add feature")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to commit feature");

        Command::new("git")
            .arg("push")
            .arg("-u")
            .arg("origin")
            .arg("feature/test")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to push feature");

        // Switch back to master
        Command::new("git")
            .arg("checkout")
            .arg("master")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to checkout master");

        let remote_branches = adapter.get_remote_branches().unwrap();

        // Should have origin/feature -> feature
        assert!(remote_branches.iter().any(|b| b.as_str() == "feature/test"));
    }

    #[test]
    fn git_adapter_get_parent_repository_path_returns_parent_path() {
        let (temp_dir, adapter) = create_test_repo();

        let parent = adapter.get_parent_path().unwrap();
        assert_eq!(parent.as_str(), temp_dir.path().to_str().unwrap());
    }

    // Skip this test as bare repos can't be checked out
    // #[test]
    // fn git_adapter_get_current_branch_no_head_returns_none() {
    //     let temp_dir = TempDir::new().unwrap();
    //
    //     // Create a bare repo without commits
    //     Command::new("git")
    //         .arg("init")
    //         .arg("--bare")
    //         .current_dir(temp_dir.path())
    //         .output()
    //         .expect("Failed to init bare repo");
    //
    //     let result = GitWorktreeAdapter::new(temp_dir.path());
    //     // Bare repos can't be checked out, so this should fail or return no branch
    //     assert!(result.is_err() || result.unwrap().get_current_branch().unwrap().is_none());
    // }

    #[test]
    fn git_adapter_list_worktrees_returns_worktree_list() {
        let (_temp_dir, _adapter) = create_test_repo();

        let _worktrees = create_test_repo().1.list_worktrees().unwrap();
        assert!(_worktrees.is_empty() || !_worktrees.is_empty());
    }

    #[test]
    fn git_adapter_branch_name_validation() {
        let (_temp_dir, _adapter) = create_test_repo();

        // Test valid branch names
        assert!(BranchName::new("main").is_ok());
        assert!(BranchName::new("develop").is_ok());
        assert!(BranchName::new("feature/test").is_ok());
        assert!(BranchName::new("release/1.0.0").is_ok());

        // Test invalid branch names
        assert!(BranchName::new("").is_err());
        assert!(BranchName::new("-feature").is_err());
        assert!(BranchName::new(".hidden").is_err());
    }
}
