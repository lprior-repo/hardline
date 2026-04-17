//! Action functions for backup command handler (Tier 3).
//!
//! I/O operations that interact with the filesystem.
//! All validation is delegated to Tier 2 (calculations).

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};
use tokio::fs;

use super::calculations::{
    backups_to_remove, build_retention_status, generate_backup_filename, get_backupable_databases,
    get_database_backup_dir, parse_backup_filename, resolve_database_target,
    validate_backup_command, validate_timestamp,
};
use super::data::{
    BackupCommand, BackupConfig, BackupCreateOutput, BackupInfo, BackupInfoOutput,
    BackupListOutput, BackupMetadata, BackupRestoreOutput, BackupStatusOutput, DatabaseBackups,
    RetentionStatusOutput,
};

// ============================================================================
// Public API
// ============================================================================

/// Execute a validated backup command.
///
/// Dispatches to the appropriate subcommand handler after validation.
///
/// # Errors
///
/// Returns errors from validation failure or subcommand execution.
pub async fn execute_backup_command(
    cmd: &BackupCommand,
    root: &Path,
    config: &BackupConfig,
) -> Result<()> {
    // TIER 2: Validate before any I/O
    validate_backup_command(cmd)?;

    // TIER 3: Dispatch to subcommand handler
    match cmd {
        BackupCommand::Create => execute_create(root, config).await,
        BackupCommand::List => execute_list(root, config).await,
        BackupCommand::Restore {
            database,
            timestamp,
        } => {
            if let Some(ts) = timestamp {
                validate_timestamp(ts)?;
            }
            execute_restore(root, database, timestamp.as_deref(), config).await
        }
        BackupCommand::Retention => execute_retention(root, config).await,
        BackupCommand::Status => execute_status(root, config).await,
    }
}

// ============================================================================
// Create
// ============================================================================

async fn execute_create(root: &Path, config: &BackupConfig) -> Result<()> {
    let backupable = get_backupable_databases(root);
    let mut created_paths = Vec::new();

    for (db_name, db_path) in &backupable {
        if !db_path.exists() {
            tracing::warn!(
                "Database file does not exist, skipping: {}",
                db_path.display()
            );
            continue;
        }
        match create_backup(db_path, db_name, config).await {
            Ok(path) => created_paths.push(path),
            Err(e) => {
                tracing::warn!("Failed to backup {db_name}: {e}");
            }
        }
    }

    let output = BackupCreateOutput {
        backup_count: created_paths.len(),
        backup_paths: created_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
    };

    if output.backup_count > 0 {
        Output::success(&format!("Created {} backup(s)", output.backup_count));
        for path in &output.backup_paths {
            Output::info(&format!("  {path}"));
        }
    } else {
        Output::info("No databases found to back up");
    }

    Ok(())
}

