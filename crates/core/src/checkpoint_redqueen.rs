//! RED QUEEN: Adversarial tests for checkpoint/backup/recover.
//!
//! These tests attack the system with hostile inputs and edge cases:
//! - Corrupt checkpoint recovery
//! - Recovery to wrong version
//! - Backup with no disk space (simulated)
//! - Concurrent checkpoint creation
//! - Checkpoint during active write
//! - Recovery race conditions

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use crate::checkpoint::*;
    use crate::error::Error;
    use crate::recovery::*;
    use crate::workspace_integrity::backup::BackupManager;
    use crate::workspace_integrity::issue::IntegrityIssue;
    use crate::workspace_integrity::repair::RepairExecutor;
    use crate::workspace_integrity::types::{CorruptionType, RepairStrategy};
    use crate::workspace_integrity::validation::IntegrityValidator;
    use crate::workspace_integrity::validation_result::ValidationResult;
    use crate::Result;
    use tempfile::TempDir;

    async fn test_pool() -> Result<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::database(format!("Failed to connect to test database: {e}")))?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        auto_cp.ensure_table().await?;
        Ok(pool)
    }

    fn create_test_root() -> Result<TempDir> {
        TempDir::new()
            .map_err(|e| Error::io_error(format!("Failed to create temp dir: {e}")))
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 1. CORRUPT CHECKPOINT RECOVERY
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: Find pending restores when checkpoint state is corrupt
    /// (manually injected garbage state in DB).
    #[tokio::test]
    async fn corrupt_checkpoint_state_not_findable() -> Result<()> {
        let pool = test_pool().await?;

        // Inject a checkpoint with a garbage state that doesn't match any valid enum
        sqlx::query("INSERT INTO checkpoints (id, created_at, state) VALUES (?, ?, ?)")
            .bind("auto-corrupt-001")
            .bind("2026-01-01T00:00:00Z")
            .bind("CORRUPTED_GARBAGE")
            .execute(&pool)
            .await
            .map_err(|e| Error::database(format!("Insert failed: {e}")))?;

        // find_pending_restores only looks for 'pending' and 'needs_restore',
        // so a garbage-state checkpoint should NOT appear in results.
        let pending = find_pending_restores(&pool).await?;
        assert!(
            !pending.contains(&"auto-corrupt-001".to_string()),
            "Corrupt-state checkpoint must not appear as pending restore"
        );
        Ok(())
    }

    /// Adversarial: Commit a checkpoint consumes the guard (rollback impossible after).
    #[tokio::test]
    async fn commit_consumes_guard_prevents_rollback() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        let guard = auto_cp
            .guard_if_risky(OperationRisk::Risky)
            .await?
            .expect("guard should exist");

        let id = guard.id().to_string();

        // commit consumes the guard, so rollback can't be called afterward
        guard.commit().await?;

        // Verify committed state in DB
        let row: Option<(String,)> =
            sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
        assert_eq!(row.map(|(s,)| s), Some("committed".to_string()));
        Ok(())
    }

    /// Adversarial: Rollback a checkpoint, then commit overwrites to committed.
    #[tokio::test]
    async fn rollback_then_commit_overwrites_state() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        let guard = auto_cp
            .guard_if_risky(OperationRisk::Risky)
            .await?
            .expect("guard should exist");

        let id = guard.id().to_string();

        // Rollback first (takes &self, guard still alive)
        guard.rollback().await?;

        // Verify needs_restore state
        let row: Option<(String,)> =
            sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
        assert_eq!(
            row.map(|(s,)| s),
            Some("needs_restore".to_string())
        );

        // Now commit (consumes guard)
        guard.commit().await?;

        // Verify committed state overwrote needs_restore
        let row: Option<(String,)> =
            sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
        assert_eq!(row.map(|(s,)| s), Some("committed".to_string()));
        Ok(())
    }

    /// Adversarial: Duplicate checkpoint ID should fail (PRIMARY KEY constraint).
    #[tokio::test]
    async fn duplicate_checkpoint_id_is_rejected() -> Result<()> {
        let pool = test_pool().await?;

        // Manually insert a checkpoint with known ID
        sqlx::query("INSERT INTO checkpoints (id, created_at, state) VALUES (?, ?, 'pending')")
            .bind("auto-dup-test")
            .bind("2026-01-01T00:00:00Z")
            .execute(&pool)
            .await
            .map_err(|e| Error::database(format!("Insert failed: {e}")))?;

        // Try to insert a duplicate manually - table enforces uniqueness
        let result = sqlx::query(
            "INSERT INTO checkpoints (id, created_at, state) VALUES (?, ?, 'pending')",
        )
        .bind("auto-dup-test")
        .bind("2026-01-01T00:00:00Z")
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Duplicate checkpoint ID must be rejected");
        Ok(())
    }

    /// Adversarial: Checkpoint table doesn't exist - should fail gracefully.
    #[tokio::test]
    async fn checkpoint_without_table_fails_gracefully() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::database(format!("Connect failed: {e}")))?;

        // Do NOT call ensure_table - table doesn't exist
        let auto_cp = AutoCheckpoint::new(pool);
        let result = auto_cp.guard_if_risky(OperationRisk::Risky).await;

        assert!(
            result.is_err(),
            "Creating checkpoint without table must fail"
        );
        Ok(())
    }

    /// Adversarial: find_pending_restores with no table should fail, not panic.
    #[tokio::test]
    async fn find_pending_without_table_fails_not_panics() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::database(format!("Connect failed: {e}")))?;

        let result = find_pending_restores(&pool).await;
        assert!(result.is_err(), "Must return error, not panic");
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 2. RECOVERY TO WRONG VERSION / INVALID STATE
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: Recover with invalid policy string.
    #[test]
    fn recovery_policy_rejects_garbage() {
        let junk_inputs = [
            "",
            "unknown",
            "repairx",
            "panic!",
            "warn repair",
            "\x00",
            "warn\x00",
            " warn",
            "warn ",
        ];
        for input in junk_inputs {
            let result = input.parse::<RecoveryPolicy>();
            assert!(
                result.is_err(),
                "Input '{input}': expected error, got ok"
            );
        }
    }

    /// Adversarial: RecoveryPolicy accepts case-insensitive valid values.
    #[test]
    fn recovery_policy_case_insensitive_accept() {
        for input in ["warn", "WARN", "Warn", "repair", "REPAIR", "panic", "PANIC"] {
            let result = input.parse::<RecoveryPolicy>();
            assert!(result.is_ok(), "Input '{input}' should be accepted");
        }
    }

    /// Adversarial: RecoveryConfig with all flags disabled - still valid.
    #[test]
    fn recovery_config_all_disabled_is_valid() {
        use crate::config::types::ValidatedBool;
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Panic,
            log_recovered: ValidatedBool::new(false),
            auto_recover_corrupted_wal: ValidatedBool::new(false),
            delete_corrupted_database: ValidatedBool::new(false),
        };
        assert!(!*config.log_recovered);
        assert!(!*config.auto_recover_corrupted_wal);
        assert!(!*config.delete_corrupted_database);
    }

    /// Adversarial: RecoveryConfig with delete_corrupted_database = true (dangerous but valid).
    #[test]
    fn recovery_config_delete_corrupted_is_dangerous_but_valid() {
        use crate::config::types::ValidatedBool;
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Repair,
            log_recovered: ValidatedBool::new(true),
            auto_recover_corrupted_wal: ValidatedBool::new(true),
            delete_corrupted_database: ValidatedBool::new(true),
        };
        assert!(*config.delete_corrupted_database);
    }

    /// Adversarial: check_database_integrity with various near-miss magic bytes.
    /// The check only compares header[..15], so bytes after the first 15 don't matter.
    #[tokio::test]
    async fn database_integrity_detects_near_miss_magic() {
        use tempfile::NamedTempFile;

        // These all have incorrect first 15 bytes, so should fail
        let near_misses: Vec<&[u8]> = vec![
            b"Sqlite format 3\0",     // lowercase 'l'
            b"SQLITE FORMAT 3\0",     // all caps
            b"sqlite format 3\0",     // all lowercase
            b"SQLite  format 3\0",    // double space
            b"SQLite format  3\0",    // double space before 3
            b"SQLite format 4\0",     // wrong version number
            b"\x00SQLite format 3\0", // null prefix
            b"QLite format 3\0\0",    // missing first char
        ];

        for bytes in &near_misses {
            let file = NamedTempFile::new().unwrap();
            std::fs::write(file.path(), bytes).unwrap();
            let result = check_database_integrity(file.path()).await.unwrap();
            assert!(
                !result,
                "Near-miss magic bytes {:?} should fail integrity check",
                std::str::from_utf8(bytes).unwrap_or("<binary>")
            );
        }
    }

    /// Adversarial: The integrity check only compares first 15 bytes.
    /// A file with correct first 15 bytes but garbage 16th byte passes.
    /// This documents the limitation of the header check.
    #[tokio::test]
    async fn database_integrity_wrong_16th_byte_passes_due_to_15byte_check() {
        use tempfile::NamedTempFile;

        // Correct first 15 bytes + garbage 16th byte
        // The check is `&header[..15] == b"SQLite format 3"` so this PASSES
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"SQLite format 3\xFF").unwrap();
        let result = check_database_integrity(file.path()).await.unwrap();
        assert!(
            result,
            "Check only verifies first 15 bytes - 16th byte is ignored. This documents the limitation."
        );
    }

    /// Adversarial: check_database_integrity with empty file.
    #[tokio::test]
    async fn database_integrity_empty_file_fails() {
        use tempfile::NamedTempFile;
        let file = NamedTempFile::new().unwrap();
        let result = check_database_integrity(file.path()).await.unwrap();
        assert!(!result, "Empty file should fail integrity check");
    }

    /// Adversarial: check_database_integrity with exactly 15 bytes of correct magic
    /// (header check reads 16 bytes via read_exact).
    #[tokio::test]
    async fn database_integrity_exactly_15_bytes_correct_magic() {
        use tempfile::NamedTempFile;
        let file = NamedTempFile::new().unwrap();
        // Exactly the magic without null - read_exact needs 16 bytes, so this fails
        std::fs::write(file.path(), b"SQLite format 3").unwrap();
        let result = check_database_integrity(file.path()).await.unwrap();
        assert!(
            !result,
            "15 bytes is too short for 16-byte read_exact, should fail"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 3. BACKUP WITH NO DISK SPACE (simulated via permission errors)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: BackupManager with readonly root - create_backup should fail.
    #[tokio::test]
    async fn backup_to_readonly_directory_fails() -> Result<()> {
        let root = create_test_root()?;

        // Create the backup directory first, then make it readonly
        let backup_dir = root.path().join(".hardline").join("backups");
        tokio::fs::create_dir_all(&backup_dir).await?;

        // Make the root readonly (no write permission)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o444);
            std::fs::set_permissions(root.path(), perms)?;
        }

        let manager = BackupManager::new(root.path());
        let result = manager.create_backup("ws", "test").await;

        // On Unix, writing to a readonly dir should fail
        #[cfg(unix)]
        {
            assert!(result.is_err(), "Backup to readonly directory must fail");
        }

        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            let _ = std::fs::set_permissions(root.path(), perms);
        }

        Ok(())
    }

    /// Adversarial: Backup with special characters in workspace name.
    #[tokio::test]
    async fn backup_with_special_chars_in_name() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        let special_names = [
            "ws-with-dashes",
            "ws_with_underscores",
            "ws.with.dots",
            "ws'quote",
            "ws;semicolon",
            "ws space",
            "../escape_attempt",
        ];

        for name in special_names {
            // Should either succeed (sanitized) or return error (rejected)
            let result = manager.create_backup(name, "test").await;
            // We accept both outcomes - the key is NO PANIC
            if let Ok(meta) = result {
                assert_eq!(meta.workspace, name);
            }
        }
        Ok(())
    }

    /// Adversarial: Restore with non-existent backup ID.
    /// Documents mock behavior so when it becomes real, this test will fail
    /// and force proper error handling.
    #[tokio::test]
    async fn restore_nonexistent_backup_succeeds_due_to_mock() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());
        let ws_path = root.path().join("ws");

        let result = manager.restore_backup("nonexistent-backup-id", "ws", &ws_path)?;
        // Mock always returns success - this is a known limitation
        assert!(result.success);
        Ok(())
    }

    /// Adversarial: list_backups returns empty for any workspace (mock limitation).
    #[tokio::test]
    async fn list_backups_always_empty_is_documented() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        manager.create_backup("ws", "test").await?;

        // list_backups returns empty because it's a mock
        let backups = manager.list_backups("ws")?;
        assert!(
            backups.is_empty(),
            "list_backups is mock - always returns empty. Documenting for future fix."
        );
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 4. CONCURRENT CHECKPOINT CREATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: Create multiple checkpoints with small delays.
    /// All should succeed with unique IDs (timestamp-based).
    #[tokio::test]
    async fn sequential_checkpoint_creation_all_succeed() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());

        let mut ids = std::collections::HashSet::new();

        for _ in 0..5 {
            // Small delay to ensure unique timestamp millis
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            let guard = auto_cp
                .guard_if_risky(OperationRisk::Risky)
                .await?
                .expect("guard");
            assert!(
                ids.insert(guard.id().to_string()),
                "Checkpoint IDs must be unique"
            );
            drop(guard);
        }

        // All 5 checkpoints should be in the DB as pending
        let pending = find_pending_restores(&pool).await?;
        assert!(
            pending.len() >= 5,
            "All sequential checkpoints should be tracked"
        );
        Ok(())
    }

    /// Adversarial: Multiple guards dropped simultaneously all become pending.
    #[tokio::test]
    async fn multiple_drops_all_become_pending() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());

        // Create guards with delays to avoid timestamp collision
        let mut guards = vec![];
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            let guard = auto_cp
                .guard_if_risky(OperationRisk::Risky)
                .await?
                .expect("guard");
            guards.push(guard);
        }

        let ids: Vec<String> = guards.iter().map(|g| g.id().to_string()).collect();

        // Drop all guards simultaneously
        drop(guards);

        // All should be pending
        let pending = find_pending_restores(&pool).await?;
        for id in &ids {
            assert!(
                pending.contains(id),
                "Checkpoint {id} should be pending after drop"
            );
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 5. CHECKPOINT DURING ACTIVE WRITE (simulated via interleaved operations)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: Ensure table is idempotent (call it multiple times).
    #[tokio::test]
    async fn ensure_table_is_idempotent() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::database(format!("Connect failed: {e}")))?;
        let auto_cp = AutoCheckpoint::new(pool.clone());

        // Call ensure_table many times
        for _ in 0..10 {
            auto_cp.ensure_table().await?;
        }

        // Should still work normally
        let guard = auto_cp.guard_if_risky(OperationRisk::Risky).await?;
        assert!(guard.is_some());
        Ok(())
    }

    /// Adversarial: Multiple AutoCheckpoint instances share the same pool.
    #[tokio::test]
    async fn multiple_instances_same_pool() -> Result<()> {
        let pool = test_pool().await?;

        let cp1 = AutoCheckpoint::new(pool.clone());
        let cp2 = AutoCheckpoint::new(pool.clone());

        let g1 = cp1.guard_if_risky(OperationRisk::Risky).await?.expect("g1");
        // Small delay for unique timestamp
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        let g2 = cp2.guard_if_risky(OperationRisk::Risky).await?.expect("g2");

        // Different IDs
        assert_ne!(g1.id(), g2.id());

        // Commit one, drop the other
        let id1 = g1.id().to_string();
        let id2 = g2.id().to_string();
        g1.commit().await?;
        drop(g2); // g2 should be pending

        let row1: Option<(String,)> =
            sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                .bind(&id1)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
        assert_eq!(row1.map(|(s,)| s), Some("committed".to_string()));

        let pending = find_pending_restores(&pool).await?;
        assert!(pending.contains(&id2));
        Ok(())
    }

    /// Adversarial: Guard dropped during async operation (simulates crash mid-write).
    #[tokio::test]
    async fn guard_dropped_mid_operation_is_tracked() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());

        let checkpoint_id = {
            let guard = auto_cp
                .guard_if_risky(OperationRisk::Risky)
                .await?
                .expect("guard");
            let id = guard.id().to_string();
            // Simulate: operation crashes mid-write by dropping guard
            drop(guard);
            id
        };

        // The checkpoint should still be findable as pending
        let pending = find_pending_restores(&pool).await?;
        assert!(
            pending.contains(&checkpoint_id),
            "Dropped guard must leave checkpoint in pending state for crash recovery"
        );
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 6. RECOVERY RACE CONDITIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Adversarial: Repair an already-deleted workspace (race: workspace deleted
    /// between validation and repair).
    #[tokio::test]
    async fn repair_after_workspace_deleted_between_validate_and_repair() -> Result<()> {
        let root = create_test_root()?;
        let ws_path = root.path().join("race-ws");

        // Create a valid workspace with a stale lock
        tokio::fs::create_dir_all(ws_path.join(".git")).await?;
        tokio::fs::write(ws_path.join(".git").join("index.lock"), "lock").await?;

        // Create validation result (workspace exists at this point)
        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Stale lock")
            .with_path(ws_path.join(".git").join("index.lock"))];
        let validation = ValidationResult::invalid("race-ws", &ws_path, issues);

        // RACE: Delete workspace between validation and repair
        tokio::fs::remove_dir_all(&ws_path).await?;

        let executor = RepairExecutor::new();
        let result = executor.repair(&validation).await?;

        // Repair should handle missing workspace gracefully
        assert!(
            !result.success,
            "Repair of deleted workspace must report failure"
        );
        Ok(())
    }

    /// Adversarial: Lock file removed between validation and repair (another process
    /// already cleaned it). Repair should be idempotent.
    #[tokio::test]
    async fn repair_lock_removed_by_another_process() -> Result<()> {
        let root = create_test_root()?;
        let ws_path = root.path().join("ws");
        let lock_path = ws_path.join(".git").join("index.lock");

        // Create workspace with lock
        tokio::fs::create_dir_all(ws_path.join(".git")).await?;
        tokio::fs::write(&lock_path, "lock").await?;

        let issues =
            vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Stale").with_path(&lock_path)];
        let validation = ValidationResult::invalid("ws", &ws_path, issues);

        // Another process removes the lock before our repair
        tokio::fs::remove_file(&lock_path).await?;

        let executor = RepairExecutor::new();
        let result = executor.repair(&validation).await?;

        // Repair should succeed (idempotent: removing nonexistent lock is OK)
        assert!(result.success);
        Ok(())
    }

    /// Adversarial: Repair on a valid workspace (no issues) returns success with no-op.
    #[tokio::test]
    async fn repair_valid_workspace_is_noop() -> Result<()> {
        let root = create_test_root()?;
        let ws_path = root.path().join("ws");
        tokio::fs::create_dir_all(&ws_path).await?;

        let validation = ValidationResult::valid("ws", &ws_path);
        let executor = RepairExecutor::new();
        let result = executor.repair(&validation).await?;

        assert!(result.success);
        assert_eq!(result.action, RepairStrategy::NoRepair);
        Ok(())
    }

    /// Adversarial: Repair with only NoRepair issues returns failure.
    #[tokio::test]
    async fn repair_no_repair_issues_returns_failure() -> Result<()> {
        let root = create_test_root()?;
        let ws_path = root.path().join("ws");
        tokio::fs::create_dir_all(&ws_path).await?;

        let issues = vec![IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            "Access denied",
        )];
        let validation = ValidationResult::invalid("ws", &ws_path, issues);
        let executor = RepairExecutor::new();
        let result = executor.repair(&validation).await?;

        assert!(!result.success);
        assert_eq!(result.action, RepairStrategy::NoRepair);
        Ok(())
    }

    /// Adversarial: Repair with backup enabled actually records backup ID.
    #[tokio::test]
    async fn repair_with_backup_records_backup_id() -> Result<()> {
        let root = create_test_root()?;
        let ws_path = root.path().join("ws");
        tokio::fs::create_dir_all(ws_path.join(".git")).await?;
        tokio::fs::write(ws_path.join(".git").join("index.lock"), "lock").await?;

        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Lock")];
        let validation = ValidationResult::invalid("ws", &ws_path, issues);

        let manager = BackupManager::new(root.path());
        let executor = RepairExecutor::new().with_backup_manager(manager);
        let result = executor.repair(&validation).await?;

        assert!(result.success);
        assert!(
            result.backup_id.is_some(),
            "Repair with backup must record backup ID"
        );
        Ok(())
    }

    /// Adversarial: RepairExecutor builder pattern ordering (backup then no-backup).
    #[tokio::test]
    async fn repair_executor_builder_overrides() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        // With backup, then without
        let executor = RepairExecutor::new()
            .with_backup_manager(manager)
            .without_backup();
        assert!(!executor.creates_backups());
        Ok(())
    }

    /// Adversarial: validate_all with mix of valid and invalid workspaces
    /// preserves order and reports correctly.
    #[tokio::test]
    async fn validate_all_mixed_validity_preserves_order() -> Result<()> {
        let root = create_test_root()?;

        // ws-valid: valid workspace
        let valid_ws = root.path().join("ws-valid");
        tokio::fs::create_dir_all(valid_ws.join(".git").join("objects")).await?;
        tokio::fs::create_dir_all(valid_ws.join(".git").join("refs")).await?;
        tokio::fs::write(valid_ws.join(".git").join("HEAD"), "ref: refs/heads/main\n").await?;

        // ws-no-git: missing .git
        let no_git_ws = root.path().join("ws-no-git");
        tokio::fs::create_dir(&no_git_ws).await?;

        // ws-missing: doesn't exist at all

        let validator = IntegrityValidator::new(root.path());
        let results = validator
            .validate_all(&[
                "ws-valid".to_string(),
                "ws-no-git".to_string(),
                "ws-missing".to_string(),
            ])
            .await?;

        assert_eq!(results.len(), 3);
        assert!(results[0].is_valid, "ws-valid should be valid");
        assert!(!results[1].is_valid, "ws-no-git should be invalid");
        assert!(!results[2].is_valid, "ws-missing should be invalid");
        assert_eq!(results[0].workspace, "ws-valid");
        assert_eq!(results[1].workspace, "ws-no-git");
        assert_eq!(results[2].workspace, "ws-missing");
        Ok(())
    }

    /// Adversarial: IntegrityValidator with path traversal workspace name.
    #[tokio::test]
    async fn validator_path_traversal_workspace_name() -> Result<()> {
        let root = create_test_root()?;
        let validator = IntegrityValidator::new(root.path());

        let traversal_names = [
            "../etc/passwd",
            "../../root",
            "../../../tmp/evil",
        ];

        for name in traversal_names {
            let result = validator.validate(name).await?;
            assert!(
                !result.is_valid,
                "Path traversal '{name}' must be detected as invalid"
            );
        }
        Ok(())
    }

    /// Adversarial: validate_all with empty list returns empty results.
    #[tokio::test]
    async fn validate_all_empty_list() -> Result<()> {
        let root = create_test_root()?;
        let validator = IntegrityValidator::new(root.path());
        let results = validator.validate_all(&[]).await?;
        assert!(results.is_empty());
        Ok(())
    }

    /// Adversarial: classify_command with injection-like strings.
    #[test]
    fn classify_command_adversarial_inputs() {
        let adversarial = [
            "",
            " ",
            "batch ",
            " batch",
            "BATCH",
            "batch\0",
            "batch;rm -rf",
            "../../../batch",
            "batch\nextra",
        ];

        for input in adversarial {
            let risk = classify_command(input);
            let expected = if input == "batch" {
                OperationRisk::Risky
            } else {
                OperationRisk::Safe
            };
            assert_eq!(
                risk, expected,
                "classify_command({input:?}) = {risk:?}, expected {expected:?}"
            );
        }
    }

    /// Adversarial: Serde roundtrip with extra fields (forward compatibility).
    #[test]
    fn recovery_config_serde_ignores_unknown_fields() {
        let json = r#"{
            "policy": "warn",
            "log_recovered": true,
            "auto_recover_corrupted_wal": true,
            "delete_corrupted_database": false,
            "unknown_field": "should_be_ignored"
        }"#;
        let result = serde_json::from_str::<RecoveryConfig>(json);
        assert!(
            result.is_ok(),
            "serde ignores unknown fields by default"
        );
    }

    /// Adversarial: Serde with missing fields.
    #[test]
    fn recovery_config_serde_missing_fields_no_panic() {
        let json = r#"{"policy": "warn"}"#;
        let result = serde_json::from_str::<RecoveryConfig>(json);
        // Either succeeds (with defaults) or fails (missing fields) - just no panic
        if let Ok(config) = result {
            assert_eq!(config.policy, RecoveryPolicy::Warn);
        }
    }

    /// Adversarial: Serde with invalid policy variant.
    #[test]
    fn recovery_policy_serde_invalid_variant() {
        let json = "\"invalid\"";
        let result = serde_json::from_str::<RecoveryPolicy>(json);
        assert!(result.is_err(), "Invalid policy variant must be rejected");
    }

    /// Adversarial: CheckpointGuard is_committed reads are safe.
    #[tokio::test]
    async fn guard_is_committed_concurrent_reads() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool);
        let guard = auto_cp
            .guard_if_risky(OperationRisk::Risky)
            .await?
            .expect("guard");

        // is_committed should be false initially
        assert!(!guard.is_committed());

        // Multiple concurrent reads
        let _unused: Vec<bool> = (0..10).map(|_| guard.is_committed()).collect();
        assert!(!guard.is_committed());
        Ok(())
    }
}
