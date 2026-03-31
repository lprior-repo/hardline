//! Tests for workspace integrity module

#[cfg(test)]
mod tests {
    use crate::workspace_integrity::RepairResult;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::workspace_integrity::backup::BackupManager;
    use crate::workspace_integrity::checks::resolve_workspace_path;
    use crate::workspace_integrity::issue::IntegrityIssue;
    use crate::workspace_integrity::repair::RepairExecutor;
    use crate::workspace_integrity::types::{CorruptionType, RepairStrategy, Severity};
    use crate::workspace_integrity::validation::IntegrityValidator;
    use crate::workspace_integrity::validation_result::ValidationResult;
    use crate::Result;

    // Helper to create a temporary workspaces root for testing
    fn create_test_root() -> Result<TempDir> {
        TempDir::new()
            .map_err(|e| crate::Error::io_error(format!("Failed to create temp dir: {e}")))
    }

    #[tokio::test]
    async fn test_integrity_validator_new() {
        let validator = IntegrityValidator::new("/tmp/workspaces");
        assert_eq!(validator.workspaces_root, PathBuf::from("/tmp/workspaces"));
        assert_eq!(validator.timeout_ms, IntegrityValidator::DEFAULT_TIMEOUT_MS);
    }

    #[tokio::test]
    async fn test_integrity_validator_with_timeout() {
        let validator = IntegrityValidator::new("/tmp/workspaces").with_timeout(1000);
        assert_eq!(validator.timeout_ms, 1000);
    }

    #[test]
    fn test_resolve_workspace_path_keeps_absolute_path() {
        let root = PathBuf::from("/tmp/workspaces");
        let resolved = resolve_workspace_path(&root, "/var/tmp/ws-a");
        assert_eq!(resolved, PathBuf::from("/var/tmp/ws-a"));
    }

    #[test]
    fn test_resolve_workspace_path_keeps_relative_path_input() {
        let root = PathBuf::from("/tmp/workspaces");
        let resolved = resolve_workspace_path(&root, ".isolate/workspaces/ws-a");
        assert_eq!(resolved, PathBuf::from(".isolate/workspaces/ws-a"));
    }

    #[test]
    fn test_resolve_workspace_path_joins_relative_name() {
        let root = PathBuf::from("/tmp/workspaces");
        let resolved = resolve_workspace_path(&root, "my-workspace");
        assert_eq!(resolved, PathBuf::from("/tmp/workspaces/my-workspace"));
    }

    #[tokio::test]
    async fn test_integrity_validator_missing_directory() -> Result<()> {
        let root = create_test_root()?;
        let validator = IntegrityValidator::new(root.path());

        let result = validator.validate("nonexistent").await?;

        assert!(!result.is_valid);
        assert_eq!(result.workspace, "nonexistent");
        assert_eq!(result.issues.len(), 1);
        assert_eq!(
            result.issues[0].corruption_type,
            CorruptionType::MissingDirectory
        );
        assert_eq!(result.max_severity, Some(Severity::Critical));
        Ok(())
    }

    #[tokio::test]
    async fn test_integrity_validator_valid_workspace() -> Result<()> {
        let root = create_test_root()?;
        let workspace_path = root.path().join("valid-ws");
        tokio::fs::create_dir_all(workspace_path.join(".jj").join("repo")).await?;
        tokio::fs::create_dir_all(workspace_path.join(".jj").join("repo").join("op_store")).await?;
        tokio::fs::write(
            workspace_path
                .join(".jj")
                .join("repo")
                .join("op_store")
                .join("test"),
            "data",
        )
        .await?;

        let validator = IntegrityValidator::new(root.path());
        let result = validator.validate("valid-ws").await?;

        assert!(result.is_valid);
        assert_eq!(result.issues.len(), 0);
        assert_eq!(result.max_severity, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_integrity_validator_missing_jj_dir() -> Result<()> {
        let root = create_test_root()?;
        let workspace_path = root.path().join("no-jj");
        tokio::fs::create_dir(&workspace_path).await?;

        let validator = IntegrityValidator::new(root.path());
        let result = validator.validate("no-jj").await?;

        assert!(!result.is_valid);
        assert!(result
            .issues
            .iter()
            .any(|i| i.corruption_type == CorruptionType::MissingJjDir));
        Ok(())
    }

    #[tokio::test]
    async fn test_integrity_validator_validate_all() -> Result<()> {
        let root = create_test_root()?;
        tokio::fs::create_dir(root.path().join("ws1")).await?;
        tokio::fs::create_dir(root.path().join("ws2")).await?;

        let validator = IntegrityValidator::new(root.path());
        let results = validator
            .validate_all(&["ws1".to_string(), "ws2".to_string()])
            .await?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].workspace, "ws1");
        assert_eq!(results[1].workspace, "ws2");
        Ok(())
    }