/// Create a backup of a single database file.
///
/// Copies the file, computes a SHA-256 checksum, and writes metadata alongside.
async fn create_backup(
    database_path: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<std::path::PathBuf> {
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);
    fs::create_dir_all(&backup_dir).await.map_err(|e| {
        Error::internal(format!(
            "Failed to create backup directory '{}': {e}",
            backup_dir.display()
        ))
    })?;

    let timestamp = chrono::Utc::now();
    let backup_filename = generate_backup_filename(&timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    fs::copy(database_path, &backup_path).await.map_err(|e| {
        Error::internal(format!(
            "Failed to copy database file to '{}': {e}",
            backup_path.display()
        ))
    })?;

    let metadata = fs::metadata(&backup_path).await.map_err(|e| {
        Error::internal(format!(
            "Failed to get backup file metadata '{}': {e}",
            backup_path.display()
        ))
    })?;
    let size_bytes = metadata.len();

    let checksum = compute_checksum(&backup_path).await?;

    let backup_meta = BackupMetadata::new(database_name.to_string(), size_bytes, checksum);
    let metadata_path = backup_path.with_extension("json");
    let metadata_json = serde_json::to_string_pretty(&backup_meta)
        .map_err(|e| Error::internal(format!("Failed to serialize backup metadata: {e}")))?;

    fs::write(&metadata_path, metadata_json)
        .await
        .map_err(|e| {
            Error::internal(format!(
                "Failed to write backup metadata '{}': {e}",
                metadata_path.display()
            ))
        })?;

    Ok(backup_path)
}

// ============================================================================
// List
// ============================================================================

async fn execute_list(root: &Path, config: &BackupConfig) -> Result<()> {
    let databases = list_all_backups(root, config).await?;

    if databases.is_empty() {
        Output::info("No backups found");
        return Ok(());
    }

    let total_count: usize = databases.iter().map(|d| d.backup_count).sum();
    Output::info(&format!(
        "Backups ({} databases, {} total):",
        databases.len(),
        total_count
    ));

    for db in &databases {
        Output::info(&format!(
            "  {} ({} backup(s)):",
            db.database, db.backup_count
        ));
        for backup in &db.backups {
            Output::info(&format!(
                "    {} - {} bytes",
                backup.timestamp.format("%Y-%m-%d %H:%M:%S"),
                backup.size_bytes
            ));
        }
    }

    Ok(())
}

/// List all backups across all databases.
async fn list_all_backups(root: &Path, config: &BackupConfig) -> Result<Vec<DatabaseBackups>> {
    let known_dbs = &["state.db", "beads.db"];
    let mut all_backups = Vec::new();

    for db_name in known_dbs {
        let backups = list_database_backups(root, db_name, config).await?;
        if !backups.is_empty() {
            all_backups.push(DatabaseBackups {
                database: db_name.to_string(),
                backup_count: backups.len(),
                backups: backups
                    .iter()
                    .map(|b| BackupInfoOutput {
                        path: b.path.display().to_string(),
                        timestamp: b.timestamp,
                        size_bytes: b.size_bytes,
                    })
                    .collect(),
            });
        }
    }

    Ok(all_backups)
}

/// List backups for a specific database.
async fn list_database_backups(
    _root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<Vec<BackupInfo>> {
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&backup_dir).await.map_err(|e| {
        Error::internal(format!(
            "Failed to read backup directory '{}': {e}",
            backup_dir.display()
        ))
    })?;

    let mut backups = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        // Only process .db files (skip .json metadata files)
        if path.extension().and_then(|s| s.to_str()) != Some("db") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::internal("Invalid backup filename"))?;

        let Ok(timestamp) = parse_backup_filename(filename) else {
            continue; // Skip unparseable filenames
        };

        let size_bytes = fs::metadata(&path).await.map_or(0, |m| m.len());

        // Try to load metadata
        let metadata_path = path.with_extension("json");
        let metadata = if metadata_path.exists() {
            fs::read_to_string(&metadata_path)
                .await
                .ok()
                .and_then(|json| serde_json::from_str::<BackupMetadata>(&json).ok())
        } else {
            None
        };

        backups.push(BackupInfo {
            path,
            timestamp,
            metadata,
            size_bytes,
        });
    }

    // Sort newest first
    backups.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

    Ok(backups)
}

// ============================================================================
// Restore
// ============================================================================

async fn execute_restore(
    root: &Path,
    database: &str,
    timestamp: Option<&str>,
    config: &BackupConfig,
) -> Result<()> {
    let target_path = resolve_database_target(root, database)?;

    let backup_path = if let Some(ts) = timestamp {
        // Find specific backup with matching timestamp
        let backups = list_database_backups(root, database, config).await?;
        backups
            .iter()
            .find(|b| b.timestamp.format("%Y%m%d-%H%M%S").to_string() == ts)
            .map(|b| b.path.clone())
            .ok_or_else(|| Error::internal(format!("No backup found with timestamp: {ts}")))?
    } else {
        find_latest_backup(root, database, config).await?
    };

    restore_backup(&backup_path, &target_path, true).await?;

    Output::success(&format!("Restored database '{database}'"));
    Output::info(&format!("  From: {}", backup_path.display()));
    Output::info(&format!("  To:   {}", target_path.display()));

    Ok(())
}

/// Find the most recent backup for a database.
async fn find_latest_backup(
    _root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<std::path::PathBuf> {
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);

    if !backup_dir.exists() {
        return Err(Error::internal(format!(
            "No backups found for database: {database_name}"
        )));
    }

    let backups = list_database_backups(_root, database_name, config).await?;

    backups.into_iter().next().map(|b| b.path).ok_or_else(|| {
        Error::internal(format!(
            "No valid backups found for database: {database_name}"
        ))
    })
}

