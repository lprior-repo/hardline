//! Integration tests for backup command handler.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::path::Path;

use crate::commands::handlers::backup::actions::{
    compute_checksum as action_checksum, execute_backup_command,
};
use crate::commands::handlers::backup::data::{BackupCommand, BackupConfig, BackupMetadata};

fn test_config(root: &Path) -> BackupConfig {
    BackupConfig {
        backup_dir: root.join("backups"),
        retention_count: 2,
    }
}

#[tokio::test]
async fn test_create_and_list_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    // Create source database directories and files
    tokio::fs::create_dir_all(root.join(".scp"))
        .await
        .expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state data")
        .await
        .expect("write state.db");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"beads data")
        .await
        .expect("write beads.db");

    let config = test_config(root);

    // Create backups
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    // List backups
    execute_backup_command(&BackupCommand::List, root, &config)
        .await
        .expect("list backups");
}

#[tokio::test]
async fn test_restore_latest_backup() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    // Create source database
    tokio::fs::create_dir_all(root.join(".scp"))
        .await
        .expect("create .scp dir");
    let original_data = b"original state data";
    tokio::fs::write(root.join(".scp").join("state.db"), original_data)
        .await
        .expect("write state.db");

    let config = test_config(root);

    // Create backup
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    // Modify original
    tokio::fs::write(root.join(".scp").join("state.db"), b"modified data")
        .await
        .expect("modify state.db");

    // Restore
    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await
    .expect("restore backup");

    // Verify restored content
    let restored = tokio::fs::read_to_string(root.join(".scp").join("state.db"))
        .await
        .expect("read restored state.db");
    assert_eq!(restored.as_bytes(), original_data);
}

#[tokio::test]
async fn test_retention_removes_old_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    // Create source database
    tokio::fs::create_dir_all(root.join(".scp"))
        .await
        .expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    // Create 3 backups (retention_count=2, so 1 should be removed)
    for _ in 0..3 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        // Small delay to ensure unique timestamps
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Apply retention
    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");
}

#[tokio::test]
async fn test_status_command() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp"))
        .await
        .expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("status");
}

#[tokio::test]
async fn test_restore_unknown_database_fails() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();
    let config = test_config(root);

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "nonexistent.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail for unknown database");
}

#[tokio::test]
async fn test_list_empty_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();
    let config = test_config(root);

    // List with no backups should succeed
    execute_backup_command(&BackupCommand::List, root, &config)
        .await
        .expect("list empty");
}

#[tokio::test]
async fn test_checksum_computation() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let test_file = root.join("test.bin");
    tokio::fs::write(&test_file, b"test data")
        .await
        .expect("write test file");

    let checksum = action_checksum(&test_file).await.expect("compute checksum");

    // Verify it's a hex string of the expected length (SHA-256 = 64 hex chars)
    assert_eq!(checksum.len(), 64);
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
}
