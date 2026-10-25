//! Action functions for the query command handler (Tier 3).
//!
//! I/O operations that execute structured queries against the real session
//! database (the same file the checkpoint commands use).

use std::path::Path;

use scp_core::{
    infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService},
    output::Output,
    Error, Result,
};
use scp_session::infrastructure::{
    sqlite_session_repository::SessionRow, SqliteSessionRepository,
};
use tokio::runtime::Runtime;

use super::data::{QueryOptions, QueryOutput, QueryType, SessionInfo, SessionStatus};
use crate::commands::handlers::config_ports::resolve_state_db_path;

/// Resolve the state database path, following the same convention as
/// `config_ports`: `SCP_STATE_DB`, then `SCP_DATABASE_PATH`, then
/// `<cwd>/.scp/state.db`.
fn resolve_db_path() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    resolve_state_db_path(&cwd)
}

/// Open the session repository at `db_path`, creating the parent directory
/// and the `sessions` table when needed. An absent database therefore reads
/// as zero sessions rather than an error.
async fn open_repository(db_path: &Path) -> Result<SqliteSessionRepository> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io_error(format!("Failed to create {}: {e}", parent.display())))?;
        }
    }
    let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
    let service = SqliteDatabaseService::new(config).await?;
    let repo = SqliteSessionRepository::new(service);
    repo.init_schema()
        .await
        .map_err(|e| Error::database(e.to_string()))?;
    Ok(repo)
}

/// Run an async query operation on a fresh runtime.
///
/// The query entry points are synchronous (like the lock commands), so
/// each operation bridges to the async repository this way.
fn block_on<T>(f: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    let rt = Runtime::new()
        .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
    rt.block_on(f)
}

/// Map a stored session state string onto the query output's status
/// vocabulary.
///
/// The session store tracks finer states than queries expose: anything
/// that is not paused, completed, or failed reads as active. States are
/// read from the stored row, not from the domain `Session` value, because
/// that type always reads back in the `Created` typestate.
fn map_status(state: &str) -> SessionStatus {
    match state {
        "Paused" => SessionStatus::Paused,
        "Completed" => SessionStatus::Completed,
        "Failed" => SessionStatus::Aborted,
        _ => SessionStatus::Active,
    }
}

/// Convert a stored session row into the query output shape.
///
/// Sessions carry a workspace id, not a filesystem path, so
/// `workspace_path` holds that id when present. Sessions carry no agent,
/// so `agent` is always `None`: an `--agent` filter therefore matches
/// nothing, which is reported honestly rather than faked.
fn to_session_info(row: &SessionRow) -> SessionInfo {
    SessionInfo {
        name: row.name.clone(),
        status: map_status(&row.session_state),
        workspace_path: row.workspace.clone(),
        agent: None,
        created_at: Some(row.created_at.clone()),
    }
}

/// Load all stored session rows from the database.
fn load_rows(db_path: &Path) -> Result<Vec<SessionRow>> {
    block_on(async {
        let repo = open_repository(db_path).await?;
        let rows = repo
            .list_rows()
            .await
            .map_err(|e| Error::database(e.to_string()))?;
        Ok::<Vec<SessionRow>, Error>(rows)
    })
}

/// Execute the query command.
///
/// # Errors
///
/// Returns errors for invalid query types or query execution failures.
pub fn run_query(options: &QueryOptions) -> Result<()> {
    if options.query_type == QueryType::Help {
        return run_help();
    }
    let db_path = resolve_db_path()?;
    run_query_with_db(options, &db_path)
}

/// Execute the query command against an explicit database file.
///
/// The public entry point resolves the database path; this variant exists
/// so tests can point at a scratch database instead of the working tree's.
fn run_query_with_db(options: &QueryOptions, db_path: &Path) -> Result<()> {
    match options.query_type {
        QueryType::SessionExists => run_session_exists_with_db(options, db_path),
        QueryType::Sessions => run_sessions_with_db(options, db_path),
        QueryType::SessionInfo => run_session_info_with_db(options, db_path),
        QueryType::Blockers => run_blockers_with_db(db_path),
        QueryType::SessionCount => run_session_count_with_db(db_path),
        QueryType::Help => run_help(),
    }
}

