//! Integration tests for backup command handler.

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

fn test_config_with_retention(root: &Path, retention: usize) -> BackupConfig {
    BackupConfig {
        backup_dir: root.join("backups"),
        retention_count: retention,
    }
}

#[tokio::test]
async fn test_create_backup_full() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    assert!(backup_dir.exists(), "backup directory should exist");

    let mut has_backup = false;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read backup dir");
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            has_backup = true;
            assert!(path.exists(), "backup file should exist");
            let metadata = path.metadata().expect("metadata");
            assert_eq!(metadata.len(), 10, "backup size should match source");
        }
    }
    assert!(has_backup, "should have at least one backup file");
}

#[tokio::test]
async fn test_create_backup_multiple_databases() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state data")
        .await
        .expect("write state.db");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"beads data")
        .await
        .expect("write beads.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    assert!(
        root.join("backups").join("state.db").exists(),
        "state.db backup dir should exist"
    );
    assert!(
        root.join("backups").join("beads.db").exists(),
        "beads.db backup dir should exist"
    );
}

#[tokio::test]
async fn test_create_backup_skips_missing_database() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    assert!(root.join("backups").join("state.db").exists());
    assert!(
        !root.join("backups").join("beads.db").exists(),
        "beads.db backup dir should not exist"
    );
}

#[tokio::test]
async fn test_create_backup_metadata_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"test data 123")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut found_metadata = false;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            found_metadata = true;
            let json = tokio::fs::read_to_string(&path).await.expect("read metadata");
            let meta: BackupMetadata = serde_json::from_str(&json).expect("parse metadata");

            assert_eq!(meta.database_name, "state.db");
            assert_eq!(meta.size_bytes, 13);
            assert_eq!(meta.checksum.len(), 64);
            assert!(meta.created_at.timestamp() > 0);
        }
    }
    assert!(found_metadata, "should have metadata file");
}

#[tokio::test]
async fn test_list_empty_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();
    let config = test_config(root);

    execute_backup_command(&BackupCommand::List, root, &config)
        .await
        .expect("list empty");
}

#[tokio::test]
async fn test_list_single_database_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    execute_backup_command(&BackupCommand::List, root, &config).await.expect("list");
}

#[tokio::test]
async fn test_list_multiple_databases_sorted() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state data")
        .await
        .expect("write state.db");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"beads data")
        .await
        .expect("write beads.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    execute_backup_command(&BackupCommand::List, root, &config).await.expect("list");
}

#[tokio::test]
async fn test_list_backups_sorted_newest_first() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    for i in 0..3 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let backup_path = root.join(".scp").join("state.db");
        tokio::fs::write(&backup_path, format!("data {}", i).as_bytes())
            .await
            .expect("update data");
    }

    execute_backup_command(&BackupCommand::List, root, &config)
        .await
        .expect("list");
}

#[tokio::test]
async fn test_restore_latest_backup() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    let original_data = b"original state data";
    tokio::fs::write(root.join(".scp").join("state.db"), original_data)
        .await
        .expect("write state.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    tokio::fs::write(root.join(".scp").join("state.db"), b"modified data")
        .await
        .expect("modify state.db");

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

    let restored = tokio::fs::read(root.join(".scp").join("state.db"))
        .await
        .expect("read restored state.db");
    assert_eq!(restored, original_data);
}

#[tokio::test]
async fn test_restore_specific_timestamp() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"version1")
        .await
        .expect("write state.db");

    let config = test_config(root);

    // Create first backup
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    // Save the first backup's timestamp by reading its metadata
    let backup_dir = root.join("backups").join("state.db");
    let mut first_backup_meta = None;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let json = tokio::fs::read_to_string(&path).await.expect("read metadata");
            let meta: BackupMetadata = serde_json::from_str(&json).expect("parse metadata");
            first_backup_meta = Some(meta);
            break;
        }
    }

    let Some(first_meta) = first_backup_meta else {
        panic!("should have found first backup metadata");
    };

    // Modify and create second backup
    tokio::fs::write(root.join(".scp").join("state.db"), b"version2")
        .await
        .expect("modify state.db");
    // Sleep for 2 seconds to ensure different timestamp
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    // Modify database again
    tokio::fs::write(root.join(".scp").join("state.db"), b"version3")
        .await
        .expect("modify state.db");

    // Restore to first version using its timestamp
    let timestamp = first_meta.created_at.format("%Y%m%d-%H%M%S").to_string();
    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: Some(timestamp),
        },
        root,
        &config,
    )
    .await
    .expect("restore backup");

    let restored = tokio::fs::read(root.join(".scp").join("state.db"))
        .await
        .expect("read restored");
    assert_eq!(restored, b"version1");
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
async fn test_restore_no_backup_found() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail when no backups exist");
}

