//! Tests for gix rebase operations
//!
//! Tests the pure-gix rebase implementation end-to-end using real git repos.

use gix::bstr::ByteSlice;
use gix::prelude::ObjectIdExt;
use scp_vcs::gix::rebase;
use scp_vcs::gix::rebase::RebaseResult;
use scp_vcs::gix::repository;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test repo with initial commit on main.
fn create_test_repo() -> (TempDir, gix::Repository) {
    let temp = TempDir::new().unwrap();
    let path = temp.path().to_path_buf();

    Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .expect("git init");

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&path)
        .output()
        .expect("git config email");

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&path)
        .output()
        .expect("git config name");

    let repo = repository::open(&path).expect("open repo");
    (temp, repo)
}

/// Create a commit with a file in the repo.
fn create_commit(repo_path: &std::path::Path, filename: &str, content: &str, msg: &str) {
    let file_path = repo_path.join(filename);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&file_path, content).expect("write file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add");

    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(repo_path)
        .output()
        .expect("git commit");
}

/// Create a branch at current HEAD.
fn create_branch(repo_path: &std::path::Path, name: &str) {
    Command::new("git")
        .args(["branch", name])
        .current_dir(repo_path)
        .output()
        .expect("git branch");
}

/// Checkout a branch.
fn git_checkout(repo_path: &std::path::Path, branch: &str) {
    let output = Command::new("git")
        .args(["checkout", branch])
        .current_dir(repo_path)
        .output()
        .expect("git checkout");
    assert!(output.status.success(), "git checkout {branch} failed");
}

/// Get HEAD commit SHA.
fn head_sha(repo_path: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("git rev-parse");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get branch tip SHA.
fn branch_sha(repo_path: &std::path::Path, branch: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", branch])
        .current_dir(repo_path)
        .output()
        .expect("git rev-parse branch");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Detect the default branch name (main or master).
fn default_branch(repo_path: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_path)
        .output()
        .expect("git branch --show-current");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Count commits between two refs.
#[allow(dead_code)]
fn count_commits_between(repo_path: &std::path::Path, from: &str, to: &str) -> usize {
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{from}..{to}")])
        .current_dir(repo_path)
        .output()
        .expect("git rev-list");

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0)
}

// ============================================================================
// Rebase Result Type Tests
// ============================================================================

#[test]
fn test_rebase_result_equality() {
    let a = RebaseResult::Success {
        commits_replayed: 3,
    };
    let b = RebaseResult::Success {
        commits_replayed: 3,
    };
    assert_eq!(a, b);

    let c = RebaseResult::AlreadyUpToDate;
    assert_ne!(a, c);
}

#[test]
fn test_rebase_result_conflict_equality() {
    let a = RebaseResult::Conflict {
        conflicted_files: vec!["file.rs".to_string()],
        commits_replayed: 1,
        remaining_commits: 2,
    };
    let b = RebaseResult::Conflict {
        conflicted_files: vec!["file.rs".to_string()],
        commits_replayed: 1,
        remaining_commits: 2,
    };
    assert_eq!(a, b);
}

// ============================================================================
// Merge Base Tests
// ============================================================================

#[test]
fn test_find_merge_base_same_commit() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "file.txt", "initial\n", "initial commit");
    let sha = head_sha(path);
    let oid = sha.parse::<gix::ObjectId>().unwrap();

    let result = rebase::find_merge_base(&repo, oid, oid).expect("find merge base");
    assert_eq!(result, Some(oid));
}

#[test]
fn test_find_merge_base_linear_history() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "a.txt", "a\n", "commit A");
    let a_sha = head_sha(path);
    create_commit(path, "b.txt", "b\n", "commit B");
    let b_sha = head_sha(path);

    let a_oid = a_sha.parse::<gix::ObjectId>().unwrap();
    let b_oid = b_sha.parse::<gix::ObjectId>().unwrap();

    let result = rebase::find_merge_base(&repo, a_oid, b_oid).expect("find merge base");
    assert_eq!(result, Some(a_oid));
}

