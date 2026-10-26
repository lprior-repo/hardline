//! Lock management CLI implementation.

use std::{env, path::PathBuf};

use scp_core::{
    coordination::locks::manager::LockManager,
    infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService},
    Output, Result,
};
use tokio::runtime::Runtime;

/// Get the database path from environment or default
fn get_db_path() -> String {
    env::var("SCP_DATABASE_PATH").unwrap_or_else(|_| {
        let mut path = env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        path.push(".scp");
        path.push("hardline.db");
        path.to_string_lossy().to_string()
    })
}

/// Ensure the parent directory of the database path exists, mirroring
/// queue's `get_queue()`. Without this, opening a database under a
/// missing directory fails (SQLite "unable to open database file")
/// instead of listing an empty store.
fn ensure_db_parent_dir(db_path: &str) -> Result<()> {
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                scp_core::Error::io_error(format!("Failed to create database directory: {e}"))
            })?;
        }
    }
    Ok(())
}

/// Helper to run async code in a temporary runtime
fn run_async<F, T>(f: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let rt = Runtime::new()
        .map_err(|e| scp_core::Error::internal(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(f)
}

/// Acquire a lock on a session
pub fn acquire(session: &str, agent: &str, ttl: Option<u64>) -> Result<()> {
    acquire_with_path(session, agent, ttl, &get_db_path())
}

/// Internal acquire with explicit path for testing
#[allow(clippy::map_or_identity)]
pub fn acquire_with_path(
    session: &str,
    agent: &str,
    ttl: Option<u64>,
    db_path: &str,
) -> Result<()> {
    ensure_db_parent_dir(db_path)?;
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr
            .lock_with_ttl(session, agent, ttl.map_or(0, |v| v))
            .await?;
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "session": res.session,
                "agent": res.agent_id,
                "result": "acquired",
                "lock_id": res.lock_id,
                "acquired_at": res.acquired_at.to_string(),
                "expires_at": res.expires_at.to_string(),
            }));
        } else {
            println!(
                "Lock acquired: {} for agent {} (expires: {})",
                res.session, res.agent_id, res.expires_at
            );
        }
        Ok(())
    })
}

/// Release a lock on a session
pub fn release(session: &str, agent: &str) -> Result<()> {
    release_with_path(session, agent, &get_db_path())
}

/// Internal release with explicit path for testing
pub fn release_with_path(session: &str, agent: &str, db_path: &str) -> Result<()> {
    ensure_db_parent_dir(db_path)?;
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        mgr.unlock(session, agent).await?;
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "session": session,
                "agent": agent,
                "result": "released",
            }));
        } else {
            println!("Lock released: {}", session);
        }
        Ok(())
    })
}

/// Send a heartbeat for a lock
pub fn heartbeat(session: &str, agent: &str) -> Result<()> {
    heartbeat_with_path(session, agent, &get_db_path())
}

/// Internal heartbeat with explicit path for testing
pub fn heartbeat_with_path(session: &str, agent: &str, db_path: &str) -> Result<()> {
    ensure_db_parent_dir(db_path)?;
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr.heartbeat(session, agent).await?;
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "session": session,
                "agent": agent,
                "result": "heartbeat",
                "expires_at": res.expires_at.to_string(),
            }));
        } else {
            println!(
                "Heartbeat sent: {} (new expiration: {})",
                session, res.expires_at
            );
        }
        Ok(())
    })
}

/// Get the status of a lock
pub fn status(session: &str) -> Result<()> {
    status_with_path(session, &get_db_path())
}

/// Internal status with explicit path for testing
pub fn status_with_path(session: &str, db_path: &str) -> Result<()> {
    ensure_db_parent_dir(db_path)?;
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr.get_lock_state(session).await?;
        match (res.holder, res.expires_at) {
            (Some(agent), Some(expires_at)) => {
                if Output::json_output() {
                    Output::json(&serde_json::json!({
                        "session": session,
                        "locked": true,
                        "agent": agent,
                        "expires_at": expires_at.to_string(),
                    }));
                } else {
                    println!(
                        "Locked: session {} held by {} (expires: {})",
                        session, agent, expires_at
                    );
                }
            }
            (Some(agent), None) => {
                if Output::json_output() {
                    Output::json(&serde_json::json!({
                        "session": session,
                        "locked": true,
                        "agent": agent,
                        "expires_at": serde_json::Value::Null,
                    }));
                } else {
                    println!(
                        "Locked: session {} held by {} (no expiration)",
                        session, agent
                    );
                }
            }
            _ => {
                if Output::json_output() {
                    Output::json(&serde_json::json!({
                        "session": session,
                        "locked": false,
                    }));
                } else {
                    println!("Unlocked: session {}", session);
                }
            }
        }
        Ok(())
    })
}

/// List all active locks
pub fn list() -> Result<()> {
    list_with_path(&get_db_path())
}

/// Internal list with explicit path for testing
pub fn list_with_path(db_path: &str) -> Result<()> {
    ensure_db_parent_dir(db_path)?;
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let locks = mgr.get_all_locks().await?;
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "locks": locks
                    .iter()
                    .map(|lock| serde_json::json!({
                        "session": lock.session,
                        "agent": lock.agent_id,
                        "lock_id": lock.lock_id,
                        "acquired_at": lock.acquired_at.to_string(),
                        "expires_at": lock.expires_at.to_string(),
                    }))
                    .collect::<Vec<_>>(),
            }));
        } else if locks.is_empty() {
            println!("No active locks");
        } else {
            println!("{:<20} {:<20} {:<25}", "SESSION", "AGENT", "EXPIRES");
            println!("{:-<65}", "");
            for lock in locks {
                println!(
                    "{:<20} {:<20} {:<25}",
                    lock.session, lock.agent_id, lock.expires_at
                );
            }
        }
        Ok(())
    })
}
