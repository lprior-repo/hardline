// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Test for repairing non-existent workspace
//!
//! This test ensures that:
//! 1. Attempting to repair a non-existent workspace returns a proper error
//! 2. No panic occurs when repairing non-existent workspace
//! 3. Error message is clear and actionable

use std::error::Error;

use isolate_core::workspace_integrity::{
    BackupManager, IntegrityValidator, RepairExecutor, ValidationResult,
};
use tempfile::TempDir;

#[tokio::test(flavor = "current_thread")]
async fn test_repair_nonexistent_workspace_returns_error() -> Result<(), Box<dyn Error>> {
    // Given: A temporary root directory
    let root = TempDir::new()?;
    let validator = IntegrityValidator::new(root.path());

    // When: Validating a non-existent workspace
    let result = validator.validate("nonexistent-workspace").await;

    // Then: Validation should succeed but show issues
    assert!(result.is_ok(), "Validation should not panic");
    let validation = match result {
        Ok(v) => v,
        Err(e) => return Err(format!("Validation should succeed but got error: {e}").into()),
    };
    assert!(
        !validation.is_valid,
        "Non-existent workspace should be invalid"
    );
    assert_eq!(
        validation.issues.len(),
        1,
        "Should have exactly one issue: missing directory"
    );
    assert_eq!(
        validation.issues[0].corruption_type.to_string(),
        "missing_directory",
        "Issue should be about missing directory"
    );

    // When: Attempting to repair the non-existent workspace
    let executor = RepairExecutor::new();
    let repair_result = executor.repair(&validation).await;

    // Then: Repair should return a proper error, NOT panic
    assert!(
        repair_result.is_ok(),
        "Repair should not panic on non-existent workspace"
    );

    let repair = match repair_result {
        Ok(r) => r,
        Err(e) => return Err(format!("Repair should succeed but got error: {e}").into()),
    };
    assert!(
        !repair.success,
        "Repair should fail for non-existent workspace"
    );

    // The error message should be clear and mention the missing directory
    assert!(
        repair.summary.contains("does not exist")
            || repair.summary.contains("Cannot repair missing workspace"),
        "Error message should clearly indicate the workspace is missing: {}",
        repair.summary
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_repair_nonexistent_workspace_with_backup_manager() -> Result<(), Box<dyn Error>> {
    // Given: A temporary root with backup manager
    let root = TempDir::new()?;
    let validator = IntegrityValidator::new(root.path());
    let backup_manager = BackupManager::new(root.path());

    // When: Validating a non-existent workspace
    let validation = match validator.validate("nonexistent-workspace").await {
        Ok(v) => v,
        Err(e) => return Err(format!("Validation should succeed but got error: {e}").into()),
    };

    // When: Attempting to repair with backup manager
    let executor = RepairExecutor::new().with_backup_manager(backup_manager);
    let repair_result = executor.repair(&validation).await;

    // Then: Should NOT panic
    assert!(
        repair_result.is_ok(),
        "Repair with backup manager should not panic on non-existent workspace"
    );

    let repair = match repair_result {
        Ok(r) => r,
        Err(e) => return Err(format!("Repair should succeed but got error: {e}").into()),
    };
    // Should fail gracefully without trying to create a backup
    assert!(
        !repair.success,
        "Repair should fail for non-existent workspace (no backup should be created)"
    );

    // The error message should be clear
    assert!(
        repair.summary.contains("does not exist")
            || repair.summary.contains("Cannot repair missing workspace"),
        "Error message should clearly indicate the workspace is missing: {}",
        repair.summary
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_forget_and_recreate_nonexistent_workspace() -> Result<(), Box<dyn Error>> {
    // This is a more direct test of the specific function that might panic
    // Given: A validation result for a non-existent workspace
    let root = TempDir::new()?;
    let workspace_path = root.path().join("nonexistent-workspace");

    let validation = ValidationResult::invalid(
        "nonexistent-workspace",
        &workspace_path,
        vec![isolate_core::workspace_integrity::IntegrityIssue::new(
            isolate_core::workspace_integrity::CorruptionType::MissingDirectory,
            "Workspace directory does not exist",
        )
        .with_path(&workspace_path)],
    );

    // When: Attempting repair on validation result for non-existent workspace
    let executor = RepairExecutor::new();
    let repair_result = executor.repair(&validation).await;

    // Then: Should return a proper result, not panic
    assert!(
        repair_result.is_ok(),
        "Repair should not panic for non-existent workspace"
    );

    let repair = match repair_result {
        Ok(r) => r,
        Err(e) => return Err(format!("Repair should succeed but got error: {e}").into()),
    };
    // Should fail gracefully
    assert!(
        !repair.success,
        "Repair should fail for non-existent workspace"
    );

    // The error message should clearly indicate the problem
    assert!(
        repair.summary.contains("does not exist")
            || repair.summary.contains("Cannot repair missing workspace"),
        "Summary should clearly describe the missing workspace: {}",
        repair.summary
    );

    Ok(())
}
