//! Checkpoint command - save and restore full session state snapshots
//!
//! Provides atomic save/restore of all session state, enabling rollback
//! to known-good configurations.

use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result as AnyhowResult};
use chrono::TimeZone;
use futures::{StreamExt, TryStreamExt};
use isolate_core::{workspace_state::WorkspaceState, OutputFormat};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use tar::Builder;
use tracing::warn;

use crate::session::{validate_session_name, Session, SessionStatus};

// ── Types ────────────────────────────────────────────────────────────

/// CLI arguments for the checkpoint command
pub struct CheckpointArgs {
    pub action: CheckpointAction,
    pub format: OutputFormat,
}

/// Which checkpoint action to perform
pub enum CheckpointAction {
    Create { description: Option<String> },
    Restore { checkpoint_id: String },
    List,
}

/// Response from checkpoint operations
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum CheckpointResponse {
    Created {
        checkpoint_id: String,
        metadata_only: Vec<String>,
    },
    Restored {
        checkpoint_id: String,
    },
    List {
        checkpoints: Vec<CheckpointInfo>,
    },
}

/// Information about a single checkpoint
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub created_at: String,
    pub session_count: usize,
    pub description: Option<String>,
}

// ── Schema ───────────────────────────────────────────────────────────

const CHECKPOINT_SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id TEXT UNIQUE NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS checkpoint_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id TEXT NOT NULL,
    session_name TEXT NOT NULL,
    status TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    branch TEXT,
    metadata TEXT,
    backup_path TEXT,
    backup_size INTEGER,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(checkpoint_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id ON checkpoint_sessions(checkpoint_id);

CREATE TABLE IF NOT EXISTS checkpoint_schema_meta (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    version INTEGER NOT NULL
);
";

const CHECKPOINT_SCHEMA_VERSION: i64 = 3;

/// Directory where workspace backups are stored
const CHECKPOINT_BACKUP_DIR: &str = ".beads/checkpoint_backups";

/// Default maximum backup size: 100MiB (prevent excessive disk usage)
const DEFAULT_MAX_BACKUP_SIZE: u64 = 100 * 1024 * 1024;

fn checkpoint_max_backup_size_bytes() -> u64 {
    std::env::var("Isolate_CHECKPOINT_MAX_BACKUP_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_BACKUP_SIZE)
}

fn checkpoint_backup_dir() -> PathBuf {
    std::env::var("Isolate_CHECKPOINT_BACKUP_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CHECKPOINT_BACKUP_DIR))
}

// ── Public entry point ───────────────────────────────────────────────

/// Run the checkpoint command
pub async fn run(args: &CheckpointArgs, pool: &SqlitePool) -> AnyhowResult<()> {
    ensure_checkpoint_tables(pool).await?;

    let response = match &args.action {
        CheckpointAction::Create { description } => {
            create_checkpoint(pool, description.as_deref()).await
        }
        CheckpointAction::Restore { checkpoint_id } => {
            restore_checkpoint(pool, checkpoint_id).await
        }
        CheckpointAction::List => list_checkpoints(pool).await,
    }?;

    output_response(&response, args.format)
}

// ── Implementation ───────────────────────────────────────────────────

async fn ensure_checkpoint_tables(pool: &SqlitePool) -> AnyhowResult<()> {
    sqlx::query(CHECKPOINT_SCHEMA)
        .execute(pool)
        .await
        .map(|_| ())
        .context("Failed to create checkpoint tables")?;

    ensure_checkpoint_schema(pool).await
}

async fn ensure_checkpoint_schema(pool: &SqlitePool) -> AnyhowResult<()> {
    recover_interrupted_checkpoint_migration(pool).await?;

    let current_version: Option<i64> =
        sqlx::query("SELECT version FROM checkpoint_schema_meta WHERE id = 1")
            .fetch_optional(pool)
            .await
            .context("Failed to read checkpoint schema version")?
            .map(|row| row.try_get("version"))
            .transpose()
            .context("Failed to parse checkpoint schema version")?;

    if let Some(version) = current_version {
        if version > CHECKPOINT_SCHEMA_VERSION {
            anyhow::bail!(
                "Checkpoint schema version mismatch: database has version {version}, but isolate expects version {CHECKPOINT_SCHEMA_VERSION}"
            );
        }
    }

    let needs_legacy_fk_migration = has_legacy_fk_to_sessions(pool).await?;

    if needs_legacy_fk_migration {
        migrate_legacy_checkpoint_fk(pool).await?;
    }

    // Add backup columns if migrating from version 2
    if current_version.is_some_and(|v| v < 3) {
        migrate_add_backup_columns(pool).await?;
    }

    sqlx::query(
        "INSERT INTO checkpoint_schema_meta (id, version) VALUES (1, ?)
         ON CONFLICT(id) DO UPDATE SET version = excluded.version",
    )
    .bind(CHECKPOINT_SCHEMA_VERSION)
    .execute(pool)
    .await
    .context("Failed to set checkpoint schema version")?;

    Ok(())
}

async fn table_exists(pool: &SqlitePool, table_name: &str) -> AnyhowResult<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ? LIMIT 1")
            .bind(table_name)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("Failed to inspect sqlite_master for table '{table_name}'"))?;

    Ok(row.is_some())
}

async fn recover_interrupted_checkpoint_migration(pool: &SqlitePool) -> AnyhowResult<()> {
    let old_exists = table_exists(pool, "checkpoints_old").await?;
    if !old_exists {
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin interrupted checkpoint migration recovery")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            checkpoint_id TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to ensure checkpoints table during migration recovery")?;

    sqlx::query(
        "INSERT INTO checkpoints (id, checkpoint_id, description, created_at)
         SELECT id, checkpoint_id, description, created_at FROM checkpoints_old
         WHERE NOT EXISTS (
             SELECT 1 FROM checkpoints c WHERE c.checkpoint_id = checkpoints_old.checkpoint_id
         )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to replay checkpoint rows from checkpoints_old")?;

    sqlx::query("DROP TABLE checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to drop checkpoints_old during migration recovery")?;

    tx.commit()
        .await
        .context("Failed to commit interrupted checkpoint migration recovery")
}

async fn has_legacy_fk_to_sessions(pool: &SqlitePool) -> AnyhowResult<bool> {
    let checkpoint_fks = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
        .fetch_all(pool)
        .await
        .context("Failed to inspect checkpoints foreign keys")?;

    Ok(checkpoint_fks.iter().any(|row| {
        let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
        table_name == "sessions"
    }))
}

async fn migrate_legacy_checkpoint_fk(pool: &SqlitePool) -> AnyhowResult<()> {
    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin checkpoint schema migration")?;

    sqlx::query("ALTER TABLE checkpoints RENAME TO checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to rename legacy checkpoints table")?;

    sqlx::query(
        "CREATE TABLE checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            checkpoint_id TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to create migrated checkpoints table")?;

    sqlx::query(
        "INSERT INTO checkpoints (id, checkpoint_id, description, created_at)
         SELECT id, checkpoint_id, description, created_at FROM checkpoints_old",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to copy checkpoint rows during migration")?;

    sqlx::query("DROP TABLE checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to drop legacy checkpoints table")?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id ON checkpoint_sessions(checkpoint_id)")
        .execute(&mut *tx)
        .await
        .context("Failed to ensure checkpoint index")?;

    tx.commit()
        .await
        .context("Failed to commit checkpoint schema migration")
}

/// Add backup columns to `checkpoint_sessions` table (migration v2 → v3)
async fn migrate_add_backup_columns(pool: &SqlitePool) -> AnyhowResult<()> {
    // SQLite doesn't support ALTER TABLE ADD COLUMN IF NOT EXISTS, so check first
    let columns: Vec<String> = sqlx::query("PRAGMA table_info(checkpoint_sessions)")
        .fetch_all(pool)
        .await
        .context("Failed to inspect checkpoint_sessions columns")?
        .iter()
        .filter_map(|row| {
            row.try_get::<String, _>("name")
                .ok()
                .map(|name| name.to_lowercase())
        })
        .collect();

    if !columns.contains(&"backup_path".to_lowercase()) {
        sqlx::query("ALTER TABLE checkpoint_sessions ADD COLUMN backup_path TEXT")
            .execute(pool)
            .await
            .context("Failed to add backup_path column")?;
    }

    if !columns.contains(&"backup_size".to_lowercase()) {
        sqlx::query("ALTER TABLE checkpoint_sessions ADD COLUMN backup_size INTEGER")
            .execute(pool)
            .await
            .context("Failed to add backup_size column")?;
    }

    Ok(())
}

/// List sessions from the sessions table
async fn list_sessions(pool: &SqlitePool) -> AnyhowResult<Vec<Session>> {
    let rows = sqlx::query(
        "SELECT name, status, workspace_path, branch, metadata, created_at, updated_at FROM sessions"
    )
    .fetch_all(pool)
    .await
    .context("Failed to list sessions")?;

    let sessions = futures::stream::iter(rows)
        .map(Ok::<sqlx::sqlite::SqliteRow, anyhow::Error>)
        .and_then(|row| async move {
            let name: String = row.try_get("name").context("Missing name")?;
            let status_str: String = row.try_get("status").context("Missing status")?;
            let workspace_path: String = row
                .try_get("workspace_path")
                .context("Missing workspace_path")?;
            let branch: Option<String> = row.try_get("branch").context("Missing branch")?;
            let metadata: Option<String> = row.try_get("metadata").context("Missing metadata")?;
            let created_at: i64 = row.try_get("created_at").context("Missing created_at")?;
            let updated_at: i64 = row.try_get("updated_at").context("Missing updated_at")?;

            let status = SessionStatus::from_str(&status_str)
                .map_err(|e| anyhow::anyhow!("Invalid status '{}': {}", status_str, e))?;

            let metadata_json: Option<serde_json::Value> =
                metadata.as_ref().and_then(|m| serde_json::from_str(m).ok());

            let created_at = u64::try_from(created_at)
                .map_err(|_| anyhow::anyhow!("created_at timestamp out of range: {created_at}"))?;
            let updated_at = u64::try_from(updated_at)
                .map_err(|_| anyhow::anyhow!("updated_at timestamp out of range: {updated_at}"))?;

            Ok(Session {
                id: None,
                name,
                status,
                state: WorkspaceState::Created,
                workspace_path,
                branch,
                created_at,
                updated_at,
                last_synced: None,
                metadata: metadata_json,
            })
        })
        .try_collect()
        .await?;

    Ok(sessions)
}

async fn create_checkpoint(
    pool: &SqlitePool,
    description: Option<&str>,
) -> AnyhowResult<CheckpointResponse> {
    let backup_dir = checkpoint_backup_dir();
    create_checkpoint_with_backup_dir(pool, description, &backup_dir).await
}

async fn create_checkpoint_with_backup_dir(
    pool: &SqlitePool,
    description: Option<&str>,
    backup_dir: &Path,
) -> AnyhowResult<CheckpointResponse> {
    let sessions = list_sessions(pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let checkpoint_id = generate_checkpoint_id()?;

    // Ensure backup directory exists
    fs::create_dir_all(backup_dir).context("Failed to create checkpoint backup directory")?;

    sqlx::query("INSERT INTO checkpoints (checkpoint_id, description) VALUES (?, ?)")
        .bind(&checkpoint_id)
        .bind(description)
        .execute(pool)
        .await
        .context("Failed to insert checkpoint")?;

    futures::stream::iter(sessions)
        .map(Ok::<Session, anyhow::Error>)
        .try_for_each(|session| {
            let pool = pool.clone();
            let checkpoint_id = checkpoint_id.clone();
            let backup_dir = backup_dir.to_path_buf();
            async move {
                let metadata_json = session
                    .metadata
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .context("Failed to serialize session metadata")?;

                // Backup workspace directory to tarball (may skip if metadata-only)
                let backup_result = backup_workspace(
                    &session.name,
                    Path::new(&session.workspace_path),
                    &checkpoint_id,
                    &backup_dir,
                )
                .await
                .context("Failed to backup workspace")?;

                let (backup_path, backup_size) = match backup_result {
                    Some((path, size)) => (Some(path), size),
                    None => (None, 0),
                };

                let backup_path_ref = backup_path.as_deref();

                sqlx::query(
                    "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path, branch, metadata, backup_path, backup_size)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&checkpoint_id)
                .bind(&session.name)
                .bind(session.status.to_string())
                .bind(&session.workspace_path)
                .bind(&session.branch)
                .bind(&metadata_json)
                .bind(backup_path_ref)
                .bind(i64::try_from(backup_size).unwrap_or(i64::MAX))
                .execute(&pool)
                .await
                .context("Failed to insert checkpoint session")?;
                Ok::<(), anyhow::Error>(())
            }
        })
        .await?;

    let metadata_only_sessions: Vec<String> = sqlx::query(
        "SELECT session_name FROM checkpoint_sessions WHERE checkpoint_id = ? AND backup_path IS NULL",
    )
    .bind(&checkpoint_id)
    .fetch_all(pool)
    .await
    .context("Failed to query metadata-only checkpoint sessions")?
    .into_iter()
    .filter_map(|row| row.try_get::<String, _>("session_name").ok())
    .collect();

    Ok(CheckpointResponse::Created {
        checkpoint_id,
        metadata_only: metadata_only_sessions,
    })
}

#[allow(clippy::too_many_lines)]
async fn restore_checkpoint(
    pool: &SqlitePool,
    checkpoint_id: &str,
) -> AnyhowResult<CheckpointResponse> {
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Verify checkpoint exists
    let exists: bool = sqlx::query("SELECT 1 FROM checkpoints WHERE checkpoint_id = ?")
        .bind(checkpoint_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to query checkpoint")?
        .is_some();

    if !exists {
        anyhow::bail!("Checkpoint '{checkpoint_id}' not found");
    }

    // Fetch saved sessions with backup info
    let rows = sqlx::query(
        "SELECT session_name, status, workspace_path, branch, metadata, backup_path
         FROM checkpoint_sessions WHERE checkpoint_id = ?",
    )
    .bind(checkpoint_id)
    .fetch_all(&mut *tx)
    .await
    .context("Failed to fetch checkpoint sessions")?;

    let invalid_names = rows.iter().try_fold(Vec::new(), |mut names, row| {
        let name: String = row
            .try_get("session_name")
            .context("Missing session_name while validating checkpoint rows")?;
        if validate_session_name(&name).is_err() {
            names.push(name);
        }
        Ok::<Vec<String>, anyhow::Error>(names)
    })?;

    let duplicate_names = rows
        .iter()
        .try_fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut seen, mut duplicates), row| {
                let name: String = row
                    .try_get("session_name")
                    .context("Missing session_name while checking duplicate checkpoint rows")?;
                if !seen.insert(name.clone()) {
                    duplicates.insert(name);
                }
                Ok::<(BTreeSet<String>, BTreeSet<String>), anyhow::Error>((seen, duplicates))
            },
        )?
        .1
        .into_iter()
        .collect::<Vec<_>>();

    let invalid_statuses = rows.iter().try_fold(Vec::new(), |mut statuses, row| {
        let status: String = row
            .try_get("status")
            .context("Missing status while validating checkpoint rows")?;
        if SessionStatus::from_str(&status).is_err() {
            statuses.push(status);
        }
        Ok::<Vec<String>, anyhow::Error>(statuses)
    })?;

    if !invalid_names.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains invalid session names; restore aborted: {}",
            checkpoint_id,
            invalid_names.join(", ")
        );
    }

    if !duplicate_names.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains duplicate session names; restore aborted: {}",
            checkpoint_id,
            duplicate_names.join(", ")
        );
    }

    if !invalid_statuses.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains invalid statuses; restore aborted: {}",
            checkpoint_id,
            invalid_statuses.join(", ")
        );
    }

    // Track statistics for reporting
    let (mut total_sessions, mut restored_count) = (0usize, 0usize);

    for row in rows {
        total_sessions += 1;

        let name: String = row
            .try_get("session_name")
            .context("Missing session_name")?;
        let status: String = row.try_get("status").context("Missing status")?;
        let workspace_path: String = row
            .try_get("workspace_path")
            .context("Missing workspace_path")?;
        let branch: Option<String> = row.try_get("branch").context("Missing branch")?;
        let metadata: Option<String> = row.try_get("metadata").context("Missing metadata")?;
        let backup_path: Option<String> =
            row.try_get("backup_path").context("Missing backup_path")?;

        // Check if we have a backup file for this session
        let has_backup = backup_path
            .as_ref()
            .map(|p| {
                Path::new(p)
                    .try_exists()
                    .map_err(|e| anyhow::anyhow!("Failed to check backup existence '{p}': {e}"))
            })
            .transpose()?
            .unwrap_or(false);

        if !has_backup {
            return Err(anyhow::anyhow!(
                "Session '{name}' has no backup file available. Cannot restore workspace data."
            ));
        }

        // Restore workspace from tarball
        restore_workspace(
            &name,
            Path::new(&workspace_path),
            Path::new(
                backup_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Missing backup_path for session '{name}'"))?,
            ),
        )
        .await
        .with_context(|| format!("Failed to restore workspace for session '{name}'"))?;

        // Restore database record
        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, branch, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))
             ON CONFLICT(name) DO UPDATE SET
                status = excluded.status,
                workspace_path = excluded.workspace_path,
                branch = excluded.branch,
                metadata = excluded.metadata,
                updated_at = strftime('%s', 'now')",
        )
        .bind(&name)
        .bind(&status)
        .bind(&workspace_path)
        .bind(&branch)
        .bind(&metadata)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to restore session record '{name}'"))?;
        restored_count += 1;
    }

    tx.commit()
        .await
        .context("Failed to commit restore transaction")?;

    debug_assert_eq!(restored_count, total_sessions);

    Ok(CheckpointResponse::Restored {
        checkpoint_id: checkpoint_id.to_string(),
    })
}

