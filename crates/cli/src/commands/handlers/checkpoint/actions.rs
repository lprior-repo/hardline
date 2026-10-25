//! Action functions for the checkpoint command handler (Tier 3).
//!
//! I/O operations that create, restore, and list session checkpoints.

use std::path::Path;

use scp_core::{
    infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService},
    Error, Result,
};
use scp_session::infrastructure::{
    sqlite_session_repository::SessionRow, SessionRepository, SqliteSessionRepository,
};
use tokio::runtime::Runtime;

use super::data::{
    generate_checkpoint_id, CheckpointAction, CheckpointInfo, CheckpointOptions, CheckpointOutput,
};
use crate::commands::handlers::config_ports::resolve_state_db_path;

/// Resolve the state database path for checkpoint storage.
///
/// Follows the same convention as `config_ports`: `SCP_STATE_DB`, then
/// `SCP_DATABASE_PATH`, then `<cwd>/.scp/state.db`. Checkpoints live in the
/// same database file as the session rows they snapshot.
fn resolve_db_path() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    resolve_state_db_path(&cwd)
}

/// Open the database service at `db_path`, creating the parent directory
/// when needed (SQLite cannot create missing parent directories itself).
async fn open_service(db_path: &Path) -> Result<SqliteDatabaseService> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io_error(format!("Failed to create {}: {e}", parent.display())))?;
        }
    }
    let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
    SqliteDatabaseService::new(config).await
}

/// Create the checkpoint tables alongside the session tables.
///
/// The session rows themselves are read and written through
/// `SqliteSessionRepository`; these two tables only hold checkpoint
/// metadata and per-session snapshots, so no second session store exists.
async fn init_checkpoint_schema(db: &SqliteDatabaseService) -> Result<()> {
    db.execute(
        r#"CREATE TABLE IF NOT EXISTS checkpoints (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            session_count INTEGER NOT NULL,
            description TEXT
        )"#,
    )
    .await?;
    db.execute(
        r#"CREATE TABLE IF NOT EXISTS checkpoint_sessions (
            checkpoint_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            name TEXT NOT NULL,
            workspace TEXT,
            bead TEXT,
            branch_state TEXT NOT NULL,
            branch_name TEXT,
            session_state TEXT NOT NULL,
            last_synced TEXT,
            created_at TEXT NOT NULL,
            PRIMARY KEY (checkpoint_id, session_id)
        )"#,
    )
    .await?;
    Ok(())
}

/// Open the session repository and the checkpoint tables on one database.
///
/// Returns the repository plus a second service handle sharing the same
/// connection pool, so checkpoint rows and session rows stay in one file.
async fn open_checkpoint_db(
    db_path: &Path,
) -> Result<(SqliteSessionRepository, SqliteDatabaseService)> {
    let service = open_service(db_path).await?;
    let pool = service.pool().clone();
    let repo = SqliteSessionRepository::new(service);
    repo.init_schema()
        .await
        .map_err(|e| Error::database(e.to_string()))?;
    let checkpoint_db = SqliteDatabaseService::from_pool(pool);
    init_checkpoint_schema(&checkpoint_db).await?;
    Ok((repo, checkpoint_db))
}

/// Run an async checkpoint operation on a fresh runtime.
///
/// The checkpoint entry points are synchronous (like the lock commands),
/// so each operation bridges to the async repository this way.
fn block_on<T>(f: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    let rt = Runtime::new()
        .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
    rt.block_on(f)
}

/// Execute the checkpoint command with the given options.
///
/// # Errors
///
/// Returns an error if:
/// - Restoring a checkpoint that does not exist
/// - The checkpoint ID is empty for a restore action
/// - Any I/O or database operation fails
pub fn run_checkpoint(options: &CheckpointOptions) -> Result<()> {
    let db_path = resolve_db_path()?;
    let output = run_checkpoint_with_db(options, &db_path)?;
    output_checkpoint(&output);
    Ok(())
}

/// Execute the checkpoint command against an explicit database file.
///
/// The public entry point resolves the database path; this variant exists
/// so tests can point at a scratch database instead of the working tree's.
fn run_checkpoint_with_db(
    options: &CheckpointOptions,
    db_path: &Path,
) -> Result<CheckpointOutput> {
    match &options.action {
        CheckpointAction::Create { description } => {
            run_create_with_db(description.as_deref(), db_path)
        }
        CheckpointAction::Restore { checkpoint_id } => {
            run_restore_with_db(checkpoint_id, db_path)
        }
        CheckpointAction::List => run_list_with_db(db_path),
    }
}