#[test]
fn test_find_merge_base_forked_branches() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    // Create shared ancestor
    create_commit(path, "base.txt", "base\n", "base commit");
    let base_sha = head_sha(path);

    create_branch(path, "feature");

    // Commit on main
    create_commit(path, "main.txt", "main\n", "main commit");
    let main_sha = head_sha(path);

    // Commit on feature
    git_checkout(path, "feature");
    create_commit(path, "feature.txt", "feature\n", "feature commit");
    let feature_sha = head_sha(path);

    let main_oid = main_sha.parse::<gix::ObjectId>().unwrap();
    let feature_oid = feature_sha.parse::<gix::ObjectId>().unwrap();
    let base_oid = base_sha.parse::<gix::ObjectId>().unwrap();

    let result = rebase::find_merge_base(&repo, main_oid, feature_oid).expect("find merge base");
    assert_eq!(result, Some(base_oid));
}

// ============================================================================
// Rebase Branch Onto Tests
// ============================================================================

#[test]
fn test_rebase_already_up_to_date_same_tip() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "file.txt", "content\n", "initial");
    create_branch(path, "feature");

    let br = default_branch(path);
    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    match result {
        Ok(RebaseResult::AlreadyUpToDate) => {}
        Ok(other) => panic!("Expected AlreadyUpToDate, got {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[test]
fn test_rebase_branch_behind_parent() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "base.txt", "base\n", "base commit");
    create_branch(path, "feature");

    // Advance parent branch
    let br = default_branch(path);
    create_commit(path, "main.txt", "main content\n", "main advance");

    // Feature is behind main — rebasing onto main
    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    match result {
        Ok(RebaseResult::AlreadyUpToDate) | Ok(RebaseResult::Success { .. }) => {}
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[test]
fn test_rebase_simple_divergence() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    // Shared base
    create_commit(path, "base.txt", "base\n", "base commit");

    // Fork: feature gets 2 commits
    create_branch(path, "feature");
    git_checkout(path, "feature");
    create_commit(path, "feature1.txt", "f1\n", "feature commit 1");
    create_commit(path, "feature2.txt", "f2\n", "feature commit 2");
    let feature_before = branch_sha(path, "feature");

    // Main gets 1 commit
    git_checkout(path, &br);
    create_commit(path, "main1.txt", "m1\n", "main commit 1");
    let main_sha = head_sha(path);

    // Rebase feature onto main
    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    match result {
        Ok(RebaseResult::Success { commits_replayed }) => {
            assert_eq!(commits_replayed, 2);
            let feature_after = branch_sha(path, "feature");
            assert_ne!(feature_before, feature_after, "feature tip should change");

            let count = count_commits_between(path, &main_sha, &feature_after);
            assert_eq!(count, 2, "feature should have 2 commits ahead of main");
        }
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[test]
fn test_rebase_nonexistent_branch() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "file.txt", "content\n", "initial");

    let result = rebase::rebase_branch_onto(&repo, "nonexistent", &br);
    assert!(result.is_err(), "Should fail for nonexistent branch");
}

#[test]
fn test_rebase_nonexistent_parent() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "file.txt", "content\n", "initial");
    create_branch(path, "feature");

    let result = rebase::rebase_branch_onto(&repo, "feature", "nonexistent");
    assert!(result.is_err(), "Should fail for nonexistent parent");
}

