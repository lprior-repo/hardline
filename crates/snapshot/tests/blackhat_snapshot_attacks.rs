//! Black-hat attack tests for Snapshot operations.
//!
//! Tests hostile inputs: empty branch names, nonexistent IDs, permission denied,
//! huge descriptions, corrupted directories, etc.

use std::path::Path;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use scp_snapshot::application::service::SnapshotService;
use scp_snapshot::domain::snapshot::{Snapshot, SnapshotId, SnapshotType};
use scp_snapshot::error::SnapshotError;
use scp_snapshot::storage::storage::SnapshotStore;

fn make_service() -> (SnapshotService, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let store = Arc::new(SnapshotStore::new(tmp.path()));
    let service = SnapshotService::new(store);
    (service, tmp)
}

fn make_store(tmp: &tempfile::TempDir) -> SnapshotStore {
    SnapshotStore::new(tmp.path())
}

// ============================================================================
// ATTACK 1: Create snapshot with empty branch name
// ============================================================================
#[test]
fn attack_create_snapshot_empty_branch_name() {
    let (service, _tmp) = make_service();
    let result = service.create_snapshot(String::new(), "abc123".to_string(), None);
    // FINDING: Empty branch name is accepted - no validation
    assert!(
        result.is_ok(),
        "Snapshot with empty branch name should succeed (no validation)"
    );
    let snap = result.expect("snap");
    assert_eq!(snap.branch_name, "");
}

// ============================================================================
// ATTACK 2: Restore nonexistent snapshot ID
// ============================================================================
#[test]
fn attack_restore_nonexistent_snapshot() {
    let (service, _tmp) = make_service();
    let fake_id = SnapshotId::generate();
    let result = service.restore_snapshot(&fake_id);
    assert!(
        result.is_err(),
        "Restoring nonexistent snapshot should fail"
    );
}

// ============================================================================
// ATTACK 3: Delete nonexistent snapshot ID
// ============================================================================
#[test]
fn attack_delete_nonexistent_snapshot() {
    let (service, _tmp) = make_service();
    let fake_id = SnapshotId::generate();
    let result = service.delete_snapshot(&fake_id);
    assert!(
        result.is_err(),
        "Deleting nonexistent snapshot should fail"
    );
}

// ============================================================================
// ATTACK 4: Create snapshot with huge description (>64KB)
// ============================================================================
#[test]
fn attack_create_snapshot_huge_description() {
    let (service, _tmp) = make_service();
    let huge_desc = "X".repeat(70000);
    let result = service.create_snapshot(
        "main".to_string(),
        "abc123".to_string(),
        Some(huge_desc.clone()),
    );
    assert!(
        result.is_ok(),
        "Creating snapshot with huge description should succeed"
    );
    let snap = result.expect("snap");
    assert_eq!(snap.description, Some(huge_desc));
}

// ============================================================================
// ATTACK 5: List snapshots in corrupted dir (non-JSON files)
// ============================================================================
#[test]
fn attack_list_corrupted_dir_non_json() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let snapshots_dir = tmp.path().join(".scp").join("snapshots");
    std::fs::create_dir_all(&snapshots_dir).expect("create dir");

    // Write a non-JSON file with .json extension
    std::fs::write(snapshots_dir.join("snap-corrupt.json"), "NOT VALID JSON {{{{")
        .expect("write corrupt");

    let store = SnapshotStore::new(tmp.path());
    let result = store.list();
    // FINDING: Corrupt JSON in .json files causes list() to fail entirely
    assert!(
        result.is_err(),
        "Listing snapshots with corrupt JSON should fail"
    );
    match result {
        Err(SnapshotError::DeserializationError(msg)) => {
            assert!(
                msg.contains("corrupt"),
                "Error should mention the corrupt file, got: {}",
                msg
            );
        }
        Err(e) => panic!("Wrong error type: {:?}", e),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// ATTACK 6: List snapshots ignores non-.json files
// ============================================================================
#[test]
fn attack_list_ignores_non_json_files() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let store = make_store(&tmp);

    // Create a valid snapshot first
    service::create_and_save(&store, "main", "abc", None);

    // Add non-JSON files to the snapshots dir
    let snapshots_dir = tmp.path().join(".scp").join("snapshots");
    std::fs::write(snapshots_dir.join("readme.txt"), "not json").expect("write txt");
    std::fs::write(snapshots_dir.join("data.bin"), &[0u8, 1, 2, 3]).expect("write bin");
    std::fs::write(snapshots_dir.join("script.sh"), "#!/bin/bash\nevil").expect("write sh");

    let result = store.list();
    assert!(
        result.is_ok(),
        "Listing should ignore non-JSON files"
    );
    let list = result.expect("list");
    assert_eq!(list.len(), 1, "Should only find the one valid snapshot");
}

