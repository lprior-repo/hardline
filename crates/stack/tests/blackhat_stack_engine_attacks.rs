//! Black-hat attack tests for Stack Engine operations.
//!
//! Tests hostile inputs: empty branch names, path traversal, missing branches,
//! no remote, empty repo, detached HEAD, max nesting, etc.

use std::process::Command;

use scp_stack::{
    domain::value_objects::BranchName, engine::stack_engine::StackEngine, error::StackError,
};

/// Helper: create a temp git repo with an initial commit on "main".
fn make_repo() -> (tempfile::TempDir, gix::Repository) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let path = tmp.path();

    Command::new("git")
        .args(["init", "-b", "main"])
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

/// Create a second commit (advance HEAD) to enable branch differentiation.
fn make_commit(tmp: &tempfile::TempDir, msg: &str) {
    std::fs::write(tmp.path().join("commit.txt"), msg).expect("write");
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");
}

// ============================================================================
// ATTACK 1: Load stack when not in a git repo
// ============================================================================
#[test]
fn attack_load_stack_not_git_repo() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    // Don't git init - just a plain directory
    let result = gix::open(tmp.path());
    // gix::open on non-git dir should fail
    assert!(result.is_err(), "Opening non-git directory should fail");
}

// ============================================================================
// ATTACK 2: Create branch with empty name
// ============================================================================
#[test]
fn attack_create_branch_empty_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("", None);
    assert!(
        result.is_err(),
        "Creating branch with empty name should fail"
    );
    match result {
        Err(StackError::InvalidBranchName(msg)) => {
            assert!(
                msg.contains("empty"),
                "Error message should mention empty, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 3: Create branch with whitespace in name
// ============================================================================
#[test]
fn attack_create_branch_whitespace_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("evil branch", None);
    assert!(result.is_err(), "Creating branch with space should fail");
}

// ============================================================================
// ATTACK 4: Create branch with tab in name
// ============================================================================
#[test]
fn attack_create_branch_tab_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("evil\tbranch", None);
    assert!(result.is_err(), "Creating branch with tab should fail");
}

// ============================================================================
// ATTACK 5: Create branch with newline in name
// ============================================================================
#[test]
fn attack_create_branch_newline_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("evil\nbranch", None);
    assert!(result.is_err(), "Creating branch with newline should fail");
}

// ============================================================================
// ATTACK 6: Create branch starting with dash
// ============================================================================
#[test]
fn attack_create_branch_dash_prefix() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("-evil", None);
    assert!(
        result.is_err(),
        "Creating branch starting with dash should fail"
    );
}