/// Create a new checkpoint of all session state.
///
/// Enumerates every session in the session database and stores a
/// metadata-only snapshot of each session row (no worktree files are
/// copied, which is why every snapshotted session is reported in
/// `metadata_only`).
///
/// # Errors
///
/// Returns an error if checkpoint creation fails.
fn run_create_with_db(description: Option<&str>, db_path: &Path) -> Result<CheckpointOutput> {
    block_on(async {
        let (repo, db) = open_checkpoint_db(db_path).await?;
        // Rows, not domain sessions: the domain type always reads back as
        // `Created`, which would flatten every stored state in the snapshot.
        let rows: Vec<SessionRow> = repo
            .list_rows()
            .await
            .map_err(|e| Error::database(e.to_string()))?;

        // Checkpoint IDs come from millisecond timestamps, so two creates
        // in the same millisecond could collide; regenerate on collision.
        let mut checkpoint_id = generate_checkpoint_id();
        for _ in 0..10 {
            let existing = db
                .query_params(
                    "SELECT id FROM checkpoints WHERE id = ?",
                    &[Some(checkpoint_id.clone())],
                )
                .await?;
            if existing.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            checkpoint_id = generate_checkpoint_id();
        }

        let created_at = chrono::Utc::now().to_rfc3339();
        db.execute_params(
            "INSERT INTO checkpoints (id, created_at, session_count, description) VALUES (?, ?, ?, ?)",
            &[
                Some(checkpoint_id.clone()),
                Some(created_at),
                Some(rows.len().to_string()),
                description.map(String::from),
            ],
        )
        .await?;
        for row in &rows {
            db.execute_params(
                r#"INSERT INTO checkpoint_sessions
                    (checkpoint_id, session_id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                &[
                    Some(checkpoint_id.clone()),
                    Some(row.id.clone()),
                    Some(row.name.clone()),
                    row.workspace.clone(),
                    row.bead.clone(),
                    Some(row.branch_state.clone()),
                    row.branch_name.clone(),
                    Some(row.session_state.clone()),
                    row.last_synced.clone(),
                    Some(row.created_at.clone()),
                ],
            )
            .await?;
        }

        Ok(CheckpointOutput::Created {
            checkpoint_id,
            metadata_only: rows.iter().map(|r| r.name.clone()).collect(),
            description: description.map(String::from),
        })
    })
}

/// Restore session state from a checkpoint.
///
/// The checkpoint must exist; its snapshot rows are upserted back into
/// the `sessions` table, so sessions deleted since the checkpoint was
/// taken come back. Sessions created after the checkpoint are untouched.
///
/// # Errors
///
/// Returns an error if the checkpoint ID is empty, malformed, or not
/// found.
fn run_restore_with_db(checkpoint_id: &str, db_path: &Path) -> Result<CheckpointOutput> {
    if checkpoint_id.is_empty() {
        return Err(Error::validation_error("Checkpoint ID cannot be empty"));
    }

    if !checkpoint_id.starts_with("chk-") {
        return Err(Error::validation_error(format!(
            "Invalid checkpoint ID format: '{checkpoint_id}' (must start with 'chk-')"
        )));
    }

    block_on(async {
        let (_, db) = open_checkpoint_db(db_path).await?;
        let found = db
            .query_params(
                "SELECT id FROM checkpoints WHERE id = ?",
                &[Some(checkpoint_id.to_string())],
            )
            .await?;
        if found.is_empty() {
            return Err(Error::not_found(format!(
                "Checkpoint not found: {checkpoint_id}"
            )));
        }

        let snapshots = db
            .query_params(
                r#"SELECT session_id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at
                   FROM checkpoint_sessions WHERE checkpoint_id = ?"#,
                &[Some(checkpoint_id.to_string())],
            )
            .await?;
        for cols in &snapshots {
            let opt = |i: usize| -> Option<String> {
                cols.get(i).filter(|s| !s.is_empty()).cloned()
            };
            db.execute_params(
                r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                       name = excluded.name,
                       workspace = excluded.workspace,
                       bead = excluded.bead,
                       branch_state = excluded.branch_state,
                       branch_name = excluded.branch_name,
                       session_state = excluded.session_state,
                       last_synced = excluded.last_synced,
                       created_at = excluded.created_at"#,
                &[
                    opt(0),
                    opt(1),
                    opt(2),
                    opt(3),
                    opt(4),
                    opt(5),
                    opt(6),
                    opt(7),
                    opt(8),
                ],
            )
            .await?;
        }

        Ok(CheckpointOutput::Restored {
            checkpoint_id: checkpoint_id.to_string(),
        })
    })
}

/// List all available checkpoints from the database.
///
/// # Errors
///
/// Returns an error if listing fails due to I/O or database errors.
fn run_list_with_db(db_path: &Path) -> Result<CheckpointOutput> {
    block_on(async {
        let (_, db) = open_checkpoint_db(db_path).await?;
        let rows = db
            .query(
                "SELECT id, created_at, session_count, description FROM checkpoints ORDER BY created_at DESC",
            )
            .await?;
        let mut checkpoints = Vec::with_capacity(rows.len());
        for cols in &rows {
            let count = cols
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_default();
            let text = |i: usize| -> String {
                match cols.get(i) {
                    Some(s) => s.clone(),
                    None => String::new(),
                }
            };
            checkpoints.push(CheckpointInfo {
                id: text(0),
                created_at: text(1),
                session_count: count,
                description: cols.get(3).filter(|s| !s.is_empty()).cloned(),
            });
        }
        Ok(CheckpointOutput::List { checkpoints })
    })
}

/// Output a checkpoint result to the user.
fn output_checkpoint(output: &CheckpointOutput) {
    match output {
        CheckpointOutput::Created {
            checkpoint_id,
            metadata_only,
            description,
        } => {
            output_created(checkpoint_id, metadata_only, description);
        }
        CheckpointOutput::Restored { checkpoint_id } => {
            scp_core::output::Output::info(&format!("Restored to checkpoint: {checkpoint_id}"));
        }
        CheckpointOutput::List { checkpoints } => {
            output_checkpoint_list(checkpoints);
        }
    }
}

/// Display a created checkpoint with optional metadata.
fn output_created(id: &str, metadata_only: &[String], description: &Option<String>) {
    scp_core::output::Output::info(&format!("Checkpoint created: {id}"));
    if let Some(desc) = description {
        scp_core::output::Output::info(&format!("  Description: {desc}"));
    }
    if !metadata_only.is_empty() {
        scp_core::output::Output::info(&format!(
            "Metadata-only snapshots recorded for {} session(s):",
            metadata_only.len()
        ));
        for session in metadata_only {
            scp_core::output::Output::info(&format!("  - {session}"));
        }
    }
}

/// Display the checkpoint list, either empty message or tabular format.
fn output_checkpoint_list(checkpoints: &[CheckpointInfo]) {
    if checkpoints.is_empty() {
        scp_core::output::Output::info("No checkpoints found.");
    } else {
        scp_core::output::Output::info(&format!(
            "{:<20} {:<28} {:>8}  Description",
            "ID", "Created", "Sessions"
        ));
        scp_core::output::Output::info(&"-".repeat(72));
        checkpoints.iter().for_each(|cp| {
            let desc = cp.description.as_deref().map_or("", |d| d);
            scp_core::output::Output::info(&format!(
                "{:<20} {:<28} {:>8}  {}",
                cp.id, cp.created_at, cp.session_count, desc
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::checkpoint::data::OutputFormat;

    fn create_options(action: CheckpointAction) -> CheckpointOptions {
        CheckpointOptions {
            action,
            format: OutputFormat::Json,
        }
    }

    /// A scratch database file per test, so tests never touch the working
    /// tree's `.scp/state.db` and never share state with each other.
    fn scratch_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("state.db");
        (dir, path)
    }

    /// Seed one session row through the real session repository.
    fn seed_session(db_path: &Path, name: &str) {
        block_on(async {
            let (repo, _) = open_checkpoint_db(db_path).await.expect("open db");
            let session = scp_session::domain::entities::Session::create(
                scp_session::domain::value_objects::SessionName::parse(name)
                    .expect("valid name"),
            )
            .expect("create session");
            repo.save(&session).await.expect("save session");
            Ok::<(), Error>(())
        })
        .expect("seed session");
    }

    /// Seed a session row in the Failed state directly. The repository's
    /// `save` only accepts `Created` sessions, so a failed row is inserted
    /// with bind parameters instead.
    fn seed_failed_session(db_path: &Path, name: &str) {
        block_on(async {
            let (repo, db) = open_checkpoint_db(db_path).await.expect("open db");
            drop(repo);
            db.execute_params(
                r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                &[
                    Some(format!("session-{name}")),
                    Some(name.to_string()),
                    None,
                    None,
                    Some("Detached".to_string()),
                    None,
                    Some("Failed".to_string()),
                    None,
                    Some("2024-01-01T00:00:00+00:00".to_string()),
                ],
            )
            .await
            .expect("insert failed session");
            Ok::<(), Error>(())
        })
        .expect("seed failed session");
    }

    #[test]
    fn run_checkpoint_create_without_description() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::Create { description: None });
        assert!(run_checkpoint_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_checkpoint_create_with_description() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::Create {
            description: Some("test checkpoint".to_string()),
        });
        assert!(run_checkpoint_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_checkpoint_restore_valid_id() {
        let (_dir, db) = scratch_db();
        let created = run_create_with_db(None, &db).expect("create");
        let id = match created {
            CheckpointOutput::Created { checkpoint_id, .. } => checkpoint_id,
            _ => panic!("Expected Created variant"),
        };
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: id,
        });
        assert!(run_checkpoint_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_checkpoint_restore_unknown_id_fails() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: "chk-abc123".to_string(),
        });
        let result = run_checkpoint_with_db(&options, &db);
        assert!(result.is_err(), "Unknown checkpoint ID should fail");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("not found"),
            "Error should mention not found: {msg}"
        );
    }

    #[test]
    fn run_checkpoint_restore_empty_id_fails() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: String::new(),
        });
        let result = run_checkpoint_with_db(&options, &db);
        assert!(result.is_err(), "Empty checkpoint ID should fail");
        let err = result.err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("cannot be empty"),
            "Error should mention empty ID: {msg}"
        );
    }

    #[test]
    fn run_checkpoint_restore_invalid_prefix_fails() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: "bad-prefix-123".to_string(),
        });
        let result = run_checkpoint_with_db(&options, &db);
        assert!(result.is_err(), "Invalid prefix should fail");
        let err = result.err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("must start with 'chk-'"),
            "Error should mention prefix requirement: {msg}"
        );
    }

    #[test]
    fn run_checkpoint_list() {
        let (_dir, db) = scratch_db();
        let options = create_options(CheckpointAction::List);
        assert!(run_checkpoint_with_db(&options, &db).is_ok());
    }

    #[test]
    fn checkpoint_roundtrip_restores_deleted_session() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "roundtrip-session");
        let created = run_create_with_db(Some("before delete"), &db).expect("create");
        let (id, metadata_only) = match created {
            CheckpointOutput::Created {
                checkpoint_id,
                metadata_only,
                ..
            } => (checkpoint_id, metadata_only),
            _ => panic!("Expected Created variant"),
        };
        assert_eq!(metadata_only, vec!["roundtrip-session".to_string()]);

        let listed = run_list_with_db(&db).expect("list");
        match listed {
            CheckpointOutput::List { checkpoints } => {
                assert_eq!(checkpoints.len(), 1);
                assert_eq!(checkpoints[0].id, id);
                assert_eq!(checkpoints[0].session_count, 1);
                assert_eq!(
                    checkpoints[0].description,
                    Some("before delete".to_string())
                );
            }
            _ => panic!("Expected List variant"),
        }

        // Delete the session, then restore: it must come back.
        block_on(async {
            let (repo, _) = open_checkpoint_db(&db).await.expect("open db");
            let sessions = repo.list().await.expect("list sessions");
            assert_eq!(sessions.len(), 1);
            repo.delete(sessions[0].id.as_str())
                .await
                .expect("delete session");
            assert!(repo.list().await.expect("list again").is_empty());
            Ok::<(), Error>(())
        })
        .expect("delete session check");

        run_restore_with_db(&id, &db).expect("restore");
        block_on(async {
            let (repo, _) = open_checkpoint_db(&db).await.expect("open db");
            let sessions = repo.list().await.expect("list sessions");
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].name.as_str(), "roundtrip-session");
            Ok::<(), Error>(())
        })
        .expect("restore check");
    }

    // -- output_checkpoint tests --

    #[test]
    fn output_checkpoint_created_no_metadata() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-test123".to_string(),
            metadata_only: vec![],
            description: None,
        };
        output_checkpoint(&output);
    }

    #[test]
    fn output_checkpoint_created_with_metadata() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-test456".to_string(),
            metadata_only: vec!["session-a".to_string(), "session-b".to_string()],
            description: Some("desc".to_string()),
        };
        output_checkpoint(&output);
    }

    #[test]
    fn output_checkpoint_created_with_description() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-desc".to_string(),
            metadata_only: vec![],
            description: Some("my description".to_string()),
        };
        output_checkpoint(&output);
    }

    #[test]
    fn output_checkpoint_restored() {
        let output = CheckpointOutput::Restored {
            checkpoint_id: "chk-restore".to_string(),
        };
        output_checkpoint(&output);
    }

    #[test]
    fn output_checkpoint_list_empty() {
        let output = CheckpointOutput::List {
            checkpoints: vec![],
        };
        output_checkpoint(&output);
    }

    #[test]
    fn output_checkpoint_list_with_entries() {
        let output = CheckpointOutput::List {
            checkpoints: vec![
                CheckpointInfo {
                    id: "chk-1".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    session_count: 3,
                    description: Some("first".to_string()),
                },
                CheckpointInfo {
                    id: "chk-2".to_string(),
                    created_at: "2024-01-02T00:00:00Z".to_string(),
                    session_count: 1,
                    description: None,
                },
            ],
        };
        output_checkpoint(&output);
    }

    // -- run_create / run_restore / run_list direct tests --

    #[test]
    fn run_create_without_description_succeeds() {
        let (_dir, db) = scratch_db();
        assert!(run_create_with_db(None, &db).is_ok());
    }

    #[test]
    fn run_create_with_description_succeeds() {
        let (_dir, db) = scratch_db();
        assert!(run_create_with_db(Some("my desc"), &db).is_ok());
    }

    #[test]
    fn run_create_output_contains_description() {
        let (_dir, db) = scratch_db();
        let output = run_create_with_db(Some("test desc"), &db).expect("should succeed");
        match output {
            CheckpointOutput::Created { description, .. } => {
                assert_eq!(description, Some("test desc".to_string()));
            }
            _ => panic!("Expected Created variant"),
        }
    }

    #[test]
    fn run_restore_valid_succeeds() {
        let (_dir, db) = scratch_db();
        let created = run_create_with_db(None, &db).expect("create");
        let id = match created {
            CheckpointOutput::Created { checkpoint_id, .. } => checkpoint_id,
            _ => panic!("Expected Created variant"),
        };
        assert!(run_restore_with_db(&id, &db).is_ok());
    }

    #[test]
    fn run_restore_empty_fails() {
        let (_dir, db) = scratch_db();
        assert!(run_restore_with_db("", &db).is_err());
    }

    #[test]
    fn run_restore_bad_prefix_fails() {
        let (_dir, db) = scratch_db();
        assert!(run_restore_with_db("invalid-id", &db).is_err());
    }

    #[test]
    fn run_restore_unknown_fails() {
        let (_dir, db) = scratch_db();
        assert!(run_restore_with_db("chk-unknown", &db).is_err());
    }

    #[test]
    fn run_list_succeeds() {
        let (_dir, db) = scratch_db();
        assert!(run_list_with_db(&db).is_ok());
    }

    #[test]
    fn checkpoint_preserves_failed_state_through_restore() {
        let (_dir, db) = scratch_db();
        seed_failed_session(&db, "failed-session");
        let created = run_create_with_db(None, &db).expect("create");
        let id = match created {
            CheckpointOutput::Created { checkpoint_id, .. } => checkpoint_id,
            _ => panic!("Expected Created variant"),
        };
        // Delete the row, restore, and confirm the Failed state comes back
        // rather than flattening to Created.
        block_on(async {
            let (repo, _) = open_checkpoint_db(&db).await.expect("open db");
            repo.delete("session-failed-session")
                .await
                .expect("delete session");
            Ok::<(), Error>(())
        })
        .expect("delete check");
        run_restore_with_db(&id, &db).expect("restore");
        block_on(async {
            let (repo, _) = open_checkpoint_db(&db).await.expect("open db");
            let rows = repo.list_rows().await.expect("list rows");
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].name, "failed-session");
            assert_eq!(rows[0].session_state, "Failed");
            Ok::<(), Error>(())
        })
        .expect("restore check");
    }

    #[test]
    fn run_list_empty_on_fresh_db() {
        let (_dir, db) = scratch_db();
        match run_list_with_db(&db).expect("list") {
            CheckpointOutput::List { checkpoints } => {
                assert!(checkpoints.is_empty());
            }
            _ => panic!("Expected List variant"),
        }
    }
}
