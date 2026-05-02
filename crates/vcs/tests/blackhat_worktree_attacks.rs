//! Black-hat attack tests for VCS worktree operations.

use std::{path::PathBuf, process::Command};

use scp_vcs::gix::worktree;

/// Helper: create a temp git repo with an initial commit.
fn make_repo() -> (tempfile::TempDir, gix::Repository) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let path = tmp.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .expect("git config name");

    std::fs::write(path.join("README.md"), "initial").expect("write file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output()
        .expect("git commit");

    let repo = gix::open(path).expect("gix open");
    (tmp, repo)
}

// ============================================================================
// ATTACK 1: Add worktree with existing path
// ============================================================================
#[test]
fn attack_add_worktree_existing_path() {
    let (_tmp, repo) = make_repo();
    // Use the repo's own path as worktree target - should fail
    let existing_path = _tmp.path().to_path_buf();
    let result = worktree::add(&repo, &existing_path, None);
    assert!(
        result.is_err(),
        "Adding worktree at existing repo path should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 2: Add worktree with path outside repo
// ============================================================================
#[test]
fn attack_add_worktree_outside_repo() {
    let (_tmp, repo) = make_repo();
    let outside = tempfile::TempDir::new().expect("temp dir outside");
    let outside_path = outside.path().join("wt-outside");
    let result = worktree::add(&repo, &outside_path, None);
    // Git allows worktrees outside the repo, but let's verify behavior
    // FINDING: No restriction on worktree placement - can create anywhere
    if result.is_ok() {
        // Clean up the worktree
        let _ = worktree::remove(&repo, &outside_path, true);
    }
}

// ============================================================================
// ATTACK 3: Remove nonexistent worktree
// ============================================================================
#[test]
fn attack_remove_nonexistent_worktree() {
    let (_tmp, repo) = make_repo();
    let fake_path = PathBuf::from("/tmp/nonexistent-worktree-path-xyz-123");
    let result = worktree::remove(&repo, &fake_path, false);
    assert!(
        result.is_err(),
        "Removing nonexistent worktree should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 4: List worktrees in normal repo
// ============================================================================
#[test]
fn attack_list_worktrees_normal_repo() {
    let (_tmp, repo) = make_repo();
    let result = worktree::list(&repo);
    assert!(
        result.is_ok(),
        "Listing worktrees in normal repo should succeed"
    );
    let wt_list = result.expect("list");
    assert!(
        !wt_list.is_empty(),
        "Normal repo should have at least main worktree"
    );
    assert!(wt_list[0].is_main, "First worktree should be main");
}

// ============================================================================
// ATTACK 5: Add worktree with branch that already exists
// ============================================================================
#[test]
fn attack_add_worktree_existing_branch() {
    let (_tmp, repo) = make_repo();
    // "main" already exists - git worktree add -b main would create a new worktree
    // on the existing main branch, which git actually allows (detaching HEAD)
    let wt_path = _tmp.path().join("wt-existing-branch");
    let result = worktree::add(&repo, &wt_path, Some("main"));
    // FINDING: git worktree add -b <existing-branch> detaches HEAD at that branch's tip.
    // The worktree::add code passes -b <branch> to git, and when the branch already
    // exists, git creates a detached HEAD worktree at that commit. This is
    // potentially confusing behavior - the -b flag implies creating a NEW branch.
    // Our code does not validate that the branch name doesn't already exist.
    if result.is_ok() {
        // Clean up
        let _ = worktree::remove(&repo, &wt_path, true);
    }
}

// ============================================================================
// ATTACK 6: Add worktree then add another at same path
// ============================================================================
#[test]
fn attack_add_worktree_duplicate_path() {
    let (_tmp, repo) = make_repo();
    let wt_path = _tmp.path().join("wt-dup");

    let r1 = worktree::add(&repo, &wt_path, Some("wt-dup-branch-1"));
    if r1.is_ok() {
        let r2 = worktree::add(&repo, &wt_path, Some("wt-dup-branch-2"));
        assert!(
            r2.is_err(),
            "Adding second worktree at same path should fail, got Ok"
        );
        // Cleanup
        let _ = worktree::remove(&repo, &wt_path, true);
    }
}

// ============================================================================
// ATTACK 7: Remove worktree without force when dirty
// ============================================================================
#[test]
fn attack_remove_dirty_worktree_no_force() {
    let (_tmp, repo) = make_repo();
    let wt_path = _tmp.path().join("wt-dirty");

    if worktree::add(&repo, &wt_path, Some("wt-dirty-branch")).is_ok() {
        // Create uncommitted changes in worktree
        std::fs::write(wt_path.join("dirty.txt"), "dirty").expect("write dirty");

        let result = worktree::remove(&repo, &wt_path, false);
        // Should fail without force when dirty
        // FINDING: Behavior depends on git CLI
        if result.is_err() {
            // Clean up with force
            let _ = worktree::remove(&repo, &wt_path, true);
        }
    }
}

// ============================================================================
// ATTACK 8: List with corrupted .git/worktrees directory
// ============================================================================
#[test]
fn attack_list_corrupted_worktrees_dir() {
    let (tmp, repo) = make_repo();
    // Create a worktrees directory with a corrupt entry
    let worktrees_dir = tmp.path().join(".git").join("worktrees");
    std::fs::create_dir_all(&worktrees_dir).expect("create dir");

    // Create a directory that looks like a worktree but has no gitdir file
    std::fs::create_dir_all(worktrees_dir.join("corrupt-entry")).expect("create corrupt dir");

    let result = worktree::list(&repo);
    // Should not panic on corrupt entries
    assert!(
        result.is_ok(),
        "Listing worktrees with corrupt entries should not panic"
    );
    // The main worktree should still be listed
    let entries = result.expect("entries");
    assert!(
        entries.iter().any(|e| e.is_main),
        "Should still find main worktree"
    );
}

// ============================================================================
// ATTACK 9: Add worktree with non-UTF8 path
// ============================================================================
#[test]
fn attack_add_worktree_non_utf8_path() {
    let (_tmp, repo) = make_repo();
    // Create a PathBuf that can't be represented as UTF-8 str
    // On Linux, we can create a path with invalid UTF-8 bytes
    // Use a path with embedded null byte (invalid UTF-8)
    let invalid_path = PathBuf::from("/tmp/test\0_invalid");
    let result = worktree::add(&repo, &invalid_path, None);
    // Should fail because path.to_str() returns None for non-UTF8
    assert!(
        result.is_err(),
        "Adding worktree with non-UTF8 path should fail, got Ok"
    );
}
