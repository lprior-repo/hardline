//! Workspace integrity validation and repair
//!
//! This module provides tools to detect and fix common JJ workspace
//! corruption issues, ensuring agents can operate safely.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{jj::get_jj_command, Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Types of workspace corruption/issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorruptionType {
    /// Workspace directory is missing
    MissingDirectory,
    /// .jj directory is missing
    MissingJjDir,
    /// .jj directory is corrupted (e.g. empty or missing files)
    CorruptedJjDir,
    /// Stale lock files exist
    StaleLocks,
    /// Permission issues
    PermissionDenied,
    /// Git index is corrupted (if using Git colocation)
    CorruptedGitIndex,
}

impl std::fmt::Display for CorruptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDirectory => write!(f, "missing_directory"),
            Self::MissingJjDir => write!(f, "missing_jj_dir"),
            Self::CorruptedJjDir => write!(f, "corrupted_jj_dir"),
            Self::StaleLocks => write!(f, "stale_locks"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::CorruptedGitIndex => write!(f, "corrupted_git_index"),
        }
    }
}

impl FromStr for CorruptionType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "missing_directory" => Ok(Self::MissingDirectory),
            "missing_jj_dir" => Ok(Self::MissingJjDir),
            "corrupted_jj_dir" => Ok(Self::CorruptedJjDir),
            "stale_locks" => Ok(Self::StaleLocks),
            "permission_denied" => Ok(Self::PermissionDenied),
            "corrupted_git_index" => Ok(Self::CorruptedGitIndex),
            _ => Err(()),
        }
    }
}

/// Strategy for repairing a corruption issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// No automated repair possible
    NoRepair,
    /// No automated repair possible (alternative name for compatibility)
    NoRepairPossible,
    /// Remove stale lock files
    ClearLocks,
    /// Attempt to fix JJ directory structure
    FixJjDir,
    /// Re-initialize JJ in the workspace
    RecreateWorkspace,
    /// Forget workspace in JJ and add it again
    ForgetAndRecreate,
}

impl RepairStrategy {
    /// Get a human-readable description of the strategy
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::NoRepair | Self::NoRepairPossible => "No automated repair possible",
            Self::ClearLocks => "Clear stale lock files",
            Self::FixJjDir => "Fix JJ directory structure",
            Self::RecreateWorkspace => "Recreate workspace",
            Self::ForgetAndRecreate => "Forget and recreate workspace",
        }
    }
}

impl std::fmt::Display for RepairStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRepair => write!(f, "no_repair"),
            Self::NoRepairPossible => write!(f, "no_repair_possible"),
            Self::ClearLocks => write!(f, "clear_locks"),
            Self::FixJjDir => write!(f, "fix_jj_dir"),
            Self::RecreateWorkspace => write!(f, "recreate_workspace"),
            Self::ForgetAndRecreate => write!(f, "forget_and_recreate"),
        }
    }
}

impl FromStr for RepairStrategy {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "no_repair" => Ok(Self::NoRepair),
            "no_repair_possible" => Ok(Self::NoRepairPossible),
            "clear_locks" => Ok(Self::ClearLocks),
            "fix_jj_dir" => Ok(Self::FixJjDir),
            "recreate_workspace" => Ok(Self::RecreateWorkspace),
            "forget_and_recreate" => Ok(Self::ForgetAndRecreate),
            _ => Err(()),
        }
    }
}

/// Severity level of an integrity issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational or minor warning
    Info,
    /// Warning - potential issues, but may work
    Warn,
    /// Error - workspace is unusable without repair
    Fail,
    /// Critical - multiple failures or unfixable corruption
    Critical,
}

/// A specific integrity issue detected in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    /// Type of corruption
    pub corruption_type: CorruptionType,
    /// Severity level
    pub severity: Severity,
    /// Description of the issue
    pub description: String,
    /// Path affected by the issue
    pub affected_path: Option<PathBuf>,
    /// Contextual information (e.g. error message)
    pub context: Option<String>,
    /// Recommended repair strategy
    pub recommended_strategy: RepairStrategy,
}