    #[tokio::test]
    async fn test_integrity_issue_new() {
        let issue = IntegrityIssue::new(CorruptionType::StaleLocks, "Locked");
        assert_eq!(issue.corruption_type, CorruptionType::StaleLocks);
        assert_eq!(issue.severity, Severity::Warn);
        assert_eq!(issue.description, "Locked");
        assert_eq!(issue.recommended_strategy, RepairStrategy::ClearLocks);
    }

    #[test]
    fn test_integrity_issue_with_path() {
        let issue =
            IntegrityIssue::new(CorruptionType::StaleLocks, "Locked").with_path("/tmp/lock");
        assert_eq!(issue.affected_path, Some(PathBuf::from("/tmp/lock")));
    }

    #[test]
    fn test_integrity_issue_with_context() {
        let issue = IntegrityIssue::new(CorruptionType::StaleLocks, "Locked")
            .with_context("File held by process 1234");
        assert_eq!(issue.context, Some("File held by process 1234".to_string()));
    }

    #[test]
    fn test_integrity_issue_with_strategy() {
        let issue = IntegrityIssue::new(CorruptionType::StaleLocks, "Locked")
            .with_strategy(RepairStrategy::NoRepair);
        assert_eq!(issue.recommended_strategy, RepairStrategy::NoRepair);
    }

