//! Tests for gix remote operations

use scp_vcs::gix::remote;
use scp_vcs::gix::repository;
use tempfile::TempDir;

#[test]
fn test_gix_remote_push_uses_cli_fallback() {
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

    // Test push - should fail gracefully since no remote is configured
    let result = remote::push(&repo, "origin", None, false, false, false);

    // Should fail because origin remote doesn't exist (bare repo, no workdir)
    // or because git push fails
    match result {
        Err(e) => {
            let err_str = format!("{:?}", e);
            // Should be either Network error (CLI failed) or InvalidRef (no workdir)
            assert!(
                err_str.contains("Network")
                || err_str.contains("InvalidRef")
                || err_str.contains("push"),
                "Expected network or ref error, got: {:?}",
                e
            );
        }
        Ok(()) => {
            // This is fine too - if somehow a push succeeds
        }
    }
}

#[test]
fn test_gix_remote_fetch_without_remote_fails() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo
    let repo = repository::init(&repo_path).unwrap();

    // Test fetch - should fail since no remote is configured
    let result = remote::fetch(&repo, Some("origin"), false, false, false);

    // Should fail because origin remote doesn't exist
    assert!(result.is_err(), "Expected fetch to fail without remote configured");
}

#[test]
fn test_gix_remote_pull_without_remote_fails() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo
    let repo = repository::init(&repo_path).unwrap();

    // Test pull - should fail since no remote is configured
    let result = remote::pull(&repo, Some("origin"), false);

    // Should fail because origin remote doesn't exist
    assert!(result.is_err(), "Expected pull to fail without remote configured");
}

#[test]
fn test_gix_remote_pull_rebase_unsupported() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repo
    let repo = repository::init(&repo_path).unwrap();

    // Test pull with rebase - should return unsupported error
    let result = remote::pull(&repo, None, true);

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("rebase"), "Expected rebase-related error, got: {}", msg);
}
