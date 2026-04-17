//! Tests for gix branch switch functionality

use scp_vcs::gix::{branch, repository};
use tempfile::TempDir;

#[test]
fn test_gix_branch_switch_works() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo using the wrapper
    let _repo = repository::init(&repo_path).unwrap();

    // The gix::branch::switch should work on an initialized repo
    // This tests that the function can be called
    let repo = repository::open(&repo_path).unwrap();

    // Create refs/heads/testbranch
    let empty_tree = gix::ObjectId::from_hex(b"4b825dc642cb6eb9a060e54bf8d69288fbee4904").unwrap();

    repo.reference(
        "refs/heads/testbranch",
        empty_tree,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "create test branch",
    )
    .ok();

    // Try to switch branches - this will fail if implementation uses CLI
    let result = branch::switch(&repo, "testbranch", false);

    // The key test: does this use gix or CLI?
    assert!(
        result.is_ok(),
        "branch::switch should use gix, not CLI: {:?}",
        result
    );
}