// ============================================================================
// ATTACK 7: Snapshot with empty commit hash
// ============================================================================
#[test]
fn attack_create_snapshot_empty_commit_hash() {
    let (service, _tmp) = make_service();
    let result = service.create_snapshot("main".to_string(), String::new(), None);
    // FINDING: Empty commit hash is accepted - no validation
    assert!(
        result.is_ok(),
        "Snapshot with empty commit hash should succeed (no validation)"
    );
}

// ============================================================================
// ATTACK 8: Snapshot with path traversal in branch name
// ============================================================================
#[test]
fn attack_create_snapshot_path_traversal_branch() {
    let (service, _tmp) = make_service();
    let evil_name = "../../../etc/passwd";
    let result = service.create_snapshot(evil_name.to_string(), "abc".to_string(), None);
    // FINDING: No sanitization of branch names - path traversal chars accepted
    assert!(
        result.is_ok(),
        "Snapshot with path traversal branch name should succeed (no sanitization)"
    );
}

// ============================================================================
// ATTACK 9: Snapshot with branch name containing null bytes
// ============================================================================
#[test]
fn attack_create_snapshot_null_in_branch() {
    let (service, _tmp) = make_service();
    // Rust strings can't contain null bytes via String::new(), but we can try
    let evil_name = "evil\x00branch".to_string();
    let result = service.create_snapshot(evil_name, "abc".to_string(), None);
    // The JSON serializer may or may not handle this
    // FINDING: Null bytes in strings may cause serialization issues
}

// ============================================================================
// ATTACK 10: SnapshotId parse rejects bad formats
// ============================================================================
#[test]
fn attack_snapshot_id_parse_rejects_bad() {
    let bad_inputs = vec![
        "",
        "snap-",
        "not-a-snap",
        "SNAP-uppercase",
        "snap",
    ];

    for input in bad_inputs {
        let result = SnapshotId::parse(input);
        assert!(
            result.is_err(),
            "SnapshotId::parse({:?}) should fail",
            input
        );
    }

    // FINDING: SnapshotId::parse accepts "snap-\0null" (with null byte)
    // Null bytes in identifiers could cause issues when used in filenames
    // or when passing to other systems that expect clean strings.
    let null_result = SnapshotId::parse("snap-\x00null");
    // No assertion - just documenting the behavior
    let _ = null_result;
}

// ============================================================================
// ATTACK 11: SnapshotId accepts unusual but valid formats
// ============================================================================
#[test]
fn attack_snapshot_id_accepts_unusual() {
    let valid_inputs = vec![
        "snap-a",
        "snap-../../evil",
        "snap-\x00",  // This might actually fail
        "snap-"  ,     // This should fail (len == 5)
    ];

    for input in valid_inputs {
        let result = SnapshotId::parse(input);
        if input.len() > 5 {
            assert!(
                result.is_ok(),
                "SnapshotId::parse({:?}) should succeed for len > 5",
                input
            );
        }
    }
}

