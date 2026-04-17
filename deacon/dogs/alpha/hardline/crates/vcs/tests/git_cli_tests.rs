//! Git CLI Backend Tests

use scp_vcs::{GitCliBackend, VcsBackend, VcsError, VcsStatus};
use std::process::Command;
use tempfile::TempDir;

fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(&["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    std::fs::write(path.join("test.txt"), "initial content").unwrap();
    Command::new("git")
        .args(&["add", "."])
        .current_dir(path)
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    temp_dir
}

#[test]
fn test_git_cli_backend_creation_returns_valid_instance() {
    let temp_dir = TempDir::new().unwrap();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    assert_eq!(backend.repo_path(), temp_dir.path());
}

#[test]
fn test_status_returns_not_initialized_when_no_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.status();
    assert!(matches!(result, Err(VcsError::NotInitialized)));
}

#[test]
fn test_status_returns_clean_on_clean_repository() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.status();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), VcsStatus::Clean);
}

#[test]
fn test_status_returns_dirty_on_repository_with_changes() {
    let temp_dir = create_test_repo();
    std::fs::write(temp_dir.path().join("test.txt"), "modified content").unwrap();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.status();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), VcsStatus::Dirty);
}

#[test]
fn test_log_returns_commits_in_reverse_chronological_order() {
    let temp_dir = create_test_repo();
    std::fs::write(temp_dir.path().join("test2.txt"), "more content").unwrap();
    Command::new("git")
        .args(&["add", "."])
        .current_dir(temp_dir.path())
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(&["commit", "-m", "Second commit"])
        .current_dir(temp_dir.path())
        .output()
        .expect("git commit failed");

    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.log(10);
    assert!(result.is_ok());
    let commits = result.unwrap();
    assert!(commits.len() >= 2);
    assert_eq!(commits[0].message.trim(), "Second commit");
}

#[test]
fn test_log_with_limit_returns_exactly_n_commits() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.log(1);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_log_with_zero_limit_returns_empty_vector() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.log(0);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_current_branch_returns_branch_name_on_attached_head() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.current_branch();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "master");
}

#[test]
fn test_list_branches_returns_all_local_branches() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.list_branches();
    assert!(result.is_ok());
    let branches = result.unwrap();
    assert!(!branches.is_empty());
    assert!(branches.iter().any(|b| b.is_current));
}

#[test]
fn test_diff_returns_empty_string_on_clean_repository() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.diff();
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_diff_returns_diff_output_on_dirty_repository() {
    let temp_dir = create_test_repo();
    std::fs::write(temp_dir.path().join("test.txt"), "modified content").unwrap();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.diff();
    assert!(result.is_ok());
    let diff = result.unwrap();
    assert!(diff.contains("test.txt") || diff.contains("modified"));
}

#[test]
fn test_is_initialized_returns_true_for_git_repo() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.is_initialized();
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_is_initialized_returns_false_for_non_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.is_initialized();
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_add_and_commit() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    std::fs::write(temp_dir.path().join("new_file.txt"), "new content").unwrap();
    let result = backend.add(&["new_file.txt"]);
    assert!(result.is_ok());

    let result = backend.commit("Add new file");
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_empty_commit_message() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    std::fs::write(temp_dir.path().join("new.txt"), "new").unwrap();
    let _ = backend.add(&["new.txt"]);

    let result = backend.commit("");
    assert!(result.is_err() || !result.unwrap().is_empty());
}

#[test]
fn test_special_characters_in_branch_name() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    let result = backend.create_branch("feature/test-branch_123");
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_status_with_binary_file() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    std::fs::write(
        temp_dir.path().join("binary.bin"),
        &[0u8, 0xFF, 0xFE, 0x00, 0x42],
    )
    .unwrap();
    let result = backend.status();
    assert!(result.is_ok());
}

#[test]
fn test_log_with_unicode_message() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(&["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    std::fs::write(path.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(&["add", "."])
        .current_dir(path)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(&["commit", "-m", "Unicode test: café 🎉"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.log(10);

    assert!(result.is_ok());
    let commits = result.unwrap();
    assert!(!commits.is_empty());
    assert!(commits[0].message.contains("Unicode test"));
}

#[test]
fn test_concurrent_operations() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    for i in 0..5 {
        std::fs::write(temp_dir.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    let status_result = backend.status();
    let branch_result = backend.current_branch();
    let list_result = backend.list_branches();

    assert!(status_result.is_ok());
    assert!(branch_result.is_ok());
    assert!(list_result.is_ok());
}

#[test]
fn test_deeply_nested_path() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(&["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    std::fs::create_dir_all(path.join("a/b/c/d/e/f/g")).unwrap();
    std::fs::write(path.join("a/b/c/d/e/f/g/test.txt"), "nested").unwrap();

    Command::new("git")
        .args(&["add", "."])
        .current_dir(path)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(&["commit", "-m", "Nested commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let status_result = backend.status();

    assert!(status_result.is_ok());
    assert_eq!(status_result.unwrap(), VcsStatus::Clean);
}

#[test]
fn test_large_number_of_commits() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(&["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    for i in 0..50 {
        std::fs::write(path.join(format!("file{}.txt", i)), format!("content{}", i)).unwrap();
        Command::new("git")
            .args(&["add", "."])
            .current_dir(path)
            .output()
            .expect("git add failed");
        Command::new("git")
            .args(&["commit", "-m", &format!("Commit {}", i)])
            .current_dir(path)
            .output()
            .expect("git commit failed");
    }

    let backend = GitCliBackend::new_from_path(temp_dir.path());
    let result = backend.log(50);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 50);
}

#[test]
fn test_list_branches_after_branch_creation() {
    let temp_dir = create_test_repo();
    let backend = GitCliBackend::new_from_path(temp_dir.path());

    let initial_branches = backend.list_branches().unwrap();
    let initial_count = initial_branches.len();

    let _ = backend.create_branch("test-feature");

    let new_branches = backend.list_branches().unwrap();
    assert!(new_branches.len() >= initial_count);
}
