//! Session validation tests (isolate-1w0d: Lock Non-Existent Session).
use crate::coordination::locks::errors::LockErrorKind;
use crate::coordination::locks::LockManager;

use sqlx::sqlite::SqlitePoolOptions;

use crate::Error;

#[allow(dead_code)]
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::database(e.to_string()))
}

// Test: Lock non-existent session returns error (when sessions table exists)
#[tokio::test]
async fn lock_nonexistent_session_returns_not_found_error() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            name TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            state TEXT NOT NULL,
            workspace_path TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;

    let result = mgr.lock("ghost-session", "agent-1").await;

    assert!(result.is_err(), "Should fail for non-existent session");

    match &result {
        Err(Error::Lock(lk)) if matches!(lk.kind(), LockErrorKind::SessionNotFound { .. }) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains("ghost-session"),
                "Expected SessionNotFound with 'ghost-session', got: {msg}"
            );
        }
        other => panic!("Expected LockError(SessionNotFound), got: {other:?}"),
    }

    let locks = mgr.get_all_locks().await?;
    assert!(
        locks.is_empty(),
        "No lock should exist for non-existent session"
    );

    Ok(())
}

// Test: Lock existing session succeeds (requires creating session in database)
#[tokio::test]
async fn lock_existing_session_succeeds() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            name TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            state TEXT NOT NULL,
            workspace_path TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;

    sqlx::query("INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)")
        .bind("real-session")
        .bind("active")
        .bind("working")
        .bind("/workspace")
        .execute(&pool)
        .await
        .map_err(|e| Error::database(e.to_string()))?;

    let result = mgr.lock("real-session", "agent-1").await;

    assert!(result.is_ok(), "Lock should succeed for existing session");

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].session, "real-session");
    assert_eq!(locks[0].agent_id, "agent-1");

    Ok(())
}

// Test: Lock after session is deleted fails
#[tokio::test]
async fn lock_deleted_session_fails_with_not_found() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            name TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            state TEXT NOT NULL,
            workspace_path TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;

    sqlx::query("INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)")
        .bind("ephemeral-session")
        .bind("active")
        .bind("working")
        .bind("/workspace")
        .execute(&pool)
        .await
        .map_err(|e| Error::database(e.to_string()))?;

    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind("ephemeral-session")
        .execute(&pool)
        .await
        .map_err(|e| Error::database(e.to_string()))?;

    let result = mgr.lock("ephemeral-session", "agent-1").await;

    assert!(result.is_err());
    assert!(
        matches!(result, Err(Error::Lock(lk)) if matches!(lk.kind(), LockErrorKind::SessionNotFound { .. }))
    );

    Ok(())
}

// Regression: The exact reported bug - locking non-existent session no longer creates orphaned
// lock
#[tokio::test]
async fn regression_lock_nonexistent_session_no_longer_creates_orphaned_lock() -> Result<(), Error>
{
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            name TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            state TEXT NOT NULL,
            workspace_path TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;

    let result = mgr.lock("ghost-session", "agent-1").await;

    assert!(result.is_err(), "Lock must fail for non-existent session");

    let locks = mgr.get_all_locks().await?;
    assert!(
        !locks.iter().any(|l| l.session == "ghost-session"),
        "REGRESSION: Orphaned lock created for non-existent session!"
    );

    Ok(())
}

// Test: Session name with newline is rejected
#[tokio::test]
async fn lock_session_with_newline_rejected() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    let result = mgr.lock("session\nwith\nnewline", "agent-1").await;

    assert!(result.is_err(), "Session name with newline should be rejected");

    match &result {
        Err(Error::Lock(lk)) if matches!(lk.kind(), LockErrorKind::InvalidSessionName { .. }) => {}
        other => panic!("Expected LockError(InvalidSessionName), got: {other:?}"),
    }

    Ok(())
}

// Test: Session name with other control characters is rejected
#[tokio::test]
async fn lock_session_with_control_chars_rejected() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool.clone());
    mgr.init().await?;

    let result = mgr.lock("session\twith\ttab", "agent-1").await;

    assert!(result.is_err(), "Session name with tab should be rejected");

    match &result {
        Err(Error::Lock(lk)) if matches!(lk.kind(), LockErrorKind::InvalidSessionName { .. }) => {}
        other => panic!("Expected LockError(InvalidSessionName), got: {other:?}"),
    }

    Ok(())
}
