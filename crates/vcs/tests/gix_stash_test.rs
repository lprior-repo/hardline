//! Integration tests for gix stash operations
//!
//! Tests real git operations using temp repositories.

use scp_vcs::gix::{repository, stash};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper: create a temp repo with an initial commit.
fn init_repo_with_commit() -> (TempDir, gix::Repository) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Init repo via git CLI (gix init doesn't create initial commit)
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Configure git user
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial file and commit
    fs::write(repo_path.join("hello.txt"), "world").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    let repo = repository::open(&repo_path).unwrap();
    (temp_dir, repo)
}

#[test]
fn stash_save_and_list() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    // Modify a file to create dirty state
    fs::write(repo_path.join("hello.txt"), "modified").unwrap();

    // Save stash with message
    let result = stash::save(&repo, Some("test stash"), false);
    assert!(result.is_ok(), "save failed: {:?}", result);

    // Working tree should be clean
    let content = fs::read_to_string(repo_path.join("hello.txt")).unwrap();
    assert_eq!(content, "world", "working tree should be restored after stash");

    // List should show the stash
    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 1, "should have exactly one stash entry");
    assert_eq!(entries[0].index, 0);
    assert!(entries[0].message.contains("test stash"));
}

#[test]
fn stash_save_pop_roundtrip() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    // Create dirty state
    fs::write(repo_path.join("hello.txt"), "stashed content").unwrap();

    // Save
    stash::save(&repo, None, false).unwrap();

    // Verify clean
    let content = fs::read_to_string(repo_path.join("hello.txt")).unwrap();
    assert_eq!(content, "world");

    // Pop
    stash::pop(&repo, 0).unwrap();

    // Verify stashed content restored
    let content = fs::read_to_string(repo_path.join("hello.txt")).unwrap();
    assert_eq!(content, "stashed content", "popped content should be restored");

    // Stash list should be empty
    let entries = stash::list(&repo).unwrap();
    assert!(entries.is_empty(), "stash should be empty after pop");
}

#[test]
fn stash_save_include_untracked() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    // Create an untracked file
    fs::write(repo_path.join("new_file.txt"), "untracked content").unwrap();

    stash::save(&repo, Some("with untracked"), true).unwrap();

    // Untracked file should be gone
    assert!(
        !repo_path.join("new_file.txt").exists(),
        "untracked file should be stashed"
    );

    // Pop and verify untracked file returns
    stash::pop(&repo, 0).unwrap();
    let content = fs::read_to_string(repo_path.join("new_file.txt")).unwrap();
    assert_eq!(content, "untracked content");
}

#[test]
fn stash_drop_removes_entry() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    // Create two stashes
    fs::write(repo_path.join("hello.txt"), "change1").unwrap();
    stash::save(&repo, Some("first"), false).unwrap();

    fs::write(repo_path.join("hello.txt"), "change2").unwrap();
    stash::save(&repo, Some("second"), false).unwrap();

    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 2);

    // Drop the first (index 0, which is "second" since stash is LIFO)
    stash::drop(&repo, 0).unwrap();

    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].message.contains("first"));
}

#[test]
fn stash_show_returns_diff() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    fs::write(repo_path.join("hello.txt"), "modified for show").unwrap();
    stash::save(&repo, Some("show test"), false).unwrap();

    let diff = stash::show(&repo, 0).unwrap();
    assert!(
        diff.contains("modified for show") || diff.contains("hello.txt"),
        "diff should reference the changed file: {diff}"
    );
}

#[test]
fn stash_list_empty_repo() {
    let (_temp_dir, repo) = init_repo_with_commit();

    let entries = stash::list(&repo).unwrap();
    assert!(entries.is_empty(), "fresh repo should have no stashes");
}

#[test]
fn stash_save_no_changes() {
    let (_temp_dir, repo) = init_repo_with_commit();

    // Save on clean working tree should succeed (no-op)
    let result = stash::save(&repo, None, false);
    assert!(result.is_ok(), "save on clean tree should be ok: {:?}", result);
}

#[test]
fn stash_pop_invalid_index() {
    let (_temp_dir, repo) = init_repo_with_commit();

    let result = stash::pop(&repo, 99);
    assert!(result.is_err(), "popping non-existent stash should fail");
}

#[test]
fn stash_multiple_save_pop_order() {
    let (temp_dir, repo) = init_repo_with_commit();
    let repo_path = temp_dir.path();

    // Create 3 stashes
    for i in 1..=3 {
        fs::write(repo_path.join("hello.txt"), format!("change{i}")).unwrap();
        stash::save(&repo, Some(&format!("stash-{i}")), false).unwrap();
    }

    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 3);

    // Pop the most recent (index 0)
    stash::pop(&repo, 0).unwrap();

    let content = fs::read_to_string(repo_path.join("hello.txt")).unwrap();
    assert_eq!(content, "change3");

    // Should have 2 left
    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 2);
}
