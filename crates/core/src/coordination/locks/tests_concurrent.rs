//! Concurrent locking tests (isolate-ggji: Lock Race Condition).
use crate::coordination::locks::{LockManager, LockResponse};

use sqlx::sqlite::SqlitePoolOptions;

use crate::Error;

#[allow(dead_code)]
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::database(e.to_string()))
}

// Regression test: TOCTOU race in lock acquisition
#[tokio::test]
async fn regression_concurrent_lock_mutual_exclusion() -> Result<(), Error> {
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

    sqlx::query(
        "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
    )
    .bind("contended-session")
    .bind("active")
    .bind("working")
    .bind("/workspace")
    .execute(&pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;

    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let mgr = mgr.clone();
            tokio::spawn(
                async move { mgr.lock("contended-session", &format!("agent-{i}")).await },
            )
        })
        .collect();

    let results: Vec<std::result::Result<LockResponse, Error>> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|join_result| match join_result {
                Ok(inner_result) => inner_result,
                Err(join_err) => Err(Error::io_error(format!("Task join failed: {join_err}"))),
            })
            .collect();

    let successful_locks = results
        .iter()
        .filter(|r| std::result::Result::is_ok(r))
        .count();

    let failed_locks = results
        .iter()
        .filter(|r| matches!(r, Err(Error::Session(_))))
        .count();

    assert_eq!(
        successful_locks, 1,
        "Expected exactly 1 successful lock, got {successful_locks}"
    );

    assert_eq!(
        failed_locks, 9,
        "Expected 9 agents to receive SessionLocked, got {failed_locks}"
    );

    let locks = mgr.get_all_locks().await?;
    assert_eq!(
        locks.len(),
        1,
        "Expected exactly 1 lock in database, got {}",
        locks.len()
    );

    let lock_state = mgr.get_lock_state("contended-session").await?;
    assert!(
        lock_state.holder.is_some(),
        "Expected a lock holder to exist"
    );

    Ok(())
}

// Stress test: 100 concurrent lock attempts across 10 sessions
#[tokio::test]
async fn stress_test_concurrent_locks_multiple_sessions() -> Result<(), Error> {
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

    for i in 0..10 {
        sqlx::query(
            "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
        )
        .bind(format!("session-{i}"))
        .bind("active")
        .bind("working")
        .bind("/workspace")
        .execute(&pool)
        .await
        .map_err(|e| Error::database(e.to_string()))?;
    }

    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let mgr = mgr.clone();
            let session_id = i % 10;
            tokio::spawn(async move {
                mgr.lock(&format!("session-{session_id}"), &format!("agent-{i}"))
                    .await
            })
        })
        .collect();

    let results: Vec<std::result::Result<LockResponse, Error>> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|join_result| match join_result {
                Ok(inner_result) => inner_result,
                Err(join_err) => Err(Error::io_error(format!("Task join failed: {join_err}"))),
            })
            .collect();

    let successful_count = results.iter().filter(|r| r.is_ok()).count();

    assert_eq!(
        successful_count, 10,
        "Expected 10 successful locks (1 per session), got {successful_count}"
    );

    let locks = mgr.get_all_locks().await?;
    assert_eq!(
        locks.len(),
        10,
        "Expected 10 locks in database, got {}",
        locks.len()
    );

    Ok(())
}