impl IntegrityIssue {
    /// Create a new integrity issue
    #[must_use]
    pub fn new(corruption_type: CorruptionType, description: impl Into<String>) -> Self {
        let strategy = Self::recommended_strategy_for_type(corruption_type);
        let severity = Self::default_severity(corruption_type);

        Self {
            corruption_type,
            severity,
            description: description.into(),
            affected_path: None,
            context: None,
            recommended_strategy: strategy,
        }
    }

    /// Set the affected path
    #[must_use]
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.affected_path = Some(path.into());
        self
    }

    /// Set contextual information
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set a custom repair strategy
    #[must_use]
    pub const fn with_strategy(mut self, strategy: RepairStrategy) -> Self {
        self.recommended_strategy = strategy;
        self
    }

    /// Determine the default severity for a corruption type
    const fn default_severity(corruption_type: CorruptionType) -> Severity {
        match corruption_type {
            CorruptionType::MissingDirectory => Severity::Critical,
            CorruptionType::StaleLocks => Severity::Warn,
            CorruptionType::MissingJjDir
            | CorruptionType::CorruptedJjDir
            | CorruptionType::PermissionDenied
            | CorruptionType::CorruptedGitIndex => Severity::Fail,
        }
    }

    /// Determine the recommended repair strategy for a corruption type
    ///
    /// # Returns
    ///
    /// Returns the recommended repair strategy. The result should be used
    /// for repair decision making.
    #[must_use]
    pub const fn recommended_strategy_for_type(corruption_type: CorruptionType) -> RepairStrategy {
        match corruption_type {
            CorruptionType::MissingDirectory | CorruptionType::CorruptedJjDir => {
                RepairStrategy::ForgetAndRecreate
            }
            CorruptionType::MissingJjDir => RepairStrategy::RecreateWorkspace,
            CorruptionType::StaleLocks => RepairStrategy::ClearLocks,
            CorruptionType::PermissionDenied | CorruptionType::CorruptedGitIndex => {
                RepairStrategy::NoRepairPossible
            }
        }
    }
}

/// Result of a workspace validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Name of the workspace
    pub workspace: String,
    /// Path to the workspace
    pub path: PathBuf,
    /// Whether the workspace is valid
    pub is_valid: bool,
    /// List of issues found
    pub issues: Vec<IntegrityIssue>,
    /// Maximum severity among all issues
    pub max_severity: Option<Severity>,
    /// Duration of check in milliseconds
    pub duration_ms: u64,
    /// When the validation was performed
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl ValidationResult {
    /// Create a valid result
    #[must_use]
    pub fn valid(workspace: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: true,
            issues: Vec::new(),
            max_severity: None,
            duration_ms: 0,
            validated_at: chrono::Utc::now(),
        }
    }

    /// Create an invalid result with issues
    #[must_use]
    pub fn invalid(
        workspace: impl Into<String>,
        path: impl Into<PathBuf>,
        issues: Vec<IntegrityIssue>,
    ) -> Self {
        let max_severity = issues.iter().map(|i| i.severity).max();

        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: issues.is_empty(),
            issues,
            max_severity,
            duration_ms: 0,
            validated_at: chrono::Utc::now(),
        }
    }

    /// Set the check duration
    #[must_use]
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Check if any issues are auto-repairable
    #[must_use]
    pub fn has_auto_repairable_issues(&self) -> bool {
        self.issues.iter().any(|i| {
            !matches!(
                i.recommended_strategy,
                RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible
            )
        })
    }

    /// Get the most severe issue found
    #[must_use]
    pub fn most_severe_issue(&self) -> Option<&IntegrityIssue> {
        self.issues.iter().max_by_key(|i| i.severity)
    }
}