fn run_session_exists_with_db(options: &QueryOptions, db_path: &Path) -> Result<()> {
    let name = options
        .argument
        .as_deref()
        .ok_or_else(|| Error::validation_error("Session name required for session-exists query"))?;
    let exists = load_rows(db_path)?.iter().any(|r| r.name == name);

    let output = QueryOutput {
        success: true,
        query_type: "session-exists".to_string(),
        data: serde_json::json!({
            "name": name,
            "exists": exists
        }),
    };

    if exists {
        Output::success(&format!("Session '{name}' exists"));
    } else {
        Output::info(&format!("Session '{name}' does not exist"));
    }

    let _ = output; // Available for JSON output when needed
    Ok(())
}

fn run_sessions_with_db(options: &QueryOptions, db_path: &Path) -> Result<()> {
    let sessions: Vec<SessionInfo> = load_rows(db_path)?
        .iter()
        .map(to_session_info)
        .collect();

    Output::info("Sessions query:");

    let mut filtered: Vec<&SessionInfo> = sessions.iter().collect();
    if let Some(status) = &options.status_filter {
        Output::info(&format!("  Filter: status={status}"));
        filtered.retain(|s| s.status.as_str() == status);
    }
    if let Some(agent) = &options.agent_filter {
        Output::info(&format!("  Filter: agent={agent}"));
        filtered.retain(|s| s.agent.as_deref() == Some(agent));
    }

    if filtered.is_empty() {
        Output::info("  No sessions found");
    } else {
        for session in filtered {
            Output::info(&format!(
                "  - {} ({})",
                session.name,
                session.status.as_str()
            ));
        }
    }
    Ok(())
}

fn run_session_info_with_db(options: &QueryOptions, db_path: &Path) -> Result<()> {
    let name = options
        .argument
        .as_deref()
        .ok_or_else(|| Error::validation_error("Session name required for session-info query"))?;
    let found = load_rows(db_path)?
        .iter()
        .find(|r| r.name == name)
        .map(to_session_info);

    let info = match found {
        Some(info) => info,
        None => return Err(Error::not_found(format!("Session not found: {name}"))),
    };
    Output::info(&format!("Session info for '{}':", info.name));
    Output::info(&format!("  Status: {}", info.status.as_str()));
    if let Some(workspace) = &info.workspace_path {
        Output::info(&format!("  Workspace: {workspace}"));
    }
    if let Some(created_at) = &info.created_at {
        Output::info(&format!("  Created: {created_at}"));
    }
    Ok(())
}

fn run_blockers_with_db(db_path: &Path) -> Result<()> {
    let blockers: Vec<SessionInfo> = load_rows(db_path)?
        .iter()
        .map(to_session_info)
        .filter(|s| s.status == SessionStatus::Aborted)
        .collect();

    Output::info("Blockers query:");
    if blockers.is_empty() {
        Output::info("  No blockers found");
    } else {
        for session in blockers {
            Output::info(&format!("  - {}", session.name));
        }
    }
    Ok(())
}

fn run_session_count_with_db(db_path: &Path) -> Result<()> {
    let count = load_rows(db_path)?.len();

    Output::info(&format!("Session count: {count}"));
    Ok(())
}

