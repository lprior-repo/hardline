//! Lock management CLI implementation.

use scp_core::coordination::locks::manager::LockManager;
use scp_core::infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService};
use scp_core::Result;
use std::env;
use std::path::PathBuf;
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
pub(crate) fn acquire_with_path(
    session: &str,
    agent: &str,
    ttl: Option<u64>,
    db_path: &str,
) -> Result<()> {
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr
            .lock_with_ttl(session, agent, ttl.map_or(0, |v| v))
            .await?;
        println!(
            "Lock acquired: {} for agent {} (expires: {})",
            res.session, res.agent_id, res.expires_at
        );
        Ok(())
    })
}

/// Release a lock on a session
pub fn release(session: &str, agent: &str) -> Result<()> {
    release_with_path(session, agent, &get_db_path())
}

/// Internal release with explicit path for testing
pub(crate) fn release_with_path(session: &str, agent: &str, db_path: &str) -> Result<()> {
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        mgr.unlock(session, agent).await?;
        println!("Lock released: {}", session);
        Ok(())
    })
}

/// Send a heartbeat for a lock
pub fn heartbeat(session: &str, agent: &str) -> Result<()> {
    heartbeat_with_path(session, agent, &get_db_path())
}

/// Internal heartbeat with explicit path for testing
pub(crate) fn heartbeat_with_path(session: &str, agent: &str, db_path: &str) -> Result<()> {
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr.heartbeat(session, agent).await?;
        println!(
            "Heartbeat sent: {} (new expiration: {})",
            session, res.expires_at
        );
        Ok(())
    })
}

/// Get the status of a lock
pub fn status(session: &str) -> Result<()> {
    status_with_path(session, &get_db_path())
}

/// Internal status with explicit path for testing
pub(crate) fn status_with_path(session: &str, db_path: &str) -> Result<()> {
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let res = mgr.get_lock_state(session).await?;
        match (res.holder, res.expires_at) {
            (Some(agent), Some(expires_at)) => {
                println!(
                    "Locked: session {} held by {} (expires: {})",
                    session, agent, expires_at
                );
            }
            (Some(agent), None) => {
                println!(
                    "Locked: session {} held by {} (no expiration)",
                    session, agent
                );
            }
            _ => println!("Unlocked: session {}", session),
        }
        Ok(())
    })
}

/// List all active locks
pub fn list() -> Result<()> {
    list_with_path(&get_db_path())
}

/// Internal list with explicit path for testing
pub(crate) fn list_with_path(db_path: &str) -> Result<()> {
    run_async(async {
        let config = DatabaseConfig::new(db_path.to_string())?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let mgr = LockManager::new(db_service.pool().clone());
        mgr.init().await?;

        let locks = mgr.get_all_locks().await?;
        if locks.is_empty() {
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
