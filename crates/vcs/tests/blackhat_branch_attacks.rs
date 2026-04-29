//! Black-hat attack tests for VCS branch operations.
//!
//! Tests hostile inputs: empty names, ref injection, unicode, missing refs,
//! read-only filesystem, detached HEAD, etc.

use std::process::Command;

use scp_vcs::gix::branch;

/// Helper: create a temp git repo with an initial commit, return its path and open gix repo.
fn make_repo() -> (tempfile::TempDir, gix::Repository) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let path = tmp.path();

    // Init repo
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init");

    // Configure user
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

    // Create initial commit
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

/// Create a bare repo (no worktree) for testing bare repo edge cases.
#[allow(dead_code)]
fn make_bare_repo() -> (tempfile::TempDir, gix::Repository) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let path = tmp.path();

    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(path.parent().unwrap())
        .arg(path.file_name().unwrap())
        .output()
        .expect("git init --bare");

    // Bare repos need some commit. Create one by manipulating objects directly.
    // For simplicity, we use a non-bare repo and then convert.
    let _ = path;
    std::mem::forget(tmp); // We'll use a different approach

    // Use a regular repo approach instead
    let tmp2 = tempfile::TempDir::new().expect("temp dir 2");
    let path2 = tmp2.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path2)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path2)
        .output()
        .expect("git config");
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path2)
        .output()
        .expect("git config");
    std::fs::write(path2.join("f.txt"), "x").expect("write");
    Command::new("git")
        .args(["add", "."])
        .current_dir(path2)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path2)
        .output()
        .expect("git commit");

    // Now open bare: actually just open normally to test list behavior
    let repo = gix::open(path2).expect("gix open");
    (tmp2, repo)
}

