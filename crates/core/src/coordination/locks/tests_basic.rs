//! Tests for the lock manager.

use sqlx::sqlite::SqlitePoolOptions;

use crate::Error;
use crate::coordination::locks::{LockManager, LockOperation};
use chrono::Duration;

#[allow(dead_code)]
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::database(e.to_string()))
}

#[allow(dead_code)]
async fn setup() -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

#[allow(dead_code)]
async fn setup_with_ttl(secs: i64) -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::with_ttl(pool, Duration::seconds(secs));
    mgr.init().await?;
    Ok(mgr)
}

// EARS 1: WHEN lock(session, agent_id) called, acquire exclusive lock within 50ms
#[tokio::test]
async fn test_lock_acquire_success() -> Result<(), Error> {
    let mgr = setup().await?;
    let start = std::time::Instant::now();
    let resp = mgr.lock("session-1", "agent-a").await?;
    let elapsed = start.elapsed();

    assert_eq!(resp.session, "session-1");
    assert_eq!(resp.agent_id, "agent-a");
    assert!(
        elapsed.as_millis() < 50,
        "Lock acquisition took {elapsed:?}"
    );
    Ok(())
}

// EARS 2: WHEN lock held by another agent, return SESSION_LOCKED error with holder info
#[tokio::test]
async fn test_lock_contention_returns_session_locked() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let result = mgr.lock("session-1", "agent-b").await;

    assert!(result.is_err());
    let err = result
        .err()
        .ok_or_else(|| Error::Internal("expected error".into()))?;
    assert!(matches!(
        &err,
        Error::SessionLocked(session, holder)
        if session == "session-1" && holder == "agent-a"
    ));
    Ok(())
}

// EARS 3: WHEN unlock(session, agent_id) called by holder, release lock
#[tokio::test]
async fn test_unlock_by_holder_succeeds() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    mgr.unlock("session-1", "agent-a").await?;

    let resp = mgr.lock("session-1", "agent-b").await?;
    assert_eq!(resp.agent_id, "agent-b");
    Ok(())
}

// EARS 4: WHEN unlock called by non-holder, return NOT_LOCK_HOLDER error
#[tokio::test]
async fn test_unlock_by_non_holder_fails() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let result = mgr.unlock("session-1", "agent-b").await;

    assert!(result.is_err());
    let err = result
        .err()
        .ok_or_else(|| Error::Internal("expected error".into()))?;
    assert!(matches!(
        &err,
        Error::NotLockHolder(session, agent_id)
        if session == "session-1" && agent_id == "agent-b"
    ));
    Ok(())
}

// EARS 5: WHEN lock TTL expires, auto-release lock
#[tokio::test]
async fn test_expired_lock_allows_new_acquisition() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let resp = mgr.lock("session-1", "agent-b").await?;
    assert_eq!(resp.agent_id, "agent-b");
    Ok(())
}

// EARS 6: WHEN get_all_locks() called, return all active locks with expiry times
#[tokio::test]
async fn test_get_all_locks_returns_active() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let _ = mgr.lock("session-2", "agent-b").await?;

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 2);

    let sessions: Vec<&str> = locks.iter().map(|l| l.session.as_str()).collect();
    assert!(sessions.contains(&"session-1"));
    assert!(sessions.contains(&"session-2"));
    Ok(())
}

// EARS 6 cont: expired locks should NOT appear
#[tokio::test]
async fn test_get_all_locks_excludes_expired() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let locks = mgr.get_all_locks().await?;
    assert!(locks.is_empty());
    Ok(())
}

// EARS 7: WHEN agent heartbeats, extend lock TTL
#[tokio::test]
async fn test_heartbeat_extends_ttl() -> Result<(), Error> {
    let mgr = setup_with_ttl(2).await?;
    let lock_resp = mgr.lock("session-1", "agent-a").await?;
    let original_expires = lock_resp.expires_at;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let hb = mgr.heartbeat("session-1", "agent-a").await?;
    assert!(hb.expires_at > original_expires);
    Ok(())
}

// Heartbeat by non-holder should fail
#[tokio::test]
async fn test_heartbeat_by_non_holder_fails() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let result = mgr.heartbeat("session-1", "agent-b").await;
    assert!(result.is_err());
    Ok(())
}

// Heartbeat on non-existent lock should fail
#[tokio::test]
async fn test_heartbeat_no_lock_fails() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.heartbeat("session-1", "agent-a").await;
    assert!(result.is_err());
    Ok(())
}

// Re-locking by same agent should succeed (idempotent)
#[tokio::test]
async fn test_relock_same_agent_idempotent() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let r2 = mgr.lock("session-1", "agent-a").await?;
    assert_eq!(r2.session, "session-1");
    Ok(())
}

// EARS: Double unlock MUST be logged as warning in audit trail
#[tokio::test]
async fn test_double_unlock_logs_warning() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    mgr.unlock("session-1", "agent-a").await?;

    let audit_log = mgr.get_lock_audit_log("session-1").await?;

    assert_eq!(
        audit_log.len(),
        2,
        "Expected 2 audit entries (lock + unlock)"
    );

    assert_eq!(audit_log[0].operation, LockOperation::Lock);
    assert_eq!(audit_log[0].agent_id, "agent-a");

    assert_eq!(audit_log[1].operation, LockOperation::Unlock);
    assert_eq!(audit_log[1].agent_id, "agent-a");

    mgr.unlock("session-1", "agent-a").await?;

    let audit_log2 = mgr.get_lock_audit_log("session-1").await?;
    assert_eq!(
        audit_log2.len(),
        3,
        "Expected 3 audit entries with double unlock warning"
    );

    assert_eq!(audit_log2[2].operation, LockOperation::DoubleUnlockWarning);

    Ok(())
}

// EARS: Lock state query MUST show current lock holder
#[tokio::test]
async fn test_lock_state_query_shows_holder() -> Result<(), Error> {
    let mgr = setup().await?;

    let state = mgr.get_lock_state("session-1").await?;
    assert!(state.holder.is_none(), "Expected no holder initially");

    let _ = mgr.lock("session-1", "agent-a").await?;
    let state = mgr.get_lock_state("session-1").await?;
    assert_eq!(state.holder.as_deref(), Some("agent-a"));

    mgr.unlock("session-1", "agent-a").await?;
    let state = mgr.get_lock_state("session-1").await?;
    assert!(state.holder.is_none(), "Expected no holder after unlock");

    Ok(())
}