async fn list_checkpoints(pool: &SqlitePool) -> AnyhowResult<CheckpointResponse> {
    let rows = sqlx::query(
        "SELECT checkpoint_id, created_at, description
         FROM checkpoints ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list checkpoints")?;

    let checkpoints = futures::stream::iter(rows)
        .map(Ok::<sqlx::sqlite::SqliteRow, anyhow::Error>)
        .and_then(|row| {
            let pool = pool.clone();
            async move {
                let id: String = row
                    .try_get("checkpoint_id")
                    .context("Missing checkpoint_id")?;
                let created_at_ts: i64 = row.try_get("created_at").context("Missing created_at")?;
                let description: Option<String> =
                    row.try_get("description").context("Missing description")?;

                // Query actual session count from checkpoint_sessions table
                let count_row: Option<(i64,)> = sqlx::query_as(
                    "SELECT COUNT(*) FROM checkpoint_sessions WHERE checkpoint_id = ?",
                )
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .context("Failed to count checkpoint sessions")?;

                let session_count = count_row.map_or(0, |(c,)| c);
                let count = usize::try_from(session_count)
                    .map_err(|_| anyhow::anyhow!("Session count out of range: {session_count}"))?;

                let created_at = chrono::Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                    .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {created_at_ts}"))?;

                Ok(CheckpointInfo {
                    id,
                    created_at,
                    session_count: count,
                    description,
                })
            }
        })
        .try_collect::<Vec<_>>()
        .await?;

    Ok(CheckpointResponse::List { checkpoints })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn generate_checkpoint_id() -> AnyhowResult<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("System time before UNIX epoch")?
        .as_millis();
    Ok(format!("chk-{now:x}"))
}

/// Backup a workspace directory to a tarball
///
/// This function creates a compressed tar archive of the workspace directory,
/// storing it in the checkpoint backup directory. The archive includes all
/// files needed to restore the JJ repository state.
async fn backup_workspace(
    session_name: &str,
    workspace_path: &Path,
    checkpoint_id: &str,
    backup_dir: &Path,
) -> AnyhowResult<Option<(String, u64)>> {
    let limit = checkpoint_max_backup_size_bytes();
    if limit == 0 {
        warn!(
            "Checkpoint backups are disabled via Isolate_CHECKPOINT_MAX_BACKUP_BYTES=0, recording metadata only"
        );
        return Ok(None);
    }

    let session_name = session_name.to_string();
    let workspace_path = workspace_path.to_path_buf();
    let checkpoint_id = checkpoint_id.to_string();
    let backup_dir = backup_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let workspace_path = workspace_path
            .canonicalize()
            .context("Failed to canonicalize workspace path")?;

        if !workspace_path.exists() {
            return Err(anyhow::anyhow!(
                "Workspace path does not exist: {}",
                workspace_path.display()
            ));
        }

        // Generate backup filename
        let backup_filename = format!("{checkpoint_id}-{session_name}.tar.gz");
        let backup_path = backup_dir.join(&backup_filename);

        // Create tarball
        let backup_file = File::create(&backup_path)
            .with_context(|| format!("Failed to create backup file: {}", backup_path.display()))?;

        let gz_encoder = flate2::write::GzEncoder::new(backup_file, flate2::Compression::default());
        let mut tar_builder = Builder::new(gz_encoder);

        // Add workspace contents to tarball
        tar_builder
            .append_dir_all(".", &workspace_path)
            .with_context(|| {
                format!("Failed to archive workspace: {}", workspace_path.display())
            })?;

        // Finish the tarball (this flushes the GzEncoder too)
        let gz_encoder = tar_builder
            .into_inner()
            .context("Failed to finalize tarball builder")?;

        gz_encoder
            .finish()
            .context("Failed to finalize gzip compression")?;

        // Verify backup size is within limits
        let backup_size = fs::metadata(&backup_path)
            .map(|m| m.len())
            .context("Failed to get backup file size")?;

        if backup_size > limit {
            fs::remove_file(&backup_path).with_context(|| {
                format!(
                    "Failed to remove oversized backup: {}",
                    backup_path.display()
                )
            })?;
            warn!(
                "Workspace backup for {session_name} exceeded limit ({limit}); recording metadata only"
            );
            return Ok(None);
        }

        Ok(Some((
            backup_path.to_string_lossy().to_string(),
            backup_size,
        )))
    })
    .await
    .context("Failed to join backup task")?
}

