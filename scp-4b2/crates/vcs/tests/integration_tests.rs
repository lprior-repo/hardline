use scp_vcs::{GitBackend, VcsBackend, VcsStatus};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("Failed to execute git");
    assert!(status.success(), "Git command failed: git {:?}", args);
}

#[test]
fn test_git_integration() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // 1. Init repo
    run_git(repo_path, &["init"]);

    // Normalize default branch to main
    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "-m", "main"])
        .status();

    // Setup git config for commits
    run_git(repo_path, &["config", "user.name", "Test User"]);
    run_git(repo_path, &["config", "user.email", "test@example.com"]);

    let backend = GitBackend::new(repo_path.to_path_buf());

    // Verify initialization
    assert!(backend.is_initialized().unwrap());

    // Status should be clean on empty repo
    let status = backend.status().unwrap();
    assert_eq!(status, VcsStatus::Clean);

    // 2. Add files
    let file_path = repo_path.join("test.txt");
    fs::write(&file_path, "hello world").unwrap();

    // Verify status is dirty after adding file
    let status = backend.status().unwrap();
    assert_eq!(status, VcsStatus::Dirty);

    run_git(repo_path, &["add", "test.txt"]);

    // 3. Commit
    run_git(repo_path, &["commit", "-m", "Initial commit"]);

    // Verify status is clean after commit
    let status = backend.status().unwrap();
    assert_eq!(status, VcsStatus::Clean);

    // 4. Log
    let log = backend.log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].message, "Initial commit");

    // 5. Diff (simulate modification and verify status again)
    fs::write(&file_path, "hello world modified").unwrap();
    let status = backend.status().unwrap();
    assert_eq!(status, VcsStatus::Dirty);

    // Check we can run git diff manually to simulate diff
    let diff_output = Command::new("git")
        .current_dir(repo_path)
        .args(["diff"])
        .output()
        .expect("Failed to execute git diff");
    assert!(diff_output.status.success());
    let diff_str = String::from_utf8_lossy(&diff_output.stdout);
    assert!(diff_str.contains("hello world modified"));
}