/// Restore a database from a backup with checksum verification.
async fn restore_backup(
    backup_path: &Path,
    target_path: &Path,
    verify_checksum: bool,
) -> Result<()> {
    if !backup_path.exists() {
        return Err(Error::internal(format!(
            "Backup file does not exist: {}",
            backup_path.display()
        )));
    }

    let metadata_path = backup_path.with_extension("json");
    if !metadata_path.exists() {
        return Err(Error::internal(format!(
            "Backup metadata file does not exist: {}",
            metadata_path.display()
        )));
    }

    let metadata_json = fs::read_to_string(&metadata_path)
        .await
        .map_err(|e| Error::internal(format!("Failed to read backup metadata: {e}")))?;

    let metadata: BackupMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| Error::internal(format!("Failed to parse backup metadata: {e}")))?;

    if verify_checksum {
        let current_checksum = compute_checksum(backup_path).await?;
        if current_checksum != metadata.checksum {
            return Err(Error::internal(format!(
                "Checksum verification failed: expected {}, got {}",
                metadata.checksum, current_checksum
            )));
        }
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::internal(format!("Failed to create target directory: {e}")))?;
    }

    fs::copy(backup_path, target_path).await.map_err(|e| {
        Error::internal(format!(
            "Failed to copy backup to target '{}': {e}",
            target_path.display()
        ))
    })?;

    Ok(())
}

// ============================================================================
// Retention
// ============================================================================

async fn execute_retention(root: &Path, config: &BackupConfig) -> Result<()> {
    let known_dbs = &["state.db", "beads.db"];
    let mut all_removed = Vec::new();

    for db_name in known_dbs {
        match apply_retention_policy(root, db_name, config).await {
            Ok(removed) => all_removed.extend(removed),
            Err(e) => {
                tracing::warn!("Failed to apply retention to {db_name}: {e}");
            }
        }
    }

    if all_removed.is_empty() {
        Output::info("All backups are within retention limits; nothing to remove");
    } else {
        Output::success(&format!("Removed {} old backup(s)", all_removed.len()));
        for path in &all_removed {
            Output::info(&format!("  Removed: {path}"));
        }
    }

    Ok(())
}

async fn apply_retention_policy(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<Vec<String>> {
    let backups = list_database_backups(root, database_name, config).await?;
    let to_remove = backups_to_remove(&backups, config.retention_count);

    let mut removed_paths = Vec::new();

    for backup in to_remove {
        fs::remove_file(&backup.path).await.map_err(|e| {
            Error::internal(format!(
                "Failed to remove backup '{}': {e}",
                backup.path.display()
            ))
        })?;

        let metadata_path = backup.path.with_extension("json");
        if metadata_path.exists() {
            let _ = fs::remove_file(&metadata_path).await;
        }

        removed_paths.push(backup.path.display().to_string());
    }

    Ok(removed_paths)
}

// ============================================================================
// Status
// ============================================================================

async fn execute_status(root: &Path, config: &BackupConfig) -> Result<()> {
    let statuses = get_retention_status(root, config).await?;

    Output::info(&format!(
        "Backup status (max {} per database):",
        config.retention_count
    ));

    for status in &statuses {
        let limit_indicator = if status.within_limit { "OK" } else { "OVER" };
        Output::info(&format!(
            "  {} - {} backup(s) [{}] ({} total)",
            status.database_name, status.backup_count, limit_indicator, status.total_size_human,
        ));
        if !status.within_limit {
            Output::info(&format!(
                "    Would free {} by applying retention",
                status.would_free_human
            ));
        }
    }

    Ok(())
}

async fn get_retention_status(
    root: &Path,
    config: &BackupConfig,
) -> Result<Vec<RetentionStatusOutput>> {
    let known_dbs = &["state.db", "beads.db"];
    let mut statuses = Vec::new();

    for db_name in known_dbs {
        let backups = list_database_backups(root, db_name, config).await?;
        let total_size: u64 = backups.iter().map(|b| b.size_bytes).sum();
        let to_remove = backups_to_remove(&backups, config.retention_count);
        let would_free: u64 = to_remove.iter().map(|b| b.size_bytes).sum();

        statuses.push(build_retention_status(
            db_name,
            backups.len(),
            total_size,
            would_free,
            config.retention_count,
        ));
    }

    Ok(statuses)
}

// ============================================================================
// Checksum
// ============================================================================

/// Compute SHA-256 checksum of a file.
pub(crate) async fn compute_checksum(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use tokio::{fs::File, io::AsyncReadExt};

    let mut file = File::open(path)
        .await
        .map_err(|e| Error::internal(format!("Failed to open file for checksum: {e}")))?;

    let mut hasher = Sha256::new();
    let mut chunk_buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut chunk_buffer)
            .await
            .map_err(|e| Error::internal(format!("Failed to read file for checksum: {e}")))?;

        if bytes_read == 0 {
            break;
        }
        hasher.update(&chunk_buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}