#[tokio::test]
async fn test_restore_specific_timestamp_not_found() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: Some("99999999-999999".to_string()),
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail when specific timestamp not found");
}

#[tokio::test]
async fn test_restore_beads_database() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    let beads_data = b"original beads data";
    tokio::fs::write(root.join(".scp").join("beads.db"), beads_data)
        .await
        .expect("write beads.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    tokio::fs::write(root.join(".scp").join("beads.db"), b"modified beads")
        .await
        .expect("modify beads.db");

    execute_backup_command(
        &BackupCommand::Restore {
            database: "beads.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await
    .expect("restore beads.db");

    let restored = tokio::fs::read(root.join(".scp").join("beads.db"))
        .await
        .expect("read restored");
    assert_eq!(restored, beads_data);
}

#[tokio::test]
async fn test_retention_removes_old_backups() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config_with_retention(root, 2);

    for i in 0..3 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        // Sleep 2 seconds to ensure unique timestamps
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let backup_path = root.join(".scp").join("state.db");
        tokio::fs::write(&backup_path, format!("data {}", i).as_bytes())
            .await
            .expect("update data");
    }

    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");

    let backup_dir = root.join("backups").join("state.db");
    let mut db_count = 0;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            db_count += 1;
        }
    }

    assert_eq!(db_count, 2, "should have exactly 2 backups after retention");
}

#[tokio::test]
async fn test_retention_with_high_retention_count() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config_with_retention(root, 10);

    for i in 0..3 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        // Sleep 2 seconds to ensure unique timestamps
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let backup_path = root.join(".scp").join("state.db");
        tokio::fs::write(&backup_path, format!("data {}", i).as_bytes())
            .await
            .expect("update data");
    }

    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");

    let backup_dir = root.join("backups").join("state.db");
    let mut db_count = 0;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            db_count += 1;
        }
    }

    assert_eq!(db_count, 3, "should have all 3 backups");
}

#[tokio::test]
async fn test_retention_multiple_databases() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"state")
        .await
        .expect("write state.db");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"beads")
        .await
        .expect("write beads.db");

    let config = test_config_with_retention(root, 2);

    for i in 0..3 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        // Sleep 2 seconds to ensure unique timestamps
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let backup_path = root.join(".scp").join("state.db");
        tokio::fs::write(&backup_path, format!("state {}", i).as_bytes())
            .await
            .expect("update state.db");
        let beads_path = root.join(".scp").join("beads.db");
        tokio::fs::write(&beads_path, format!("beads {}", i).as_bytes())
            .await
            .expect("update beads.db");
    }

    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");

    for db_name in &["state.db", "beads.db"] {
        let backup_dir = root.join("backups").join(db_name);
        let mut db_count = 0;
        let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                db_count += 1;
            }
        }
        assert_eq!(db_count, 2, "{db_name} should have 2 backups");
    }
}

#[tokio::test]
async fn test_status_command() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
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
async fn test_status_within_retention_limit() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config_with_retention(root, 10);

    for _ in 0..2 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("status");
}

#[tokio::test]
async fn test_status_over_retention_limit() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config_with_retention(root, 2);

    for _ in 0..5 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("status");
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

    assert_eq!(checksum.len(), 64);
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_checksum_deterministic() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let test_file = root.join("test.bin");
    let test_content = b"deterministic test content";
    tokio::fs::write(&test_file, test_content)
        .await
        .expect("write test file");

    let checksum1 = action_checksum(&test_file).await.expect("compute checksum 1");
    let checksum2 = action_checksum(&test_file).await.expect("compute checksum 2");

    assert_eq!(checksum1, checksum2, "checksums should be identical for same content");
}

#[tokio::test]
async fn test_checksum_different_content() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let file1 = root.join("test1.bin");
    let file2 = root.join("test2.bin");
    tokio::fs::write(&file1, b"content a").await.expect("write file1");
    tokio::fs::write(&file2, b"content b").await.expect("write file2");

    let checksum1 = action_checksum(&file1).await.expect("compute checksum 1");
    let checksum2 = action_checksum(&file2).await.expect("compute checksum 2");

    assert_ne!(checksum1, checksum2, "checksums should differ for different content");
}