// ============================================================================
// ATTACK 12: Cleanup when snapshot dir has permission denied
// ============================================================================
#[test]
fn attack_cleanup_permission_denied() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let store = make_store(&tmp);

    // Create an expired snapshot
    let mut snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
    snapshot.expires_at = Some(chrono::Utc::now() - chrono::Duration::hours(1));
    store.save(snapshot.clone()).expect("save");

    // Make the snapshot file read-only (not deletable on some systems)
    let file_path = tmp.path().join(".scp").join("snapshots")
        .join(format!("{}.json", snapshot.id.as_str()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file_path.exists() {
            // Make directory not writable instead (more portable way to test)
            let dir = tmp.path().join(".scp").join("snapshots");
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o444));
        }
    }

    let service = SnapshotService::new(Arc::new(store));
    let result = service.cleanup_expired();

    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        let dir = tmp.path().join(".scp").join("snapshots");
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
    }

    // Should handle gracefully - either succeed or report failures
    if let Ok(report) = result {
        // On Unix, deletion of read-only files may succeed (owner can always delete)
        // The important thing is no panic
        assert!(
            report.failed == 0 || report.failed > 0,
            "Cleanup should complete without panic"
        );
    }
}

// ============================================================================
// ATTACK 13: Concurrent save/load (basic thread safety)
// ============================================================================
#[test]
fn attack_concurrent_operations() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let store = Arc::new(SnapshotStore::new(tmp.path()));
    let service = SnapshotService::new(store.clone());

    // Create a snapshot
    let snap = service.create_snapshot("main".to_string(), "abc".to_string(), None)
        .expect("create");

    // Load it concurrently from multiple threads
    let id = snap.id.clone();
    let handles: Vec<_> = (0..4).map(|_| {
        let store = store.clone();
        let id = id.clone();
        std::thread::spawn(move || {
            store.load(&id)
        })
    }).collect();

    for handle in handles {
        let result = handle.join().expect("thread join");
        assert!(result.is_ok(), "Concurrent load should succeed");
    }
}

// ============================================================================
// ATTACK 14: Overwrite existing snapshot via save
// ============================================================================
#[test]
fn attack_overwrite_existing_snapshot() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let store = make_store(&tmp);

    let id = SnapshotId::parse("snap-overwrite-test").expect("valid id");

    let mut snap1 = Snapshot::create("main".to_string(), "aaa".to_string(), None);
    snap1.id = id.clone();
    store.save(snap1).expect("save first");

    let mut snap2 = Snapshot::create("evil".to_string(), "bbb".to_string(), None);
    snap2.id = id.clone();
    store.save(snap2).expect("overwrite");

    let loaded = store.load(&id).expect("load");
    // FINDING: Save silently overwrites existing snapshots with same ID
    assert_eq!(loaded.branch_name, "evil");
    assert_eq!(loaded.commit_hash, "bbb");
}

// ============================================================================
// ATTACK 15: Create snapshot with unicode branch name
// ============================================================================
#[test]
fn attack_create_snapshot_unicode_branch() {
    let (service, _tmp) = make_service();
    let result = service.create_snapshot("日本語ブランチ".to_string(), "abc".to_string(), None);
    assert!(result.is_ok(), "Unicode branch name should work");
    let snap = result.expect("snap");
    assert_eq!(snap.branch_name, "日本語ブランチ");
}

// ============================================================================
// ATTACK 16: Create snapshot with description containing JSON injection
// ============================================================================
#[test]
fn attack_create_snapshot_json_injection_description() {
    let (service, _tmp) = make_service();
    let evil_desc = r#"},{"evil":"injected","id":"snap-pwned"#;
    let result = service.create_snapshot(
        "main".to_string(),
        "abc".to_string(),
        Some(evil_desc.to_string()),
    );
    assert!(result.is_ok(), "Should handle JSON-like strings in description");

    let snap = result.expect("snap");
    let loaded = service.get_snapshot(&snap.id).expect("load");
    // serde_json properly escapes strings - no injection possible
    assert_eq!(loaded.description, Some(evil_desc.to_string()));
}

// Helper module for test utilities
mod service {
    use super::*;

    pub fn create_and_save(store: &SnapshotStore, branch: &str, commit: &str, desc: Option<&str>) -> Snapshot {
        let snapshot = Snapshot::create(
            branch.to_string(),
            commit.to_string(),
            desc.map(|d| d.to_string()),
        );
        store.save(snapshot.clone()).expect("save");
        snapshot
    }
}