/// Restore a workspace directory from a tarball
///
/// This function extracts a compressed tar archive to restore the workspace
/// directory and JJ repository state.
async fn restore_workspace(
    _session_name: &str,
    workspace_path: &Path,
    backup_path: &Path,
) -> AnyhowResult<()> {
    let workspace_path = workspace_path.to_path_buf();
    let backup_path = backup_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        if !backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file does not exist: {}",
                backup_path.display()
            ));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = workspace_path.parent() {
            let parent_display = parent.display().to_string();
            fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("Failed to create parent directory: {parent_display}: {e}")
            })?;
        }

        // Open and decompress the tarball
        let backup_path_display = backup_path.display().to_string();
        let backup_file = File::open(&backup_path).map_err(|e| {
            anyhow::anyhow!("Failed to open backup file: {backup_path_display}: {e}")
        })?;

        let gz_decoder = flate2::read::GzDecoder::new(backup_file);
        let mut tar_archive = tar::Archive::new(gz_decoder);

        // Extract to workspace path
        let workspace_path_display = workspace_path.display().to_string();
        tar_archive.unpack(&workspace_path).map_err(|e| {
            anyhow::anyhow!("Failed to extract backup to workspace: {workspace_path_display}: {e}")
        })?;

        Ok(())
    })
    .await
    .context("Failed to join restore task")?
}