#[tokio::test]
async fn test_checksum_empty_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let empty_file = root.join("empty.bin");
    tokio::fs::write(&empty_file, b"")
        .await
        .expect("write empty file");

    let checksum = action_checksum(&empty_file).await.expect("compute checksum");

    assert_eq!(checksum.len(), 64);
    assert_eq!(checksum, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[tokio::test]
async fn test_checksum_large_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let large_file = root.join("large.bin");
    let large_content = vec![0u8; 1_048_576];
    tokio::fs::write(&large_file, large_content)
        .await
        .expect("write large file");

    let checksum = action_checksum(&large_file).await.expect("compute checksum");

    assert_eq!(checksum.len(), 64);
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_backup_metadata_checksum_matches_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    let test_data = b"test backup data";
    tokio::fs::write(root.join(".scp").join("state.db"), test_data)
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut found_match = false;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            let db_checksum = action_checksum(&path).await.expect("compute db checksum");

            let metadata_path = path.with_extension("json");
            if metadata_path.exists() {
                let json = tokio::fs::read_to_string(&metadata_path)
                    .await
                    .expect("read metadata");
                let meta: BackupMetadata =
                    serde_json::from_str(&json).expect("parse metadata");

                assert_eq!(
                    db_checksum, meta.checksum,
                    "backup checksum should match metadata checksum"
                );
                found_match = true;
            }
        }
    }

    assert!(found_match, "should have found backup and metadata");
}

#[tokio::test]
async fn test_verify_backup_checksum_success() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    let original_data = b"original data";
    tokio::fs::write(root.join(".scp").join("state.db"), original_data)
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await
    .expect("restore with valid checksum");
}

#[tokio::test]
async fn test_verify_backup_checksum_mismatch() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    let original_data = b"original data";
    tokio::fs::write(root.join(".scp").join("state.db"), original_data)
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            let mut existing = tokio::fs::read(&path).await.expect("read backup");
            existing.extend_from_slice(b"corrupted");
            tokio::fs::write(&path, existing).await.expect("corrupt backup");
            break;
        }
    }

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail with corrupted backup");
}

#[tokio::test]
async fn test_corrupt_backup_file_detection() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"original")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            tokio::fs::write(&path, b"completely different content")
                .await
                .expect("corrupt backup");
            break;
        }
    }

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Corrupted backup should fail restore");
}

#[tokio::test]
async fn test_restore_corrupted_metadata() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            tokio::fs::write(&path, b"not valid json {{{").await.expect("corrupt metadata");
            break;
        }
    }

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail with corrupted metadata");
}

#[tokio::test]
async fn test_custom_backup_directory() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let config = BackupConfig {
        backup_dir: root.join("custom_backups"),
        retention_count: 5,
    };

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    assert!(
        root.join("custom_backups").join("state.db").exists(),
        "backup should be in custom directory"
    );
    assert!(
        !root.join("backups").join("state.db").exists(),
        "backup should not be in default directory"
    );
}

#[tokio::test]
async fn test_custom_backup_directory_nested() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let config = BackupConfig {
        backup_dir: root.join("deeply").join("nested").join("backups"),
        retention_count: 3,
    };

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    assert!(
        root
            .join("deeply")
            .join("nested")
            .join("backups")
            .join("state.db")
            .exists(),
        "backup should be in nested directory"
    );
}

#[tokio::test]
async fn test_backup_dir_auto_created() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    let config = BackupConfig {
        backup_dir: root.join("new_backup_dir"),
        retention_count: 2,
    };

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    assert!(!config.backup_dir.exists());

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    assert!(config.backup_dir.exists());
}

#[tokio::test]
async fn test_restore_missing_metadata_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let backup_dir = root.join("backups").join("state.db");
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            tokio::fs::remove_file(&path).await.expect("delete metadata");
            break;
        }
    }

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await;

    assert!(result.is_err(), "Should fail without metadata file");
}

#[tokio::test]
async fn test_restore_backup_not_found_for_timestamp() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: Some("20000101-000000".to_string()),
        },
        root,
        &config,
    )
    .await;

    assert!(
        result.is_err(),
        "Should fail when timestamp doesn't match any backup"
    );
}

#[tokio::test]
async fn test_invalid_timestamp_format() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    let result = execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: Some("invalid-timestamp".to_string()),
        },
        root,
        &config,
    )
    .await;

    assert!(
        result.is_err(),
        "Should fail with invalid timestamp format"
    );
}

#[tokio::test]
async fn test_backup_empty_database_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"")
        .await
        .expect("write empty state.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup from empty file");

    let backup_dir = root.join("backups").join("state.db");
    let mut found_backup = false;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            found_backup = true;
            let metadata = path.metadata().expect("metadata");
            assert_eq!(metadata.len(), 0, "empty file backup should have size 0");
        }
    }
    assert!(found_backup, "should have backup of empty file");
}