fn run_help() -> Result<()> {
    Output::info("Available query types:");
    for name in QueryType::all_names() {
        Output::info(&format!("  {name}"));
    }
    Output::info("\nUsage: scp query <type> [argument] [options]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_session::{
        domain::{entities::Session, value_objects::SessionName},
        infrastructure::SessionRepository,
    };

    /// A scratch database file per test, so tests never touch the working
    /// tree's `.scp/state.db` and never share state with each other.
    fn scratch_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("state.db");
        (dir, path)
    }

    fn options(
        query_type: QueryType,
        argument: Option<&str>,
        status_filter: Option<&str>,
        agent_filter: Option<&str>,
    ) -> QueryOptions {
        QueryOptions {
            query_type,
            argument: argument.map(String::from),
            status_filter: status_filter.map(String::from),
            agent_filter: agent_filter.map(String::from),
        }
    }

    /// Seed one session row through the real session repository.
    fn seed_session(db_path: &Path, name: &str) {
        block_on(async {
            let repo = open_repository(db_path).await.expect("open db");
            let session = Session::create(SessionName::parse(name).expect("valid name"))
                .expect("create session");
            repo.save(&session).await.expect("save session");
            Ok::<(), Error>(())
        })
        .expect("seed session");
    }

    /// Seed a session row in the Failed state directly. The repository's
    /// `save` only accepts `Created` sessions, so a failed row (the kind
    /// a real pipeline would have written before failing) is inserted
    /// with bind parameters instead.
    fn seed_failed_session(db_path: &Path, name: &str) {
        block_on(async {
            let repo = open_repository(db_path).await.expect("open db");
            drop(repo);
            let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())
                .expect("config");
            let db = SqliteDatabaseService::new(config).await.expect("service");
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
    fn run_query_help() {
        let options = options(QueryType::Help, None, None, None);
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_session_exists_no_arg() {
        let (_dir, db) = scratch_db();
        let options = options(QueryType::SessionExists, None, None, None);
        assert!(run_query_with_db(&options, &db).is_err());
    }

    #[test]
    fn run_query_session_exists_with_arg() {
        let (_dir, db) = scratch_db();
        let options = options(
            QueryType::SessionExists,
            Some("nonexistent-session"),
            None,
            None,
        );
        // A missing session is a successful "does not exist" answer.
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_session_exists_after_seed() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "present-session");
        let options = options(
            QueryType::SessionExists,
            Some("present-session"),
            None,
            None,
        );
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_sessions() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "listed-session");
        let options = options(QueryType::Sessions, None, Some("active"), None);
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_sessions_status_filter_excludes() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "listed-session");
        // Seeded sessions read as active, so a completed filter matches none
        // but must still succeed rather than error.
        let options = options(QueryType::Sessions, None, Some("completed"), None);
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_session_count() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "counted-session");
        let options = options(QueryType::SessionCount, None, None, None);
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_session_count_empty_db() {
        let (_dir, db) = scratch_db();
        let options = options(QueryType::SessionCount, None, None, None);
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_blockers() {
        let (_dir, db) = scratch_db();
        seed_failed_session(&db, "failed-session");
        seed_session(&db, "healthy-session");
        let options = options(QueryType::Blockers, None, None, None);
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_session_info_no_arg() {
        let (_dir, db) = scratch_db();
        let options = options(QueryType::SessionInfo, None, None, None);
        assert!(run_query_with_db(&options, &db).is_err());
    }

    #[test]
    fn run_query_session_info_after_seed() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "detailed-session");
        let options = options(
            QueryType::SessionInfo,
            Some("detailed-session"),
            None,
            None,
        );
        assert!(run_query_with_db(&options, &db).is_ok());
    }

    #[test]
    fn run_query_session_info_missing_errors() {
        let (_dir, db) = scratch_db();
        let options = options(
            QueryType::SessionInfo,
            Some("no-such-session"),
            None,
            None,
        );
        let result = run_query_with_db(&options, &db);
        assert!(result.is_err(), "Missing session info should fail");
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("not found"), "unexpected error: {msg}");
    }

    #[test]
    fn stored_states_surface_through_rows() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "healthy-session");
        seed_failed_session(&db, "failed-session");
        let rows = load_rows(&db).expect("load rows");
        assert_eq!(rows.len(), 2);
        let infos: Vec<SessionInfo> = rows.iter().map(to_session_info).collect();
        let healthy = infos
            .iter()
            .find(|s| s.name == "healthy-session")
            .expect("healthy row");
        assert_eq!(healthy.status, SessionStatus::Active);
        let failed = infos
            .iter()
            .find(|s| s.name == "failed-session")
            .expect("failed row");
        assert_eq!(failed.status, SessionStatus::Aborted);
        let blockers: Vec<&SessionInfo> = infos
            .iter()
            .filter(|s| s.status == SessionStatus::Aborted)
            .collect();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].name, "failed-session");
    }

    #[test]
    fn session_count_matches_seeded_rows() {
        let (_dir, db) = scratch_db();
        seed_session(&db, "one-session");
        seed_failed_session(&db, "two-session");
        assert_eq!(load_rows(&db).expect("load rows").len(), 2);
    }
}