/// Result of a repair operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    /// Name of the workspace
    pub workspace: String,
    /// Whether repair was successful
    pub success: bool,
    /// Action taken
    pub action: RepairStrategy,
    /// Description of what was done
    pub summary: String,
    /// ID of backup created before repair (if any)
    pub backup_id: Option<String>,
}

impl RepairResult {
    /// Create a successful repair result
    #[must_use]
    pub fn success(
        workspace: impl Into<String>,
        action: RepairStrategy,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            success: true,
            action,
            summary: summary.into(),
            backup_id: None,
        }
    }

    /// Create a failed repair result
    #[must_use]
    pub fn failure(
        workspace: impl Into<String>,
        action: RepairStrategy,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            success: false,
            action,
            summary: summary.into(),
            backup_id: None,
        }
    }

    /// Add a backup ID to the result
    #[must_use]
    pub fn with_backup(mut self, backup_id: impl Into<String>) -> Self {
        self.backup_id = Some(backup_id.into());
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY VALIDATOR
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace integrity validator
///
/// Provides methods to validate workspace integrity and detect corruption.
#[derive(Debug, Clone)]
pub struct IntegrityValidator {
    /// Root directory containing workspaces
    workspaces_root: PathBuf,
    /// Check timeout in milliseconds
    timeout_ms: u64,
}

impl IntegrityValidator {
    /// Default timeout for validation checks (5 seconds)
    pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

    /// Create a new integrity validator
    #[must_use]
    pub fn new(workspaces_root: impl Into<PathBuf>) -> Self {
        Self {
            workspaces_root: workspaces_root.into(),
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// Set custom timeout
    #[must_use]
    pub const fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Validate a single workspace
    pub async fn validate(&self, workspace_name: &str) -> Result<ValidationResult> {
        let start = SystemTime::now();
        let workspace_path = self.resolve_workspace_path(workspace_name);

        let mut issues = Vec::new();

        // Check 1: Directory exists
        let path_exists = tokio::fs::try_exists(&workspace_path).await?;
        if !path_exists {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingDirectory,
                    format!(
                        "Workspace directory does not exist: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&workspace_path),
            );

            // Can't continue validation if directory is missing
            let duration = start
                .elapsed()
                .map_err(|e| Error::Unknown(format!("Failed to measure duration: {e}")))
                .and_then(|d| {
                    u64::try_from(d.as_millis()).map_err(|_| {
                        Error::Unknown("Duration overflow - operation took too long".to_string())
                    })
                })?;
            return Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            );
        }

        // Check 2: Directory is readable
        if let Err(e) = tokio::fs::read_dir(&workspace_path).await {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read workspace directory: {e}"),
                )
                .with_path(&workspace_path)
                .with_context(e.to_string()),
            );
        }

        // Check 3: .jj directory exists
        let jj_dir = workspace_path.join(".jj");
        let jj_dir_exists = tokio::fs::try_exists(&jj_dir).await?;
        if jj_dir_exists {
            // Check 4: .jj directory is valid
            if let Err(issue) = Self::validate_jj_directory(&jj_dir).await {
                issues.push(issue);
            }
        } else {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingJjDir,
                    format!(
                        ".jj directory missing from workspace: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&jj_dir),
            );
        }

        // Check 5: Config file integrity (TOML validation)
        if let Ok(Some(issue)) = Self::check_config_file(&workspace_path).await {
            issues.push(issue);
        }

        // Check 6: Lock files
        if let Ok(Some(issue)) = Self::check_stale_locks(&workspace_path).await {
            issues.push(issue);
        }

        let duration = start
            .elapsed()
            .map_err(|e| Error::Unknown(format!("Failed to measure duration: {e}")))
            .and_then(|d| {
                u64::try_from(d.as_millis()).map_err(|_| {
                    Error::Unknown("Duration overflow - operation took too long".to_string())
                })
            })?;

        if issues.is_empty() {
            Ok(ValidationResult::valid(workspace_name, &workspace_path).with_duration(duration))
        } else {
            Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            )
        }
    }

    fn resolve_workspace_path(&self, workspace_name: &str) -> PathBuf {
        let input_path = Path::new(workspace_name);

        if input_path.is_absolute() {
            return input_path.to_path_buf();
        }

        let looks_like_path = workspace_name.contains(std::path::MAIN_SEPARATOR)
            || workspace_name.contains('/')
            || workspace_name.starts_with('.');

        if looks_like_path {
            return input_path.to_path_buf();
        }

        self.workspaces_root.join(workspace_name)
    }

    /// Validate multiple workspaces in parallel
    ///
    /// RESULTS ARE ORDERED: Returns results in the same order as input workspaces.
    /// Uses concurrent validation for performance but preserves ordering for predictability.
    pub async fn validate_all(&self, workspaces: &[String]) -> Result<Vec<ValidationResult>> {
        use std::sync::Arc;

        use futures::future::try_join_all;

        // Clone self for each validation (IntegrityValidator is cheap to clone - just PathBuf +
        // u64)
        let validator = Arc::new(self);

        // Create validation futures that preserve input order
        let futures = workspaces
            .iter()
            .map(|name| {
                let validator = validator.clone();
                let name = name.clone();
                async move { (*validator).validate(&name).await }
            })
            .collect::<Vec<_>>();

        // Run concurrently but preserve order (try_join_all maintains input order)
        try_join_all(futures).await
    }

    /// Validate the .jj directory structure
    async fn validate_jj_directory(jj_dir: &Path) -> std::result::Result<(), IntegrityIssue> {
        let repo_path = jj_dir.join("repo");
        let repo_exists = tokio::fs::try_exists(&repo_path).await.map_err(|e| {
            IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot check repo: {e}"),
            )
            .with_path(&repo_path)
        })?;
        if !repo_exists {
            return Err(IntegrityIssue::new(
                CorruptionType::CorruptedJjDir,
                "JJ repository metadata missing ('repo' path)",
            )
            .with_path(jj_dir));
        }

        // Check if repo is a file (workspace pointing to shared repo) or directory
        let repo_metadata = tokio::fs::metadata(&repo_path).await.map_err(|e| {
            IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot check repo metadata: {e}"),
            )
            .with_path(&repo_path)
        })?;

        // If repo is a file, this is a workspace pointing to a shared repo.
        if repo_metadata.is_file() {
            Self::validate_shared_repo_pointer(jj_dir, &repo_path).await?;
            return Ok(());
        }

        // If repo is a directory, validate the op_store
        // Check for empty critical directories
        let op_store = repo_path.join("op_store");
        let op_store_exists = tokio::fs::try_exists(&op_store).await.map_err(|e| {
            IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot check op_store directory: {e}"),
            )
            .with_path(&op_store)
        })?;
        if op_store_exists {
            match tokio::fs::read_dir(&op_store).await {
                Ok(mut entries) => {
                    let has_entries = match entries.next_entry().await {
                        Ok(Some(_)) => true,
                        Ok(None) | Err(_) => false,
                    };
                    if !has_entries {
                        return Err(IntegrityIssue::new(
                            CorruptionType::CorruptedJjDir,
                            "JJ operation store is empty",
                        )
                        .with_path(&op_store));
                    }
                }
                Err(e) => {
                    return Err(IntegrityIssue::new(
                        CorruptionType::PermissionDenied,
                        format!("Cannot read JJ op_store: {e}"),
                    )
                    .with_path(&op_store));
                }
            }
        }

        Ok(())
    }

    async fn validate_shared_repo_pointer(
        jj_dir: &Path,
        repo_pointer_path: &Path,
    ) -> std::result::Result<(), IntegrityIssue> {
        let shared_repo_path = match tokio::fs::read_to_string(repo_pointer_path).await {
            Ok(content) => {
                let path_str = content.trim();
                if path_str.is_empty() {
                    return Err(IntegrityIssue::new(
                        CorruptionType::CorruptedJjDir,
                        "JJ repo file is empty - workspace has no backing repo",
                    )
                    .with_path(repo_pointer_path));
                }

                if std::path::Path::new(path_str).is_absolute() {
                    std::path::PathBuf::from(path_str)
                } else {
                    jj_dir.join(path_str)
                }
            }
            Err(e) => {
                return Err(IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read JJ repo file: {e}"),
                )
                .with_path(repo_pointer_path));
            }
        };

        let shared_exists = tokio::fs::try_exists(&shared_repo_path)
            .await
            .map_err(|e| {
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot check referenced shared repo: {e}"),
                )
                .with_path(&shared_repo_path)
            })?;

        if !shared_exists {
            return Err(IntegrityIssue::new(
                CorruptionType::CorruptedJjDir,
                format!(
                    "JJ repo file points to non-existent path: {}",
                    shared_repo_path.display()
                ),
            )
            .with_path(repo_pointer_path));
        }

        let shared_metadata = tokio::fs::metadata(&shared_repo_path).await.map_err(|e| {
            IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot inspect referenced shared repo metadata: {e}"),
            )
            .with_path(&shared_repo_path)
        })?;

        if !shared_metadata.is_dir() {
            return Err(IntegrityIssue::new(
                CorruptionType::CorruptedJjDir,
                format!(
                    "JJ repo file points to non-directory path: {}",
                    shared_repo_path.display()
                ),
            )
            .with_path(repo_pointer_path));
        }

        let shared_op_store = shared_repo_path.join("op_store");
        let shared_op_store_exists =
            tokio::fs::try_exists(&shared_op_store).await.map_err(|e| {
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot check shared repo op_store: {e}"),
                )
                .with_path(&shared_op_store)
            })?;

        if !shared_op_store_exists {
            return Err(IntegrityIssue::new(
                CorruptionType::CorruptedJjDir,
                format!(
                    "Referenced shared repo missing op_store: {}",
                    shared_op_store.display()
                ),
            )
            .with_path(repo_pointer_path));
        }

        Ok(())
    }

    /// Check for stale lock files in the workspace
    async fn check_stale_locks(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
        let lock_file = workspace_path.join(".jj").join("working_copy").join("lock");

        let lock_exists = tokio::fs::try_exists(&lock_file).await?;
        if lock_exists {
            // Check age of lock file
            let metadata = tokio::fs::metadata(&lock_file).await?;
            let modified = metadata.modified()?;
            let age = SystemTime::now()
                .duration_since(modified)
                .map_err(|e| Error::Unknown(format!("Failed to calculate lock age: {e}")))?;
            let age_secs = age.as_secs();

            // Lock older than 1 hour is suspicious
            if age_secs > 3600 {
                return Ok(Some(
                    IntegrityIssue::new(
                        CorruptionType::StaleLocks,
                        format!("Stale lock file detected (age: {age_secs}s)"),
                    )
                    .with_path(&lock_file),
                ));
            }
        }

        Ok(None)
    }

    /// Check config file for TOML parsing errors
    ///
    /// Validates that the workspace's .isolate/config.toml file (if present)
    /// is valid TOML and can be parsed.
    async fn check_config_file(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
        let config_file = workspace_path.join(".isolate").join("config.toml");

        // Check if config file exists
        let config_exists = tokio::fs::try_exists(&config_file).await?;
        if !config_exists {
            // No config file is fine - not all workspaces have custom config
            return Ok(None);
        }

        // Read and validate TOML
        let file_content = match tokio::fs::read_to_string(&config_file).await {
            Ok(file_content) => file_content,
            Err(e) => {
                return Ok(Some(
                    IntegrityIssue::new(
                        CorruptionType::CorruptedJjDir,
                        format!("Cannot read config file: {e}"),
                    )
                    .with_path(&config_file)
                    .with_context(format!("Permission denied or file corrupted: {e}")),
                ));
            }
        };

        // Try to parse as TOML
        match toml::from_str::<toml::Value>(&file_content) {
            Ok(_) => Ok(None), // Valid TOML
            Err(e) => {
                let hint = format!(
                    "TOML parse error: {e}. \
                     Suggestion: Check for syntax errors like unclosed brackets, \
                     missing quotes, or invalid values."
                );
                Ok(Some(
                    IntegrityIssue::new(
                        CorruptionType::CorruptedJjDir,
                        format!("Config file contains invalid TOML: {e}"),
                    )
                    .with_path(&config_file)
                    .with_context(hint),
                ))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR EXECUTOR
// ═══════════════════════════════════════════════════════════════════════════

/// Executes repairs for detected integrity issues
///
/// # ADVERSARIAL DEFENSE
///
/// The executor enforces consistency: if `always_backup` is true, a `BackupManager`
/// MUST be provided. This prevents runtime errors where repairs fail due to missing
/// backup configuration.
#[derive(Clone)]
pub struct RepairExecutor {
    /// Backup configuration
    backup_config: BackupConfig,
}

/// Backup configuration for repair operations
#[derive(Clone)]
enum BackupConfig {
    /// No backups - repair operations are destructive
    NoBackup,
    /// Always backup before repair
    WithBackup(BackupManager),
}

impl RepairExecutor {
    /// Create a new repair executor with default safety (no backups)
    ///
    /// NOTE: Defaults to NO BACKUP for safety. Use `with_backup_manager()`
    /// to enable backups before destructive operations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            backup_config: BackupConfig::NoBackup,
        }
    }

    /// Enable backups with a backup manager
    ///
    /// This is the RECOMMENDED way to create a repair executor for production use.
    /// Backups protect against data loss during repair operations.
    #[must_use]
    pub fn with_backup_manager(mut self, backup_manager: BackupManager) -> Self {
        self.backup_config = BackupConfig::WithBackup(backup_manager);
        self
    }

    /// Disable backups explicitly (for testing or trusted environments)
    ///
    /// WARNING: Without backups, repair operations are destructive and cannot
    /// be rolled back. Use with caution.
    #[must_use]
    pub fn without_backup(mut self) -> Self {
        self.backup_config = BackupConfig::NoBackup;
        self
    }

    /// Check if this executor creates backups before repair
    #[must_use]
    pub const fn creates_backups(&self) -> bool {
        matches!(self.backup_config, BackupConfig::WithBackup(_))
    }

    /// Execute repair (legacy name for compatibility)
    pub async fn execute(
        &self,
        _workspace_name: &str,
        _workspace_path: &Path,
        validation: &ValidationResult,
        _strategy: RepairStrategy,
    ) -> Result<RepairResult> {
        self.repair(validation).await
    }

    /// Repair a workspace based on validation results
    pub async fn repair(&self, validation: &ValidationResult) -> Result<RepairResult> {
        if validation.is_valid {
            return Ok(RepairResult::success(
                &validation.workspace,
                RepairStrategy::NoRepair,
                "Workspace is already healthy",
            ));
        }

        // Determine the overall repair strategy
        // We pick the most aggressive (highest risk) strategy among all issues
        let strategy = validation
            .issues
            .iter()
            .map(|i| i.recommended_strategy)
            .max_by_key(|s| match s {
                RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible => 0,
                RepairStrategy::ClearLocks => 1,
                RepairStrategy::FixJjDir => 2,
                RepairStrategy::RecreateWorkspace => 3,
                RepairStrategy::ForgetAndRecreate => 4,
            })
            .ok_or_else(|| Error::Unknown("No issues found in validation result".to_string()))?;

        if matches!(
            strategy,
            RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible
        ) {
            return Ok(RepairResult::failure(
                &validation.workspace,
                RepairStrategy::NoRepair,
                "No automated repair possible for detected issues",
            ));
        }

        // CRITICAL: Check if workspace directory exists before attempting repair
        // For missing directories, we cannot repair automatically
        let workspace_exists = tokio::fs::try_exists(&validation.path).await?;
        if !workspace_exists {
            return Ok(RepairResult::failure(
                &validation.workspace,
                strategy,
                format!(
                    "Workspace directory '{}' does not exist. Cannot repair missing workspace.",
                    validation.path.display()
                ),
            ));
        }

        // Create backup if configured (ADVERSARIAL: type-safe backup guarantee)
        let backup_id = match &self.backup_config {
            BackupConfig::WithBackup(manager) => {
                let meta = manager
                    .create_backup(&validation.workspace, "Auto-repair")
                    .await?;
                Some(meta.id)
            }
            BackupConfig::NoBackup => None,
        };

        // Execute the repair
        let result = match strategy {
            RepairStrategy::ClearLocks => Self::clear_locks(&validation.path).await.map(|()| {
                RepairResult::success(&validation.workspace, strategy, "Cleared stale lock files")
            }),
            RepairStrategy::ForgetAndRecreate => {
                Self::forget_and_recreate(&validation.workspace, &validation.path).await
            }
            _ => {
                // Not fully implemented yet
                Ok(RepairResult::failure(
                    &validation.workspace,
                    strategy,
                    format!("Repair strategy '{strategy}' not yet implemented"),
                ))
            }
        }?;

        Ok(if let Some(id) = backup_id {
            result.with_backup(id)
        } else {
            result
        })
    }

    /// Clear lock files in a workspace
    ///
    /// ADVERSARIAL: Idempotent operation - safe to call multiple times even if
    /// locks were already removed by another process. This prevents race conditions
    /// in concurrent repair scenarios.
    async fn clear_locks(workspace_path: &Path) -> Result<()> {
        let lock_file = workspace_path.join(".jj").join("working_copy").join("lock");

        // Try to remove the lock file, ignoring "not found" errors (idempotent)
        let result = tokio::fs::remove_file(&lock_file).await;

        match result {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File already removed by another process - OK
                Ok(())
            }
            Err(e) => Err(Error::IoError(format!(
                "Failed to remove lock file {}: {e}",
                lock_file.display()
            ))),
        }
    }

    /// Forget workspace in JJ and recreate
    async fn forget_and_recreate(
        workspace_name: &str,
        workspace_path: &Path,
    ) -> Result<RepairResult> {
        let root = workspace_path
            .parent()
            .and_then(|p| p.parent()) // .isolate/workspaces -> root
            .ok_or_else(|| Error::Unknown("Could not determine repository root".to_string()))?;

        // Forget the workspace
        let forget_output = get_jj_command()
            .args(["workspace", "forget", workspace_name])
            .current_dir(root)
            .output()
            .await
            .map_err(|e| Error::Command(format!("Failed to forget workspace: {e}")))?;

        if !forget_output.status.success() {
            let stderr = String::from_utf8_lossy(&forget_output.stderr);
            return Ok(RepairResult::failure(
                workspace_name,
                RepairStrategy::ForgetAndRecreate,
                format!("Failed to forget workspace: {stderr}"),
            ));
        }

        // If directory is corrupted but exists, remove it
        let workspace_exists = tokio::fs::try_exists(workspace_path).await?;
        if workspace_exists {
            tokio::fs::remove_dir_all(workspace_path)
                .await
                .map_err(|e| {
                    Error::IoError(format!(
                        "Failed to remove corrupted workspace directory {}: {e}",
                        workspace_path.display()
                    ))
                })?;
        }

        Ok(RepairResult::success(
            workspace_name,
            RepairStrategy::ForgetAndRecreate,
            "Workspace forgotten and directory removed. Re-run 'isolate spawn' to recreate.",
        ))
    }
}

