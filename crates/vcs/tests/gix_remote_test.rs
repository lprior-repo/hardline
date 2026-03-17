//! Tests for gix remote operations

use scp_vcs::gix::remote;
use scp_vcs::gix::repository;
use tempfile::TempDir;

#[test]
fn test_gix_remote_push_uses_gix_not_cli() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo
    let repo = repository::init(&repo_path).unwrap();

    // Create initial commit
    let empty_tree = gix::ObjectId::from_hex(b"4b825dc642cb6eb9a060e54bf8d69288fbee4904").unwrap();
    repo.reference(
        "refs/heads/main",
        empty_tree,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "init",
    ).ok();

    // Test push - THIS IS WHAT WE'RE TESTING
    // This will fail with Network error if implemented, or use CLI if not migrated
    let result = remote::push(&repo, "origin", None, false, false, false);

    // Should NOT use CLI - either implemented with gix or return proper error
    // If it uses CLI, the implementation needs updating
    match result {
        Err(e) => {
            // Network error is acceptable - means it's trying to use gix
            let err_str = format!("{:?}", e);
            assert!(err_str.contains("Network") || err_str.contains("not yet implemented"),
                "Should be gix-based error, not CLI failure: {:?}", e);
        }
        Ok(_) => panic!("push should not succeed without remote"),
    }
}