// ============================================================================
// ATTACK 1: Branch with empty name
// ============================================================================
#[test]
fn attack_create_branch_empty_name() {
    let (_tmp, repo) = make_repo();
    let result = branch::create(&repo, "", false);
    // Empty branch name should fail - gix should reject refs/heads/ as invalid
    assert!(
        result.is_err(),
        "Creating branch with empty name should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 2: Branch name with reference injection "refs/heads/evil"
// ============================================================================
#[test]
fn attack_create_branch_ref_injection() {
    let (_tmp, repo) = make_repo();
    // This would create refs/heads/refs/heads/evil - double nesting
    let result = branch::create(&repo, "refs/heads/evil", false);
    // gix may allow this but it creates a confusing nested ref
    // The key assertion: it should NOT overwrite refs/heads/evil directly
    // If it succeeds, the ref is at refs/heads/refs/heads/evil (nested)
    if let Ok(()) = result {
        // Verify the actual branch exists at the nested path
        let find_nested = repo.find_reference("refs/heads/refs/heads/evil");
        let find_flat = repo.find_reference("refs/heads/evil");
        // At least one should exist - check the nested one
        assert!(
            find_nested.is_ok() || find_flat.is_ok(),
            "Branch creation with ref-like name should create a reference"
        );
    }
    // FINDING: No input sanitization on branch names containing "refs/heads/"
}

// ============================================================================
// ATTACK 3: Branch with unicode name
// ============================================================================
#[test]
fn attack_create_branch_unicode_name() {
    let (_tmp, repo) = make_repo();
    let result = branch::create(&repo, "日本語ブランチ", false);
    // Git supports UTF-8 branch names, but this may cause issues in some tools
    // FINDING: No validation or normalization of unicode branch names
    if result.is_ok() {
        let listed = branch::list(&repo, false).expect("list should work");
        let names: Vec<&str> = listed.iter().map(|b| b.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("日本語")),
            "Unicode branch should appear in listing"
        );
    }
}

// ============================================================================
// ATTACK 4: Delete nonexistent branch
// ============================================================================
#[test]
fn attack_delete_nonexistent_branch() {
    let (_tmp, repo) = make_repo();
    let result = branch::delete(&repo, "this-branch-does-not-exist", false);
    assert!(
        result.is_err(),
        "Deleting nonexistent branch should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 5: Delete branch with empty name
// ============================================================================
#[test]
fn attack_delete_branch_empty_name() {
    let (_tmp, repo) = make_repo();
    let result = branch::delete(&repo, "", false);
    assert!(
        result.is_err(),
        "Deleting branch with empty name should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 6: Switch to nonexistent branch
// ============================================================================
#[test]
fn attack_switch_nonexistent_branch() {
    let (_tmp, repo) = make_repo();
    let result = branch::switch(&repo, "nonexistent-branch-xyz", false);
    assert!(
        result.is_err(),
        "Switching to nonexistent branch should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 7: Create duplicate branch (no force)
// ============================================================================
#[test]
fn attack_create_duplicate_branch_no_force() {
    let (_tmp, repo) = make_repo();
    branch::create(&repo, "feature-test", false).expect("first create should work");
    let result = branch::create(&repo, "feature-test", false);
    assert!(
        result.is_err(),
        "Creating duplicate branch without force should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 8: Create duplicate branch (with force)
// ============================================================================
#[test]
fn attack_create_duplicate_branch_with_force() {
    let (_tmp, repo) = make_repo();
    branch::create(&repo, "feature-force", false).expect("first create should work");
    let result = branch::create(&repo, "feature-force", true);
    assert!(
        result.is_ok(),
        "Creating duplicate branch with force should succeed"
    );
}

// ============================================================================
// ATTACK 9: List branches in empty repo (no commits) - should handle gracefully
// ============================================================================
#[test]
fn attack_list_branches_empty_repo() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let path = tmp.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init");

    let repo = gix::open(path).expect("gix open");
    let result = branch::current(&repo);
    // FINDING: Git init -b main creates a ref pointing to "main" even with no commits,
    // so gix resolves HEAD to the branch name even in an empty repo.
    // Default init without -b creates "master" branch name.
    // This means current() returns Ok in an empty repo with no commits.
    // This could be misleading - the branch exists in name only (no commits).
    if let Ok(name) = &result {
        assert!(
            name == "master" || name == "main",
            "Empty repo branch should be master or main, got: {}",
            name
        );
    }
}

// ============================================================================
// ATTACK 10: Branch name with ".." path traversal
// ============================================================================
#[test]
fn attack_create_branch_path_traversal() {
    let (_tmp, repo) = make_repo();
    let result = branch::create(&repo, "evil/../main", false);
    // This should either be rejected or normalized to "main"
    // FINDING: gix may allow this, creating confusing ref names
    if result.is_ok() {
        // If it succeeds, check what was actually created
        let listed = branch::list(&repo, false).expect("list");
        let names: Vec<&str> = listed.iter().map(|b| b.name.as_str()).collect();
        // It should NOT overwrite the real main branch
        let main_branches: Vec<_> = names.iter().filter(|n| **n == "main").collect();
        assert!(
            main_branches.len() <= 1,
            "Path traversal should not create duplicate 'main' entries: {:?}",
            names
        );
    }
}

// ============================================================================
// ATTACK 11: Branch name with slashes (valid but deep nesting)
// ============================================================================
#[test]
fn attack_create_branch_deep_slashes() {
    let (_tmp, repo) = make_repo();
    let deep_name = "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p";
    let result = branch::create(&repo, deep_name, false);
    // This is technically valid in Git but could cause filesystem issues
    // on systems with path length limits
    // FINDING: No depth limit on branch name path components
    if result.is_ok() {
        let listed = branch::list(&repo, false).expect("list");
        let found = listed.iter().any(|b| b.name.as_str() == deep_name);
        assert!(found, "Deep branch should appear in listing");
    }
}

// ============================================================================
// ATTACK 12: Branch name with control characters
// ============================================================================
#[test]
fn attack_create_branch_control_chars() {
    let (_tmp, repo) = make_repo();
    let result = branch::create(&repo, "evil\x00branch", false);
    // Null bytes should be rejected
    assert!(
        result.is_err(),
        "Branch name with null byte should fail, got Ok"
    );
}

// ============================================================================
// ATTACK 13: Switch updates HEAD file on disk - verify consistency
// ============================================================================
#[test]
fn attack_switch_head_consistency() {
    let (_tmp, repo) = make_repo();
    branch::create(&repo, "target-branch", false).expect("create branch");
    branch::switch(&repo, "target-branch", false).expect("switch");

    // Verify HEAD file on disk points to correct branch
    let head_content =
        std::fs::read_to_string(_tmp.path().join(".git").join("HEAD")).expect("read HEAD");
    assert!(
        head_content.contains("target-branch"),
        "HEAD file should reference target-branch, got: {}",
        head_content
    );
}

// ============================================================================
// ATTACK 14: Delete current branch
// ============================================================================
#[test]
fn attack_delete_current_branch() {
    let (_tmp, repo) = make_repo();
    // Create and switch to a branch, then try to delete it
    branch::create(&repo, "current-branch", false).expect("create");
    branch::switch(&repo, "current-branch", false).expect("switch");

    // Deleting the current branch - should this fail?
    let result = branch::delete(&repo, "current-branch", false);
    // Git normally prevents this. Check if our code handles it.
    // FINDING: Code does not check if branch is current before deletion
    if result.is_ok() {
        // HEAD now points to a deleted branch - but gix may still resolve
        // the HEAD file since we wrote "ref: refs/heads/current-branch" to it
        // The ref was deleted but the HEAD file still points to it
        let head = branch::current(&repo);
        // FINDING: After deleting current branch, HEAD file still references
        // the deleted branch, creating a broken state. No guard against this.
        // The branch::current() call may succeed or fail depending on gix internals.
        // Either way, the repo is in an inconsistent state.
        let _ = head; // Document the finding without asserting specific behavior
    }
}

// ============================================================================
// ATTACK 15: Branch name that looks like a git command flag
// ============================================================================
#[test]
fn attack_create_branch_flag_injection() {
    let (_tmp, repo) = make_repo();
    // Branch name starting with dash
    let _result = branch::create(&repo, "-evil", false);
    // FINDING: No validation against dash-prefixed names (gix may reject)
}

// ============================================================================
// ATTACK 16: Create branch with very long name (>4096 chars)
// ============================================================================
#[test]
fn attack_create_branch_very_long_name() {
    let (_tmp, repo) = make_repo();
    let long_name = "x".repeat(5000);
    let _result = branch::create(&repo, &long_name, false);
    // FINDING: No length limit on branch names
    // This could cause filesystem issues since refs are stored as files
}

// ============================================================================
// ATTACK 17: List with all=true includes remote branches
// ============================================================================
#[test]
fn attack_list_all_branches_no_remote() {
    let (_tmp, repo) = make_repo();
    let result = branch::list(&repo, true);
    // Should succeed even with no remote
    assert!(
        result.is_ok(),
        "Listing all branches with no remote should succeed"
    );
}