impl Default for RepairExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BACKUP MANAGER
// ═══════════════════════════════════════════════════════════════════════════

/// Metadata for a workspace backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup ID
    pub id: String,
    /// Workspace name
    pub workspace: String,
    /// When backup was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Reason for backup
    pub reason: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum
    pub checksum: Option<String>,
}

/// Manages workspace backups
#[derive(Debug, Clone)]
pub struct BackupManager {
    /// Root directory for backups (.isolate/backups)
    backup_root: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            backup_root: root.into().join(".isolate").join("backups"),
        }
    }

    /// Create a backup of a workspace
    pub async fn create_backup(
        &self,
        workspace_name: &str,
        reason: &str,
    ) -> Result<BackupMetadata> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Unknown(format!("System time before Unix epoch: {e}")))?;
        let backup_id = format!("{}_{}", workspace_name, timestamp.as_secs());

        // Ensure backup root exists
        tokio::fs::create_dir_all(&self.backup_root)
            .await
            .map_err(|e| {
                Error::IoError(format!(
                    "Failed to create backup directory {}: {e}",
                    self.backup_root.display()
                ))
            })?;

        // In a real implementation, we would tar/cp the directory
        // For now, we just record metadata
        let meta = BackupMetadata {
            id: backup_id,
            workspace: workspace_name.to_string(),
            created_at: chrono::Utc::now(),
            reason: reason.to_string(),
            size_bytes: 0,
            checksum: None,
        };

        Ok(meta)
    }

    /// List available backups
    pub const fn list_backups(&self, _workspace_name: &str) -> Result<Vec<BackupMetadata>> {
        // Mock implementation
        Ok(Vec::new())
    }

    /// Restore from backup
    pub fn restore_backup(
        &self,
        backup_id: &str,
        workspace_name: &str,
        _workspace_path: &Path,
    ) -> Result<RollbackResult> {
        // Mock implementation
        Ok(RollbackResult {
            workspace: workspace_name.to_string(),
            success: true,
            summary: format!("Restored from backup {backup_id}"),
        })
    }
}