fn output_response(response: &CheckpointResponse, format: OutputFormat) -> AnyhowResult<()> {
    if format.is_json() {
        let json = serde_json::to_string_pretty(response)
            .context("Failed to serialize checkpoint response")?;
        println!("{json}");
    } else {
        match response {
            CheckpointResponse::Created {
                checkpoint_id,
                metadata_only,
            } => {
                println!("Checkpoint created: {checkpoint_id}");
                if !metadata_only.is_empty() {
                    println!(
                        "Metadata-only snapshots recorded for {} session(s):",
                        metadata_only.len()
                    );
                    for session in metadata_only {
                        println!("  - {session}");
                    }
                }
            }
            CheckpointResponse::Restored { checkpoint_id } => {
                println!("Restored to checkpoint: {checkpoint_id}");
            }
            CheckpointResponse::List { checkpoints } => {
                if checkpoints.is_empty() {
                    println!("No checkpoints found.");
                } else {
                    println!(
                        "{:<20} {:<28} {:>8}  Description",
                        "ID", "Created", "Sessions"
                    );
                    println!("{}", "-".repeat(72));
                    for cp in checkpoints {
                        let desc = cp.description.as_deref().map_or("", |s| s);
                        println!(
                            "{:<20} {:<28} {:>8}  {}",
                            cp.id, cp.created_at, cp.session_count, desc
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_backup_dir(root: &Path) -> PathBuf {
        root.join("checkpoint_backups")
    }

    // ── Type Structure Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_action_create_with_description() {
        let action = CheckpointAction::Create {
            description: Some("test checkpoint".to_string()),
        };
        assert!(matches!(
            action,
            CheckpointAction::Create { description } if description == Some("test checkpoint".to_string())
        ));
    }

    #[test]
    fn test_checkpoint_action_create_without_description() {
        let action = CheckpointAction::Create { description: None };
        assert!(matches!(
            action,
            CheckpointAction::Create { description } if description.is_none()
        ));
    }

    #[test]
    fn test_checkpoint_action_restore() {
        let action = CheckpointAction::Restore {
            checkpoint_id: "chk-abc123".to_string(),
        };
        assert!(matches!(
            action,
            CheckpointAction::Restore { checkpoint_id } if checkpoint_id == "chk-abc123"
        ));
    }

    #[test]
    fn test_checkpoint_action_list() {
        let action = CheckpointAction::List;
        assert!(matches!(action, CheckpointAction::List));
    }

    // ── Response Serialization Tests ─────────────────────────────────────

    #[test]
    fn test_checkpoint_response_created_serialization() {
        let response = CheckpointResponse::Created {
            checkpoint_id: "chk-abc123".to_string(),
            metadata_only: vec!["session1".to_string()],
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("Created"));
        assert!(json_str.contains("chk-abc123"));
    }

    #[test]
    fn test_checkpoint_response_restored_serialization() {
        let response = CheckpointResponse::Restored {
            checkpoint_id: "chk-def456".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("Restored"));
        assert!(json_str.contains("chk-def456"));
    }

    #[test]
    fn test_checkpoint_response_list_empty_serialization() {
        let response = CheckpointResponse::List {
            checkpoints: vec![],
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("List"));
        assert!(json_str.contains("checkpoints"));
    }

    #[test]
    fn test_checkpoint_response_list_with_checkpoints_serialization() {
        let response = CheckpointResponse::List {
            checkpoints: vec![
                CheckpointInfo {
                    id: "chk-1".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    session_count: 3,
                    description: Some("first checkpoint".to_string()),
                },
                CheckpointInfo {
                    id: "chk-2".to_string(),
                    created_at: "2024-01-02T00:00:00Z".to_string(),
                    session_count: 5,
                    description: None,
                },
            ],
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("chk-1"));
        assert!(json_str.contains("chk-2"));
        assert!(json_str.contains("first checkpoint"));
        assert!(json_str.contains("\"session_count\":3"));
        assert!(json_str.contains("\"session_count\":5"));
    }

    // ── CheckpointInfo Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_info_with_description() {
        let info = CheckpointInfo {
            id: "chk-test".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 10,
            description: Some("Test description".to_string()),
        };
        assert_eq!(info.id, "chk-test");
        assert_eq!(info.created_at, "2024-06-15T10:30:00Z");
        assert_eq!(info.session_count, 10);
        assert_eq!(info.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_checkpoint_info_without_description() {
        let info = CheckpointInfo {
            id: "chk-test2".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 0,
            description: None,
        };
        assert!(info.description.is_none());
        assert_eq!(info.session_count, 0);
    }

    #[test]
    fn test_checkpoint_info_clone() {
        let info = CheckpointInfo {
            id: "chk-orig".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            session_count: 5,
            description: Some("original".to_string()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.created_at, info.created_at);
        assert_eq!(cloned.session_count, info.session_count);
        assert_eq!(cloned.description, info.description);
    }

    // ── CheckpointArgs Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_args_create() {
        let args = CheckpointArgs {
            action: CheckpointAction::Create {
                description: Some("new checkpoint".to_string()),
            },
            format: OutputFormat::Json,
        };
        assert!(matches!(args.action, CheckpointAction::Create { .. }));
        assert!(args.format.is_json());
    }

    #[test]
    fn test_checkpoint_args_list_json() {
        let args = CheckpointArgs {
            action: CheckpointAction::List,
            format: OutputFormat::Json,
        };
        assert!(matches!(args.action, CheckpointAction::List));
        assert!(args.format.is_json());
    }

    // ── Helper Function Tests ────────────────────────────────────────────

    #[test]
    fn test_generate_checkpoint_id_format() {
        let id_result = generate_checkpoint_id();
        assert!(id_result.is_ok(), "id generation should succeed");
        let Ok(id) = id_result else { return };
        assert!(id.starts_with("chk-"));
        // Should be a valid hex string after the prefix
        let hex_part = &id[4..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_checkpoint_id_uniqueness() {
        let Ok(id1) = generate_checkpoint_id() else {
            return;
        };
        // Sleep briefly to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));
        let Ok(id2) = generate_checkpoint_id() else {
            return;
        };
        // IDs should be different (based on timestamp)
        assert_ne!(id1, id2);
    }

    // ── Schema Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_checkpoint_schema_contains_required_tables() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoint_sessions"));
    }

    #[test]
    fn test_checkpoint_schema_has_foreign_key() {
        assert!(CHECKPOINT_SCHEMA.contains("FOREIGN KEY (checkpoint_id)"));
        assert!(CHECKPOINT_SCHEMA.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_checkpoint_schema_has_index() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id"));
    }

    #[test]
    fn test_checkpoint_schema_has_version_table() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoint_schema_meta"));
    }

    #[test]
    fn test_checkpoint_schema_checkpoints_columns() {
        assert!(CHECKPOINT_SCHEMA.contains("checkpoint_id TEXT UNIQUE NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("description TEXT"));
        assert!(CHECKPOINT_SCHEMA.contains("created_at INTEGER NOT NULL"));
    }

    #[test]
    fn test_checkpoint_schema_sessions_columns() {
        assert!(CHECKPOINT_SCHEMA.contains("session_name TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("status TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("workspace_path TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("branch TEXT"));
        assert!(CHECKPOINT_SCHEMA.contains("metadata TEXT"));
    }

    // ── Session Name Validation Tests ─────────────────────────────────

    /// Create a fake but valid tar.gz backup for testing
    fn create_fake_backup(backup_path: &Path, content_dir: &Path) -> std::io::Result<()> {
        use std::fs::File;

        use tar::Builder;

        let backup_file = File::create(backup_path)?;
        let gz_encoder = flate2::write::GzEncoder::new(backup_file, flate2::Compression::default());
        let mut tar_builder = Builder::new(gz_encoder);

        // Add a dummy file to make it a valid tarball
        if content_dir.exists() {
            tar_builder.append_dir_all(".", content_dir)?;
        }

        let gz_encoder = tar_builder.into_inner()?;
        gz_encoder.finish()?;
        Ok(())
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_valid_session_names() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with valid session names
        let checkpoint_id = "chk-test-valid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        // Create temp workspace dirs
        let workspace1 = dir.path().join("my-session");
        let workspace2 = dir.path().join("feature-auth");
        let workspace3 = dir.path().join("test_123");

        std::fs::create_dir_all(&workspace1).expect("Failed to create workspace1");
        std::fs::create_dir_all(&workspace2).expect("Failed to create workspace2");
        std::fs::create_dir_all(&workspace3).expect("Failed to create workspace3");

        // Insert sessions with valid names and create fake backup files
        let valid_sessions = vec![
            (
                "my-session",
                "active",
                workspace1.to_string_lossy().to_string(),
            ),
            (
                "feature-auth",
                "active",
                workspace2.to_string_lossy().to_string(),
            ),
            (
                "test_123",
                "paused",
                workspace3.to_string_lossy().to_string(),
            ),
        ];

        for (name, status, path) in &valid_sessions {
            // Create fake backup file
            let backup_path = dir.path().join(format!("{checkpoint_id}-{name}.tar.gz"));
            let workspace_path = Path::new(path);
            create_fake_backup(&backup_path, workspace_path).expect("Failed to create fake backup");

            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path, backup_path)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .bind(backup_path.to_string_lossy().as_ref())
            .execute(&pool)
            .await
            .expect("Failed to insert session");
        }

        // Restore the checkpoint
        let result = restore_checkpoint(&pool, checkpoint_id).await;

        // Should succeed
        assert!(
            result.is_ok(),
            "Restore with valid names should succeed: {:?}",
            result.err()
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_invalid_session_names_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with both valid and invalid session names
        let checkpoint_id = "chk-test-invalid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        // Insert a mix of valid and invalid session names
        let test_cases = vec![
            ("valid-session", "active", "/path/to/valid"), // Valid
            ("", "active", "/path/to/empty"),              // Invalid: empty
            ("  ", "active", "/path/to/space"),            // Invalid: whitespace
            ("../../etc/passwd", "active", "/path/to/traverse"), // Invalid: path traversal
            ("feature-auth", "active", "/path/to/feature"), // Valid
            ("\t", "active", "/path/to/tab"),              // Invalid: tab
            ("123-starts-with-digit", "active", "/path/to/digit"), // Invalid: starts with digit
        ];

        for (name, status, path) in test_cases {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .execute(&pool)
            .await
            .expect("Failed to insert session");
        }

        // Seed an existing valid session; restore must not delete it on validation failure
        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
        )
        .bind("existing-safe")
        .bind("active")
        .bind("/path/to/existing")
        .execute(&pool)
        .await
        .expect("Failed to seed existing session");

        // Restore the checkpoint
        let result = restore_checkpoint(&pool, checkpoint_id).await;

        // Should fail before deleting current sessions
        assert!(
            result.is_err(),
            "Restore should fail with invalid checkpoint names"
        );

        // Verify existing sessions remain untouched
        let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM sessions")
            .fetch_all(&pool)
            .await
            .expect("Failed to list sessions");
        assert_eq!(
            rows.len(),
            1,
            "Restore failure should not wipe existing sessions"
        );
        assert_eq!(rows[0].0, "existing-safe");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_only_invalid_names_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with only invalid session names
        let checkpoint_id = "chk-test-all-invalid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        // Insert only invalid session names
        let invalid_sessions = vec![
            ("", "active", "/path/to/empty"),
            ("  ", "active", "/path/to/space"),
            ("../../etc/passwd", "active", "/path/to/traverse"),
        ];

        for (name, status, path) in invalid_sessions {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .execute(&pool)
            .await
            .expect("Failed to insert session");
        }

        // Seed an existing valid session; restore must not delete it on validation failure
        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
        )
        .bind("existing-safe")
        .bind("active")
        .bind("/path/to/existing")
        .execute(&pool)
        .await
        .expect("Failed to seed existing session");

        // Restore the checkpoint
        let result = restore_checkpoint(&pool, checkpoint_id).await;

        // Should fail before deleting current sessions
        assert!(
            result.is_err(),
            "Restore should fail when checkpoint contains only invalid names"
        );

        // Verify existing sessions remain untouched
        let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM sessions")
            .fetch_all(&pool)
            .await
            .expect("Failed to list sessions");
        assert_eq!(
            rows.len(),
            1,
            "Restore failure should not wipe existing sessions"
        );
        assert_eq!(rows[0].0, "existing-safe");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_restore_checkpoint_duplicate_names_rolls_back_without_data_loss() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
        )
        .bind("existing-safe")
        .bind("active")
        .bind("/path/to/existing")
        .execute(&pool)
        .await
        .expect("Failed to seed existing session");

        let checkpoint_id = "chk-test-duplicate-rollback";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        for path in ["/path/to/one", "/path/to/two"] {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind("dup-session")
            .bind("active")
            .bind(path)
            .execute(&pool)
            .await
            .expect("Failed to insert duplicate checkpoint session rows");
        }

        let result = restore_checkpoint(&pool, checkpoint_id).await;
        assert!(
            result.is_err(),
            "Restore should fail on duplicate session inserts"
        );
        let error_message = result.err().map_or_else(String::new, |err| err.to_string());
        assert!(
            error_message.contains("duplicate session names"),
            "Restore should fail with duplicate preflight validation error"
        );

        let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM sessions")
            .fetch_all(&pool)
            .await
            .expect("Failed to list sessions");
        assert_eq!(rows.len(), 1, "Rollback must preserve pre-restore sessions");
        assert_eq!(rows[0].0, "existing-safe");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_restore_checkpoint_invalid_status_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
        )
        .bind("existing-safe")
        .bind("active")
        .bind("/path/to/existing")
        .execute(&pool)
        .await
        .expect("Failed to seed existing session");

        let checkpoint_id = "chk-test-invalid-status";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
             VALUES (?, ?, ?, ?)",
        )
        .bind(checkpoint_id)
        .bind("restored-session")
        .bind("bad-status")
        .bind("/path/to/restored")
        .execute(&pool)
        .await
        .expect("Failed to insert invalid checkpoint status");

        let result = restore_checkpoint(&pool, checkpoint_id).await;
        assert!(result.is_err(), "Restore should fail on invalid status");

        let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM sessions")
            .fetch_all(&pool)
            .await
            .expect("Failed to list sessions");
        assert_eq!(rows.len(), 1, "Existing session must remain untouched");
        assert_eq!(rows[0].0, "existing-safe");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_legacy_checkpoint_schema_migration_removes_session_fk() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        sqlx::query("DROP TABLE IF EXISTS checkpoint_sessions")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoint_sessions");
        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoints");

        sqlx::query(
            "CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                session_id INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create legacy checkpoints table");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to run checkpoint schema ensure/migration");

        let fk_rows = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
            .fetch_all(&pool)
            .await
            .expect("Failed to read checkpoint foreign keys");

        let has_session_fk = fk_rows.iter().any(|row| {
            let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
            table_name == "sessions"
        });

        assert!(
            !has_session_fk,
            "Legacy FK to sessions should be removed by migration"
        );

        let version: i64 = sqlx::query("SELECT version FROM checkpoint_schema_meta WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("Failed to read checkpoint schema version")
            .try_get("version")
            .expect("Failed to parse checkpoint schema version");

        assert_eq!(
            version, CHECKPOINT_SCHEMA_VERSION,
            "Checkpoint schema version should be updated after migration"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_checkpoint_schema_migration_is_idempotent_across_repeated_runs() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        sqlx::query("DROP TABLE IF EXISTS checkpoint_sessions")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoint_sessions");
        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoints");

        sqlx::query(
            "CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                session_id INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create legacy checkpoints table");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("First migration run should succeed");
        ensure_checkpoint_tables(&pool)
            .await
            .expect("Second migration run should be idempotent");

        let fk_rows = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
            .fetch_all(&pool)
            .await
            .expect("Failed to read checkpoint foreign keys");

        let has_session_fk = fk_rows.iter().any(|row| {
            let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
            table_name == "sessions"
        });
        assert!(
            !has_session_fk,
            "session FK should stay removed after re-run"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_checkpoint_schema_recovery_from_interrupted_rename_copy_drop_window() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoints");
        sqlx::query("DROP TABLE IF EXISTS checkpoints_old")
            .execute(&pool)
            .await
            .expect("Failed to drop checkpoints_old");

        sqlx::query(
            "CREATE TABLE checkpoints_old (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create checkpoints_old");

        sqlx::query(
            "INSERT INTO checkpoints_old (checkpoint_id, description, created_at)
             VALUES ('chk-interrupted', 'from-old', strftime('%s', 'now'))",
        )
        .execute(&pool)
        .await
        .expect("Failed to seed checkpoints_old");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Recovery should succeed for interrupted migration window");

        let restored: Option<(String,)> = sqlx::query_as(
            "SELECT description FROM checkpoints WHERE checkpoint_id = 'chk-interrupted'",
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to query restored checkpoint");
        assert_eq!(
            restored.map(|row| row.0),
            Some("from-old".to_string()),
            "Interrupted migration data should be replayed into checkpoints"
        );

        let old_exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'checkpoints_old' LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to check checkpoints_old presence");
        assert!(
            old_exists.is_none(),
            "checkpoints_old should be dropped after recovery"
        );

        pool.close().await;
    }

    // ── Backup/Restore Integration Tests ───────────────────────────────────

    #[tokio::test]
    async fn test_checkpoint_create_backs_up_workspace_data() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a test session with real workspace
        let workspace = dir.path().join("test-workspace");
        std::fs::create_dir_all(&workspace).expect("Failed to create workspace");

        // Create some test files in workspace
        let test_file = workspace.join("test.txt");
        std::fs::write(&test_file, "Hello, World!").expect("Failed to write test file");

        let jj_dir = workspace.join(".jj");
        std::fs::create_dir_all(&jj_dir).expect("Failed to create .jj dir");

        let repo_file = jj_dir.join("repo.toml");
        std::fs::write(&repo_file, "# JJ repo config").expect("Failed to write repo file");

        // Create session in database
        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
        )
        .bind("test-session")
        .bind("active")
        .bind(workspace.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .expect("Failed to create session");

        // Create checkpoint
        let backup_dir = test_backup_dir(dir.path());
        let response =
            create_checkpoint_with_backup_dir(&pool, Some("Test checkpoint"), &backup_dir)
                .await
                .expect("Failed to create checkpoint");

        let checkpoint_id = match response {
            CheckpointResponse::Created { checkpoint_id, .. } => checkpoint_id,
            _ => panic!("Expected Created response"),
        };

        // Verify backup file exists
        let backup_path = backup_dir.join(format!("{checkpoint_id}-test-session.tar.gz"));

        assert!(
            backup_path.exists(),
            "Backup file should exist at {}",
            backup_path.display()
        );

        // Verify backup is not empty
        let metadata = std::fs::metadata(&backup_path).expect("Failed to get backup metadata");
        assert!(metadata.len() > 0, "Backup should not be empty");

        // Clean up workspace to simulate data loss
        std::fs::remove_dir_all(&workspace).expect("Failed to remove workspace");

        assert!(!workspace.exists(), "Workspace should be deleted");

        // Restore from checkpoint
        let response = restore_checkpoint(&pool, &checkpoint_id)
            .await
            .expect("Failed to restore checkpoint");

        match response {
            CheckpointResponse::Restored { .. } => {}
            _ => panic!("Expected Restored response"),
        }

        // Verify workspace was restored
        assert!(workspace.exists(), "Workspace should be restored");

        let restored_file = workspace.join("test.txt");
        assert!(restored_file.exists(), "Test file should be restored");

        let content =
            std::fs::read_to_string(&restored_file).expect("Failed to read restored file");
        assert_eq!(content, "Hello, World!", "File content should match");

        let restored_repo = workspace.join(".jj/repo.toml");
        assert!(restored_repo.exists(), "JJ repo file should be restored");

        let _ = std::fs::remove_file(&backup_path);
        pool.close().await;
    }

    #[tokio::test]
    async fn test_checkpoint_restore_fails_without_backup() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        let checkpoint_id = "chk-no-backup";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint");

        // Insert session without backup_path (simulates old checkpoint)
        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
             VALUES (?, ?, ?, ?)",
        )
        .bind(checkpoint_id)
        .bind("test-session")
        .bind("active")
        .bind("/path/to/workspace")
        .execute(&pool)
        .await
        .expect("Failed to insert session");

        // Restore should fail with clear error message
        let result = restore_checkpoint(&pool, checkpoint_id).await;

        assert!(result.is_err(), "Restore should fail without backup file");

        let err = result.err().unwrap();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("no backup file available"),
            "Error should mention missing backup: {}",
            err_msg
        );

        pool.close().await;
    }

    // ── Additional Tests for hardline port ───────────────────────────────

    #[tokio::test]
    async fn test_list_checkpoints_empty() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        let response = list_checkpoints(&pool)
            .await
            .expect("list_checkpoints should succeed");

        match response {
            CheckpointResponse::List { checkpoints } => {
                assert!(checkpoints.is_empty(), "Should have no checkpoints");
            }
            _ => panic!("Expected List response"),
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_list_checkpoints_with_data() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create database pool");

        ensure_checkpoint_tables(&pool)
            .await
            .expect("Failed to create checkpoint tables");

        // Create some checkpoints
        sqlx::query("INSERT INTO checkpoints (checkpoint_id, description) VALUES (?, ?)")
            .bind("chk-1")
            .bind("First checkpoint")
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint 1");

        sqlx::query("INSERT INTO checkpoints (checkpoint_id, description) VALUES (?, ?)")
            .bind("chk-2")
            .bind("Second checkpoint")
            .execute(&pool)
            .await
            .expect("Failed to create checkpoint 2");

        // Add sessions to first checkpoint
        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
             VALUES (?, ?, ?, ?)",
        )
        .bind("chk-1")
        .bind("session-a")
        .bind("active")
        .bind("/path/to/a")
        .execute(&pool)
        .await
        .expect("Failed to add session to checkpoint 1");

        let response = list_checkpoints(&pool)
            .await
            .expect("list_checkpoints should succeed");

        match response {
            CheckpointResponse::List { checkpoints } => {
                assert_eq!(checkpoints.len(), 2, "Should have 2 checkpoints");
                // Most recent first
                assert_eq!(checkpoints[0].id, "chk-2");
                assert_eq!(checkpoints[1].id, "chk-1");
                assert_eq!(checkpoints[1].session_count, 1);
            }
            _ => panic!("Expected List response"),
        }

        pool.close().await;
    }
}