    #[tokio::test]
    async fn test_validation_result_valid() {
        let result = ValidationResult::valid("ws", "/tmp/ws");
        assert!(result.is_valid);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.path, PathBuf::from("/tmp/ws"));
        assert!(result.issues.is_empty());
        assert_eq!(result.max_severity, None);
    }

    #[tokio::test]
    async fn test_validation_result_invalid() {
        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Locked")];
        let result = ValidationResult::invalid("ws", "/tmp/ws", issues);
        assert!(!result.is_valid);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.max_severity, Some(Severity::Warn));
    }

    #[tokio::test]
    async fn test_validation_result_has_auto_repairable() {
        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Locked")];
        let result = ValidationResult::invalid("ws", "/tmp/ws", issues);
        assert!(result.has_auto_repairable_issues());
    }

    #[tokio::test]
    async fn test_validation_result_no_auto_repairable() {
        let issues = vec![IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            "Access denied",
        )];
        let result = ValidationResult::invalid("ws", "/tmp/ws", issues);
        assert!(!result.has_auto_repairable_issues());
    }

    #[tokio::test]
    async fn test_validation_result_most_severe_issue() {
        let issues = vec![
            IntegrityIssue::new(CorruptionType::StaleLocks, "Locked"),
            IntegrityIssue::new(CorruptionType::MissingDirectory, "Missing"),
        ];
        let result = ValidationResult::invalid("ws", "/tmp/ws", issues);
        let most_severe = result.most_severe_issue();
        assert!(most_severe.is_some());
        assert_eq!(
            most_severe.unwrap().corruption_type,
            CorruptionType::MissingDirectory
        );
    }

    #[tokio::test]
    async fn test_repair_result_success() {
        let result = RepairResult::success("ws", RepairStrategy::ClearLocks, "Fixed");
        assert!(result.success);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.action, RepairStrategy::ClearLocks);
        assert_eq!(result.summary, "Fixed");
        assert_eq!(result.backup_id, None);
    }

    #[tokio::test]
    async fn test_repair_result_failure() {
        let result = RepairResult::failure("ws", RepairStrategy::NoRepair, "Cannot fix");
        assert!(!result.success);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.action, RepairStrategy::NoRepair);
        assert_eq!(result.summary, "Cannot fix");
    }

    #[tokio::test]
    async fn test_repair_result_with_backup() {
        let result = RepairResult::success("ws", RepairStrategy::ClearLocks, "Fixed")
            .with_backup("backup-123");
        assert_eq!(result.backup_id, Some("backup-123".to_string()));
    }

    #[tokio::test]
    async fn test_repair_executor_new() {
        let executor = RepairExecutor::new();
        assert!(!executor.creates_backups());
    }

    #[tokio::test]
    async fn test_repair_executor_with_backup() {
        let root = create_test_root().unwrap();
        let manager = BackupManager::new(root.path());
        let executor = RepairExecutor::new().with_backup_manager(manager);
        assert!(executor.creates_backups());
    }

    #[tokio::test]
    async fn test_repair_executor_clear_stale_locks() -> Result<()> {
        let root = create_test_root()?;
        let ws = root.path().join("ws");
        tokio::fs::create_dir_all(ws.join(".jj").join("working_copy")).await?;
        let lock = ws.join(".jj").join("working_copy").join("lock");
        tokio::fs::write(&lock, "lock").await?;

        let executor = RepairExecutor::new();
        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLocks, "Lock").with_path(&lock)];
        let validation = ValidationResult::invalid("ws", &ws, issues);

        let result = executor.repair(&validation).await?;
        assert!(result.success);
        assert_eq!(result.action, RepairStrategy::ClearLocks);
        assert!(!tokio::fs::try_exists(&lock).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_repair_executor_already_valid() -> Result<()> {
        let root = create_test_root()?;
        let ws = root.path().join("ws");
        tokio::fs::create_dir_all(&ws).await?;

        let executor = RepairExecutor::new();
        let validation = ValidationResult::valid("ws", &ws);

        let result = executor.repair(&validation).await?;
        assert!(result.success);
        assert_eq!(result.action, RepairStrategy::NoRepair);
        assert_eq!(result.summary, "Workspace is already healthy");
        Ok(())
    }

    #[tokio::test]
    async fn test_backup_manager_create_and_list() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        let meta = manager.create_backup("ws", "Test").await?;
        assert_eq!(meta.workspace, "ws");
        assert_eq!(meta.reason, "Test");
        assert!(tokio::fs::try_exists(root.path().join(".isolate").join("backups")).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_backup_manager_list_backups() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        let backups = manager.list_backups("ws")?;
        assert!(backups.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_backup_manager_restore() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());
        let ws_path = root.path().join("ws");

        let result = manager.restore_backup("backup-123", "ws", &ws_path)?;
        assert!(result.success);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.summary, "Restored from backup backup-123");
        Ok(())
    }

    #[tokio::test]
    async fn test_corruption_type_display() {
        assert_eq!(
            format!("{}", CorruptionType::MissingDirectory),
            "missing_directory"
        );
        assert_eq!(
            format!("{}", CorruptionType::MissingJjDir),
            "missing_jj_dir"
        );
        assert_eq!(format!("{}", CorruptionType::StaleLocks), "stale_locks");
    }

    #[tokio::test]
    async fn test_repair_strategy_display() {
        assert_eq!(format!("{}", RepairStrategy::ClearLocks), "clear_locks");
        assert_eq!(
            format!("{}", RepairStrategy::ForgetAndRecreate),
            "forget_and_recreate"
        );
    }

    #[tokio::test]
    async fn test_repair_strategy_description() {
        assert_eq!(
            RepairStrategy::ClearLocks.description(),
            "Clear stale lock files"
        );
        assert_eq!(
            RepairStrategy::NoRepairPossible.description(),
            "No automated repair possible"
        );
    }

    #[test]
    fn test_corruption_type_from_str() {
        use std::str::FromStr;
        assert_eq!(
            CorruptionType::from_str("missing_directory"),
            Ok(CorruptionType::MissingDirectory)
        );
        assert_eq!(
            CorruptionType::from_str("stale_locks"),
            Ok(CorruptionType::StaleLocks)
        );
        assert!(CorruptionType::from_str("invalid").is_err());
    }

    #[test]
    fn test_repair_strategy_from_str() {
        use std::str::FromStr;
        assert_eq!(
            RepairStrategy::from_str("clear_locks"),
            Ok(RepairStrategy::ClearLocks)
        );
        assert_eq!(
            RepairStrategy::from_str("no_repair_possible"),
            Ok(RepairStrategy::NoRepairPossible)
        );
        assert!(RepairStrategy::from_str("invalid").is_err());
    }
}