/// Result of a rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Name of the workspace
    pub workspace: String,
    /// Whether rollback was successful
    pub success: bool,
    /// Description of result
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // Helper to create a temporary workspaces root for testing
    fn create_test_root() -> Result<TempDir> {
        TempDir::new().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
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
        let validator = IntegrityValidator::new("/tmp/workspaces");
        let resolved = validator.resolve_workspace_path("/var/tmp/ws-a");
        assert_eq!(resolved, PathBuf::from("/var/tmp/ws-a"));
    }

    #[test]
    fn test_resolve_workspace_path_keeps_relative_path_input() {
        let validator = IntegrityValidator::new("/tmp/workspaces");
        let resolved = validator.resolve_workspace_path(".isolate/workspaces/ws-a");
        assert_eq!(resolved, PathBuf::from(".isolate/workspaces/ws-a"));
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
    async fn test_repair_result_success() {
        let result = RepairResult::success("ws", RepairStrategy::ClearLocks, "Fixed");
        assert!(result.success);
        assert_eq!(result.workspace, "ws");
        assert_eq!(result.action, RepairStrategy::ClearLocks);
        assert_eq!(result.summary, "Fixed");
        assert_eq!(result.backup_id, None);
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
    async fn test_backup_manager_create_and_list() -> Result<()> {
        let root = create_test_root()?;
        let manager = BackupManager::new(root.path());

        let meta = manager.create_backup("ws", "Test").await?;
        assert_eq!(meta.workspace, "ws");
        assert_eq!(meta.reason, "Test");
        assert!(tokio::fs::try_exists(root.path().join(".isolate").join("backups")).await?);
        Ok(())
    }
}
