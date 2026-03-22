//! TTL-related regression tests.

use sqlx::sqlite::SqlitePoolOptions;

use crate::Error;

#[allow(dead_code)]
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::Database(e.to_string()))
}

#[tokio::test]
async fn regression_lock_with_ttl_maps_contention_race_to_session_locked() -> Result<(), Error> {
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
    .map_err(|e| Error::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
    )
    .bind("ttl-contended-session")
    .bind("active")
    .bind("working")
    .bind("/workspace")
    .execute(&pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;

    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let mgr = mgr.clone();
            tokio::spawn(async move {
                mgr.lock_with_ttl("ttl-contended-session", &format!("agent-{i}"), 60)
                    .await
            })
        })
        .collect();

    let results: Vec<std::result::Result<LockResponse, Error>> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|r| r.map_err(|e| Error::Internal(e.to_string())))
            .collect::<Result<Vec<_>>>()?;

    let successful_locks = results
        .iter()
        .filter(|r| std::result::Result::is_ok(r))
        .count();
    let session_locked_errors = results
        .iter()
        .filter(|r| matches!(r, Err(Error::SessionLocked(..))))
        .count();
    let database_errors = results
        .iter()
        .filter(|r| matches!(r, Err(Error::Database(_))))
        .count();

    assert_eq!(successful_locks, 1, "expected exactly 1 successful lock");
    assert_eq!(
        session_locked_errors, 9,
        "expected all losing attempts to map to SessionLocked"
    );
    assert_eq!(
        database_errors, 0,
        "contention should not leak DatabaseError"
    );

    Ok(())
}

#[tokio::test]
async fn regression_lock_with_ttl_fails_fast_before_session_validation() -> Result<(), Error> {
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
    .map_err(|e| Error::Database(e.to_string()))?;

    sqlx::query(
        "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
    )
    .bind("ordered-session")
    .bind("active")
    .bind("working")
    .bind("/workspace")
    .execute(&pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;

    let _lock = mgr.lock("ordered-session", "agent-a").await?;

    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind("ordered-session")
        .execute(&pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    let result = mgr.lock_with_ttl("ordered-session", "agent-b", 60).await;
    assert!(matches!(
        result,
        Err(Error::SessionLocked(session, holder))
        if session == "ordered-session" && holder == "agent-a"
    ));

    Ok(())
}