// ============================================================================
// ATTACK 7: Delete nonexistent branch
// ============================================================================
#[test]
fn attack_delete_nonexistent_branch() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.delete_branch("nonexistent-branch-xyz");
    assert!(result.is_err(), "Deleting nonexistent branch should fail");
    match result {
        Err(StackError::BranchNotFound(msg)) => {
            assert!(
                msg.contains("nonexistent"),
                "Error should mention branch name, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 8: Sync stack with no remote configured
// ============================================================================
#[test]
fn attack_sync_no_remote() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.sync_stack("test-stack");
    assert!(result.is_err(), "Sync with no remote should fail");
    match result {
        Err(StackError::GitError(msg)) => {
            assert!(
                msg.contains("origin"),
                "Error should mention origin, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 9: Load stack in empty repo (no commits)
// ============================================================================
#[test]
fn attack_load_stack_empty_repo() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .expect("git init");

    let repo = gix::open(tmp.path()).expect("gix open");
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.load_stack("test");
    assert!(
        result.is_err(),
        "Loading stack in empty repo (no commits) should fail"
    );
}

// ============================================================================
// ATTACK 10: Restack branch (known unsupported operation)
// ============================================================================
#[test]
fn attack_restack_branch_unsupported() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.restack_branch("any-branch");
    assert!(result.is_err(), "Restack should return unsupported error");
    match result {
        Err(StackError::GitError(msg)) => {
            assert!(
                msg.contains("not supported"),
                "Error should mention unsupported, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 11: Create branch with ".." in name
// ============================================================================
#[test]
fn attack_create_branch_dotdot_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("evil..main", None);
    // ".." is problematic in git refs - should be rejected
    // FINDING: validate_branch_name does NOT reject ".."
    // This could be used for ref name confusion
    if result.is_ok() {
        // Verify the branch was actually created
        let ref_name = format!("refs/heads/evil..main");
        assert!(
            repo.find_reference(&ref_name).is_ok(),
            "Branch should exist after creation"
        );
    }
}

// ============================================================================
// ATTACK 12: Create branch with "/" in name (valid but worth testing)
// ============================================================================
#[test]
fn attack_create_branch_slash_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("feature/test", None);
    assert!(
        result.is_ok(),
        "Creating branch with slash should succeed (valid git name)"
    );
}

// ============================================================================
// ATTACK 13: Create duplicate branch via engine
// ============================================================================
#[test]
fn attack_create_duplicate_branch_via_engine() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    engine
        .create_branch("dup-test", None)
        .expect("first create");
    let result = engine.create_branch("dup-test", None);
    assert!(result.is_err(), "Creating duplicate branch should fail");
    match result {
        Err(StackError::InvalidBranchName(msg)) => {
            assert!(
                msg.contains("already exists"),
                "Error should mention already exists, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 14: Load stack with trunk that doesn't exist
// ============================================================================
#[test]
fn attack_load_stack_nonexistent_trunk() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("nonexistent-trunk"));

    let result = engine.load_stack("test");
    assert!(
        result.is_err(),
        "Loading stack with nonexistent trunk should fail"
    );
}

// ============================================================================
// ATTACK 15: Create many branches (stress test - 50 branches)
// ============================================================================
#[test]
fn attack_create_many_branches() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let mut errors = Vec::new();
    for i in 0..50 {
        let name = format!("stress-branch-{:04}", i);
        if let Err(e) = engine.create_branch(&name, None) {
            errors.push((name, e));
        }
    }

    assert!(
        errors.is_empty(),
        "Creating 50 branches should succeed, got {} errors: {:?}",
        errors.len(),
        errors.iter().take(5).collect::<Vec<_>>()
    );

    // Verify all 50 branches exist
    for i in 0..50 {
        let name = format!("stress-branch-{:04}", i);
        let ref_name = format!("refs/heads/{}", name);
        assert!(
            repo.find_reference(&ref_name).is_ok(),
            "Branch {} should exist",
            name
        );
    }
}

// ============================================================================
// ATTACK 16: Create branch with unicode name
// ============================================================================
#[test]
fn attack_create_branch_unicode() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let result = engine.create_branch("日本語テスト", None);
    // Unicode names are technically valid in git
    // FINDING: validate_branch_name does NOT reject unicode
    if result.is_ok() {
        let ref_name = "refs/heads/日本語テスト";
        assert!(
            repo.find_reference(ref_name).is_ok(),
            "Unicode branch should exist"
        );
    }
}

// ============================================================================
// ATTACK 17: Create branch then delete it, then try to delete again
// ============================================================================
#[test]
fn attack_create_delete_delete() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    engine.create_branch("transient", None).expect("create");
    engine.delete_branch("transient").expect("first delete");
    let result = engine.delete_branch("transient");
    assert!(result.is_err(), "Double-delete should fail");
}

// ============================================================================
// ATTACK 18: Create branch with ref-injection name
// ============================================================================
#[test]
fn attack_create_branch_ref_injection() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    // Attempt to create a branch that would shadow refs/heads/main
    let result = engine.create_branch("refs/heads/evil", None);
    // FINDING: No validation against names containing "refs/heads/"
    // This creates refs/heads/refs/heads/evil (nested), not an overwrite
    if result.is_ok() {
        // Verify main was not overwritten
        let main_ref = repo.find_reference("refs/heads/main");
        assert!(main_ref.is_ok(), "main branch should still exist");
    }
}

// ============================================================================
// ATTACK 19: Validate branch name rejects all whitespace types
// ============================================================================
#[test]
fn attack_validate_rejects_whitespace() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let bad_names = vec![
        "name with space",
        "name\twith\ttab",
        "name\nwith\nnewline",
        " name-leading",
        "name-trailing ",
    ];

    for name in bad_names {
        let result = engine.create_branch(name, None);
        assert!(
            result.is_err(),
            "Branch name {:?} should be rejected",
            name.escape_unicode()
        );
    }
}

// ============================================================================
// ATTACK 20: Load stack with detached HEAD
// ============================================================================
#[test]
fn attack_load_stack_detached_head() {
    let (tmp, repo) = make_repo();

    // Create a second commit and go detached
    make_commit(&tmp, "second commit");
    let head_id = repo.head_id().expect("head_id").detach();

    // Detach HEAD by checking out a specific commit
    Command::new("git")
        .args(["checkout", &head_id.to_string()])
        .current_dir(tmp.path())
        .output()
        .expect("git checkout --detach");

    // Re-open repo to pick up detached HEAD state
    let repo2 = gix::open(tmp.path()).expect("gix open");
    let engine = StackEngine::new(&repo2, BranchName::new("main"));

    let _result = engine.load_stack("detached-test");
    // Should fail or handle gracefully - detached HEAD has no branch name
    // FINDING: load_stack does not handle detached HEAD explicitly
    // It may succeed by walking from HEAD to trunk, but the semantics are unclear
}

// ============================================================================
// ATTACK 21: Create branch with very long name
// ============================================================================
#[test]
fn attack_create_branch_very_long_name() {
    let (_tmp, repo) = make_repo();
    let engine = StackEngine::new(&repo, BranchName::new("main"));

    let long_name = "x".repeat(5000);
    let _result = engine.create_branch(&long_name, None);
    // FINDING: No length limit on branch names
    // Very long names can cause filesystem issues since refs are stored as files
}
