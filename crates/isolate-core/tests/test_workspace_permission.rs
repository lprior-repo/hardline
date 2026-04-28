#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

mod common;

use isolate_core::jj_operation_sync::create_workspace_synced;

#[tokio::test]
#[allow(clippy::expect_used)]
async fn test_workspace_creation_with_permission_denied() {
    // This test verifies that permission errors from JJ are handled gracefully

    // Set up a real jj repo for the test
    let repo_temp = common::setup_test_repo().expect("Failed to setup test repo");
    let repo_root = repo_temp.path().to_path_buf();

    // Create a directory that we will make inaccessible
    let restricted_parent = repo_root.join("restricted_dir");
    std::fs::create_dir_all(&restricted_parent).expect("Failed to create restricted parent");

    let bad_path = restricted_parent.join("test-workspace");

    // On Unix, we can use chmod 000 to deny permissions to the parent directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o000))
            .expect("Failed to set restricted permissions");

        // If we can still read the directory, we are likely running as root,
        // and the chmod 000 will not prevent access. Skip the test in this case.
        if std::fs::read_dir(&restricted_parent).is_ok() {
            println!("Skipping permission test: running as root or chmod 000 not effective");
            let _ = std::fs::set_permissions(
                &restricted_parent,
                std::fs::Permissions::from_mode(0o755),
            );
            return;
        }
    }

    // If not on Unix, we still use a path that is likely to fail, though less reliably
    #[cfg(not(unix))]
    let bad_path = Path::new("Z:\\nonexistent\\path\\test-workspace");

    let result = create_workspace_synced("test-perm-denied", &bad_path, &repo_root).await;

    // Cleanup permissions so TempDir can be deleted
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ =
            std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o755));
    }

    assert!(
        result.is_err(),
        "Should fail with permission denied for path {:?}",
        bad_path
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();

    // Error should mention the problem - may be permission error, JJ error, or lock issue
    assert!(
        err_msg.contains("permission")
            || err_msg.contains("jj")
            || err_msg.contains("lock")
            || err_msg.contains("timeout")
            || err_msg.contains("access denied")
            || err_msg.contains("failed to create directory"),
        "Error should mention permission, JJ, or lock issue: {err_msg}"
    );

    println!("✓ Test passed: Error handled gracefully: {err_msg}");

    // Cleanup: restore directory permissions before temp dir drop.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ =
            std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o755));
    }
}
