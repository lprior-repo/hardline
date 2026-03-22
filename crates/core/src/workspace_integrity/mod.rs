//! Workspace integrity validation and repair
//!
//! This module provides tools to detect and fix common JJ workspace
//! corruption issues, ensuring agents can operate safely.
//!
//! # Architecture
//!
//! - [`types`] - Enums (CorruptionType, RepairStrategy, Severity)
//! - [`issue`] - IntegrityIssue type for detected issues
//! - [`validation_result`] - ValidationResult type for validation outcomes
//! - [`repair_result`] - RepairResult, RollbackResult, BackupMetadata types
//! - [`validation`] - IntegrityValidator for detecting workspace issues
//! - [`repair`] - RepairExecutor for fixing issues
//! - [`backup`] - BackupManager for creating/restoring backups
//! - [`checks`] - Helper functions for validation checks (internal)
//!
//! # Example
//!
//! ```ignore
//! use scp_core::workspace_integrity::{IntegrityValidator, RepairExecutor};
//!
//! // Validate a workspace
//! let validator = IntegrityValidator::new("/path/to/workspaces");
//! let result = validator.validate("my-workspace").await?;
//!
//! // Repair if needed
//! if !result.is_valid {
//!     let executor = RepairExecutor::new();
//!     executor.repair(&result).await?;
//! }
//! ```

// Re-export all public types from submodules
pub mod backup;
pub mod checks;
pub mod issue;
pub mod repair;
pub mod repair_result;
pub mod tests;
pub mod types;
pub mod validation;
pub mod validation_result;

// Re-export types for convenience
pub use types::{CorruptionType, RepairStrategy, Severity};
pub use issue::IntegrityIssue;
pub use validation_result::ValidationResult;
pub use repair_result::{BackupMetadata, RepairResult, RollbackResult};

// Re-export main structs
pub use validation::IntegrityValidator;
pub use repair::RepairExecutor;
pub use backup::BackupManager;
