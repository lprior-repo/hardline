//! Tests for gix branch switch functionality

use scp_vcs::gix::{branch, repository};
use tempfile::TempDir;

#[test]
#[allow(clippy::needless_borrows_for_generic_args)]
fn test_gix_branch_switch_works() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo using the wrapper
    let _repo = repository::init(&repo_path).unwrap();

    // The gix::branch::switch should work on an initialized repo
    // This tests that the function can be called
    let mut repo = repository::open(&repo_path).unwrap();

    // Reflog writes need a committer; a fresh temp repo has none
    // configured, so set one in-memory before creating any refs.
    {
        let mut cfg = repo.config_snapshot_mut();
        cfg.set_value(&gix::config::tree::User::NAME, "test")
            .unwrap();
        cfg.set_value(&gix::config::tree::User::EMAIL, "test@example.com")
            .unwrap();
    }

    // Build a real root commit via gix. The old fixture pointed
    // refs/heads/testbranch at a hardcoded empty-tree id that was never
    // written to this repo's object store, so the branch-ref creation
    // silently failed (`.ok()`) and switch failed with InvalidRef.
    let tree_id = repo
        .write_object(&gix::objs::Tree::empty())
        .unwrap()
        .detach();
    // gix-actor 0.38 keeps SignatureRef.time as the raw timestamp string.
    let sig = gix::actor::SignatureRef {
        name: "test".into(),
        email: "test@example.com".into(),
        time: "0 +0000",
    };
    let commit_id = repo
        .commit_as(
            sig,
            sig,
            "HEAD",
            "initial commit",
            tree_id,
            std::iter::empty::<gix::ObjectId>(),
        )
        .unwrap()
        .detach();

    // Create refs/heads/testbranch at the real commit
    repo.reference(
        "refs/heads/testbranch",
        commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "create test branch",
    )
    .unwrap();

    // Try to switch branches - this will fail if implementation uses CLI
    let result = branch::switch(&repo, "testbranch", false);

    // The key test: does this use gix or CLI?
    assert!(
        result.is_ok(),
        "branch::switch should use gix, not CLI: {:?}",
        result
    );
}
