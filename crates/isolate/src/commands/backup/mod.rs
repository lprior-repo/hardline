//! Database backup command handler

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![forbid(unsafe_code)]

mod backup_internal;

pub mod create;
pub mod list;
pub mod restore;
pub mod retention;

// Re-export backup types
use anyhow::Result;
pub use backup_internal::BackupConfig;
use isolate_core::{OutputFormat, SchemaEnvelope};

/// Create backup
///
/// # Errors
///
/// Returns error if backup creation fails
#[allow(dead_code)]
// Part of public API for backup commands
pub async fn run_create(root: &std::path::Path, format: OutputFormat) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);

    let config = BackupConfig::default();

    let backup_paths = create::backup_all_databases(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "backups": backup_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>(),
                "message": format!("Created {} backup(s)", backup_paths.len())
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

/// List backups
///
/// # Errors
///
/// Returns error if listing backups fails
#[allow(dead_code)]
// Part of public API for backup commands
pub async fn run_list(root: &std::path::Path, format: OutputFormat) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);

    let config = BackupConfig::default();
    let all_backups = list::list_all_backups(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "databases": all_backups
                    .iter()
                    .map(|(name, backups)| {
                        serde_json::json!({
                            "database": name,
                            "backup_count": backups.len(),
                            "backups": backups.iter().map(|b| serde_json::json!({
                                "path": b.path.display().to_string(),
                                "timestamp": b.timestamp.to_rfc3339(),
                                "size_bytes": b.size_bytes
                            })).collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>()
            });
            let envelope = SchemaEnvelope::new("backup-list-response", "single", output);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
    }

    Ok(())
}

/// Restore from backup
///
/// # Errors
///
/// Returns error if restore operation fails
#[allow(dead_code)]
// Part of public API for backup commands
pub async fn run_restore(
    root: &std::path::Path,
    database: &str,
    timestamp: Option<&str>,
    _format: OutputFormat,
) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);

    let config = BackupConfig::default();

    // If timestamp provided, find specific backup
    // Otherwise use latest
    let backup_path = if let Some(ts) = timestamp {
        // Find backup with matching timestamp
        let backups = list::list_database_backups(&root_path, database, &config).await?;
        backups
            .iter()
            .find(|b| b.timestamp.format("%Y%m%d-%H%M%S").to_string() == ts)
            .map(|b| b.path.clone())
            .ok_or_else(|| anyhow::anyhow!("No backup found with timestamp: {ts}"))?
    } else {
        restore::find_latest_backup(&root_path, database, &config).await?
    };

    // Determine target path
    // Note: queue.db has been merged into state.db (bd-30s)
    let target_path = match database {
        "state.db" => root_path.join(".isolate").join(database),
        "queue.db" => {
            // Legacy: restore to state.db since queue.db was merged
            root_path.join(".isolate").join("state.db")
        }
        "beads.db" => root_path.join(".beads").join(database),
        _ => anyhow::bail!("Unknown database: {database}"),
    };

    // Verify checksum by default
    restore::restore_backup(&backup_path, &target_path, true).await?;

    let output = serde_json::json!({
        "success": true,
        "database": database,
        "restored_from": backup_path.display().to_string(),
        "restored_to": target_path.display().to_string()
    });
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// Apply retention policy
///
/// # Errors
///
/// Returns error if retention policy application fails
#[allow(dead_code)]
// Part of public API for backup commands
pub async fn run_retention(root: &std::path::Path, _format: OutputFormat) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);

    let config = BackupConfig::default();
    let removed = retention::apply_retention_policy_all(&root_path, &config).await?;

    let output = serde_json::json!({
        "success": true,
        "removed_count": removed.len(),
        "removed_backups": removed
    });
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// Show backup status
///
/// # Errors
///
/// Returns error if status retrieval fails
#[allow(dead_code)]
// Part of public API for backup commands
pub async fn run_status(root: &std::path::Path, _format: OutputFormat) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);

    let config = BackupConfig::default();
    let statuses = retention::get_retention_status(&root_path, &config).await?;

    let output = serde_json::json!({
        "success": true,
        "retention_policy": {
            "max_backups_per_database": config.retention_count
        },
        "databases": statuses
            .iter()
            .map(|s| serde_json::json!({
                "database": s.database_name,
                "backup_count": s.backup_count,
                "retention_limit": s.retention_limit,
                "total_size_bytes": s.total_size_bytes,
                "would_free_bytes": s.would_free_bytes,
                "within_limit": s.within_limit,
                "total_size_human": retention::RetentionStatus::format_size(s.total_size_bytes),
                "would_free_human": retention::RetentionStatus::format_size(s.would_free_bytes)
            }))
            .collect::<Vec<_>>()
    });
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}