#[tokio::test]
async fn test_backup_with_special_characters_in_content() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");

    let special_data: Vec<u8> = (0..=255).collect();
    tokio::fs::write(root.join(".scp").join("state.db"), special_data.clone())
        .await
        .expect("write special data");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await
    .expect("restore");

    let restored = tokio::fs::read(root.join(".scp").join("state.db"))
        .await
        .expect("read restored");
    assert_eq!(restored, special_data);
}

#[tokio::test]
async fn test_backup_very_large_file() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");

    let large_data = vec![b'x'; 10_485_760];
    tokio::fs::write(root.join(".scp").join("state.db"), large_data.clone())
        .await
        .expect("write large file");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup of large file");

    let backup_dir = root.join("backups").join("state.db");
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            let metadata = path.metadata().expect("metadata");
            assert_eq!(metadata.len(), 10_485_760);
            break;
        }
    }
}

#[tokio::test]
async fn test_restore_preserves_permissions() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config(root);
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create backup");

    tokio::fs::write(root.join(".scp").join("state.db"), b"modified")
        .await
        .expect("modify");

    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        },
        root,
        &config,
    )
    .await
    .expect("restore");

    let restored = tokio::fs::read(root.join(".scp").join("state.db"))
        .await
        .expect("read");
    assert_eq!(restored, b"data");
}

#[tokio::test]
async fn test_full_backup_restore_workflow() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"initial state")
        .await
        .expect("write state.db");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"initial beads")
        .await
        .expect("write beads.db");

    let config = test_config(root);

    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create initial backup");

    tokio::fs::write(root.join(".scp").join("state.db"), b"modified state")
        .await
        .expect("modify state");
    tokio::fs::write(root.join(".scp").join("beads.db"), b"modified beads")
        .await
        .expect("modify beads");

    // Sleep 2 seconds to ensure different timestamp
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    execute_backup_command(&BackupCommand::Create, root, &config)
        .await
        .expect("create second backup");

    execute_backup_command(&BackupCommand::List, root, &config)
        .await
        .expect("list backups");

    tokio::fs::write(root.join(".scp").join("state.db"), b"temp state")
        .await
        .expect("modify state");

    let backup_dir = root.join("backups").join("state.db");
    let mut oldest_meta = None;
    let mut oldest_timestamp = None;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let json = tokio::fs::read_to_string(&path)
                .await
                .expect("read metadata");
            let meta: BackupMetadata = serde_json::from_str(&json).expect("parse metadata");
            // Find the oldest backup by comparing timestamps
            if oldest_timestamp.is_none() || meta.created_at < oldest_timestamp.unwrap() {
                oldest_timestamp = Some(meta.created_at);
                oldest_meta = Some(meta);
            }
        }
    }

    let Some(meta) = oldest_meta else {
        panic!("should have found backup metadata");
    };

    let timestamp = meta.created_at.format("%Y%m%d-%H%M%S").to_string();
    execute_backup_command(
        &BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: Some(timestamp.clone()),
        },
        root,
        &config,
    )
    .await
    .expect("restore first backup");

    let restored = tokio::fs::read(root.join(".scp").join("state.db"))
        .await
        .expect("read restored state");
    assert_eq!(restored, b"initial state");

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("check status");

    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");
}

#[tokio::test]
async fn test_backup_with_retention_policy_compliance() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let root = temp_dir.path();

    tokio::fs::create_dir_all(root.join(".scp")).await.expect("create .scp dir");
    tokio::fs::write(root.join(".scp").join("state.db"), b"data")
        .await
        .expect("write state.db");

    let config = test_config_with_retention(root, 3);

    for i in 0..6 {
        execute_backup_command(&BackupCommand::Create, root, &config)
            .await
            .expect("create backup");
        // Sleep 2 seconds to ensure unique timestamps
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        tokio::fs::write(root.join(".scp").join("state.db"), format!("data {}", i).as_bytes())
            .await
            .expect("update data");
    }

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("status before retention");

    execute_backup_command(&BackupCommand::Retention, root, &config)
        .await
        .expect("apply retention");

    execute_backup_command(&BackupCommand::Status, root, &config)
        .await
        .expect("status after retention");

    let backup_dir = root.join("backups").join("state.db");
    let mut db_count = 0;
    let mut entries = tokio::fs::read_dir(&backup_dir).await.expect("read dir");
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            db_count += 1;
        }
    }

    assert_eq!(db_count, 3, "should have exactly 3 backups after retention");
}