#[test]
fn test_rebase_single_commit() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "base.txt", "base\n", "base");
    create_branch(path, "feature");
    git_checkout(path, "feature");
    create_commit(path, "feature.txt", "f\n", "feature work");

    git_checkout(path, &br);
    create_commit(path, "main.txt", "m\n", "main work");

    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    match result {
        Ok(RebaseResult::Success { commits_replayed }) => {
            assert_eq!(commits_replayed, 1);
        }
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[test]
fn test_rebase_preserves_files() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "base.txt", "base\n", "base");
    create_branch(path, "feature");
    git_checkout(path, "feature");
    create_commit(
        path,
        "feature_file.txt",
        "feature content\n",
        "add feature file",
    );

    git_checkout(path, &br);
    create_commit(path, "main_file.txt", "main content\n", "add main file");

    let result = rebase::rebase_branch_onto(&repo, "feature", &br);
    assert!(result.is_ok(), "Rebase should succeed: {result:?}");

    // Verify feature_file.txt is in the rebased tree
    let feature_sha = branch_sha(path, "feature");
    let feature_oid = feature_sha.parse::<gix::ObjectId>().unwrap();

    let commit = feature_oid
        .attach(&repo)
        .object()
        .expect("read commit")
        .peel_to_commit()
        .expect("peel to commit");

    let tree = commit
        .tree_id()
        .expect("tree id")
        .object()
        .expect("read tree")
        .into_tree();

    let mut found_feature_file = false;
    for entry in tree.iter() {
        let entry = entry.expect("tree entry");
        let name = entry.filename().to_str_lossy().to_string();
        if name == "feature_file.txt" {
            found_feature_file = true;
        }
    }
    assert!(
        found_feature_file,
        "feature_file.txt should be in rebased tree"
    );
}

// ============================================================================
// Rebase Continue Tests
// ============================================================================

#[test]
fn test_rebase_continue_no_remaining() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "base.txt", "base\n", "base");
    create_branch(path, "feature");
    git_checkout(path, "feature");
    create_commit(path, "feature.txt", "f\n", "feature work");

    git_checkout(path, &br);
    create_commit(path, "main.txt", "m\n", "main work");

    let main_sha = head_sha(path);

    // Continue with the main tip as resolved state
    // The feature branch has 1 commit ahead of the base, so continuing
    // from the main tip should replay that 1 feature commit
    let result = rebase::rebase_continue(&repo, "feature", &br, &main_sha);

    match result {
        Ok(RebaseResult::Success { commits_replayed }) => {
            // The feature commit gets replayed onto the main tip
            assert!(commits_replayed <= 1);
        }
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

// ============================================================================
// Rebase In Progress Tests
// ============================================================================

#[test]
fn test_rebase_in_progress_clean_repo() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "file.txt", "content\n", "initial");

    let result = rebase::rebase_in_progress(&repo).expect("check rebase state");
    assert!(result.is_none(), "No rebase should be in progress");
}

#[test]
fn test_rebase_in_progress_with_state() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();

    create_commit(path, "file.txt", "content\n", "initial");

    let git_dir = path.join(".git").join("rebase-merge");
    fs::create_dir_all(&git_dir).expect("create rebase-merge dir");
    fs::write(git_dir.join("stopped-sha"), "abc123").expect("write stopped-sha");

    let result = rebase::rebase_in_progress(&repo).expect("check rebase state");
    assert!(result.is_some(), "Should detect rebase in progress");
}

// ============================================================================
// Multi-Commit Rebase Tests
// ============================================================================

#[test]
fn test_rebase_three_commits() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "base.txt", "base\n", "base");
    create_branch(path, "feature");
    git_checkout(path, "feature");
    create_commit(path, "c1.txt", "c1\n", "commit 1");
    create_commit(path, "c2.txt", "c2\n", "commit 2");
    create_commit(path, "c3.txt", "c3\n", "commit 3");

    git_checkout(path, &br);
    create_commit(path, "main1.txt", "m1\n", "main commit 1");

    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    match result {
        Ok(RebaseResult::Success { commits_replayed }) => {
            assert_eq!(commits_replayed, 3);
        }
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[test]
fn test_rebase_no_extra_commits() {
    let (_temp, repo) = create_test_repo();
    let path = repo.workdir().unwrap();
    let br = default_branch(path);

    create_commit(path, "base.txt", "base\n", "base");
    create_branch(path, "feature");

    // Only main advances
    create_commit(path, "main1.txt", "m1\n", "main commit 1");

    let result = rebase::rebase_branch_onto(&repo, "feature", &br);

    // Feature is ancestor of main → already up to date
    match result {
        Ok(RebaseResult::AlreadyUpToDate) | Ok(RebaseResult::Success { .. }) => {}
        Ok(other) => panic!("Unexpected result: {other:?}"),
        Err(e) => panic!("Unexpected error: {e}"),
    }
}
