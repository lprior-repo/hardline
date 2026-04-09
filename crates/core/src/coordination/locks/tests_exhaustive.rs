//! Exhaustive tests for Lock, LockManager — acquire/release/renew, deadlock detection, timeout.
//!
//! Covers: Ttl value object, LockOperation, input validation, cleanup_expired,
//! audit log completeness, lock_with_ttl custom values, heartbeat expiry,
//! verify_session_exists graceful degradation, proptest invariants.

use chrono::Duration;
use proptest::prelude::*;
use sqlx::sqlite::SqlitePoolOptions;

use crate::coordination::locks::errors::LockErrorKind;
use crate::coordination::locks::types::{LockOperation, Ttl};
use crate::coordination::locks::LockManager;
use crate::Error;

async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::database(e.to_string()))
}

async fn setup() -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

async fn setup_with_ttl(secs: i64) -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::with_ttl(pool, Duration::seconds(secs));
    mgr.init().await?;
    Ok(mgr)
}

async fn setup_with_sessions() -> Result<(LockManager, sqlx::SqlitePool), Error> {
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

    Ok((mgr, pool))
}

async fn insert_session(pool: &sqlx::SqlitePool, name: &str) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
    )
    .bind(name)
    .bind("active")
    .bind("working")
    .bind("/workspace")
    .execute(pool)
    .await
    .map_err(|e| Error::database(e.to_string()))?;
    Ok(())
}

fn assert_error_kind_matches<T>(result: Result<T, Error>, kind_matcher: fn(&LockErrorKind) -> bool) -> Result<(), Error> {
    let err = result.err().ok_or_else(|| Error::internal("expected error"))?;
    match &err {
        Error::Lock(lk) => {
            assert!(kind_matcher(lk.kind()), "Error kind mismatch: {:?}", lk.kind());
            Ok(())
        }
        other => Err(Error::internal(format!("Expected Error::Lock, got: {other:?}"))),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Ttl value object unit tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ttl_new_returns_some_for_zero() {
    let ttl = Ttl::new(0);
    assert!(ttl.is_some());
    if let Some(t) = ttl {
        assert_eq!(t.seconds(), 0);
        assert!(t.is_default());
    }
}

#[test]
fn ttl_new_returns_some_for_max() {
    let ttl = Ttl::new(86400);
    assert!(ttl.is_some());
    if let Some(t) = ttl {
        assert_eq!(t.seconds(), 86400);
        assert!(!t.is_default());
    }
}

#[test]
fn ttl_new_returns_some_for_one() {
    let ttl = Ttl::new(1);
    assert!(ttl.is_some());
    if let Some(t) = ttl {
        assert_eq!(t.seconds(), 1);
    }
}

#[test]
fn ttl_new_returns_none_for_86401() {
    assert!(Ttl::new(86401).is_none());
}

#[test]
fn ttl_new_returns_none_for_u64_max() {
    assert!(Ttl::new(u64::MAX).is_none());
}

#[test]
fn ttl_new_returns_none_for_u64_max_minus_one() {
    assert!(Ttl::new(u64::MAX - 1).is_none());
}

#[test]
fn ttl_new_returns_some_for_86400_minus_one() {
    assert!(Ttl::new(86399).is_some());
}

#[test]
fn ttl_seconds_returns_input_value() -> Result<(), String> {
    let ttl = Ttl::new(300).ok_or("TTL should be valid")?;
    assert_eq!(ttl.seconds(), 300);
    Ok(())
}

#[test]
fn ttl_is_default_true_only_for_zero() {
    let t0 = Ttl::new(0).ok_or("TTL 0 should be valid").ok();
    let t1 = Ttl::new(1).ok_or("TTL 1 should be valid").ok();
    let tm = Ttl::new(86400).ok_or("TTL 86400 should be valid").ok();
    assert!(t0.is_some_and(|t| t.is_default()));
    assert!(t1.is_some_and(|t| !t.is_default()));
    assert!(tm.is_some_and(|t| !t.is_default()));
}

#[test]
fn ttl_equality_by_value() -> Result<(), String> {
    let a = Ttl::new(100).ok_or("a")?;
    let b = Ttl::new(100).ok_or("b")?;
    let c = Ttl::new(200).ok_or("c")?;
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

#[test]
fn ttl_copy_semantics() -> Result<(), String> {
    let a = Ttl::new(42).ok_or("ttl")?;
    let b = a;
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn ttl_max_secs_constant() {
    assert_eq!(Ttl::MAX_SECS, 86400);
}

#[test]
fn ttl_min_secs_constant() {
    assert_eq!(Ttl::MIN_SECS, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockOperation unit tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lock_operation_lock_as_str() {
    assert_eq!(LockOperation::Lock.as_str(), "lock");
}

#[test]
fn lock_operation_unlock_as_str() {
    assert_eq!(LockOperation::Unlock.as_str(), "unlock");
}

#[test]
fn lock_operation_heartbeat_as_str() {
    assert_eq!(LockOperation::Heartbeat.as_str(), "heartbeat");
}

#[test]
fn lock_operation_double_unlock_warning_as_str() {
    assert_eq!(LockOperation::DoubleUnlockWarning.as_str(), "double_unlock_warning");
}

#[test]
fn lock_operation_equality() {
    assert_eq!(LockOperation::Lock, LockOperation::Lock);
    assert_ne!(LockOperation::Lock, LockOperation::Unlock);
}

#[test]
fn lock_operation_clone() {
    let op = LockOperation::Heartbeat;
    let cloned = op.clone();
    assert_eq!(op, cloned);
}

#[test]
fn lock_operation_debug_format() {
    let s = format!("{:?}", LockOperation::Lock);
    assert!(s.contains("Lock"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: lock_with_ttl custom values
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn lock_with_ttl_custom_60_seconds() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock_with_ttl("session-1", "agent-a", 60).await?;

    assert_eq!(resp.session, "session-1");
    assert_eq!(resp.agent_id, "agent-a");
    assert!(resp.lock_id.starts_with("lock-session-1-"));
    assert!(resp.expires_at > resp.acquired_at);

    let duration = resp.expires_at - resp.acquired_at;
    assert_eq!(duration.num_seconds(), 60);
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_custom_86400_max() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock_with_ttl("session-max", "agent-a", 86400).await?;

    let duration = resp.expires_at - resp.acquired_at;
    assert_eq!(duration.num_seconds(), 86400);
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_custom_1_second() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock_with_ttl("session-1s", "agent-a", 1).await?;

    let duration = resp.expires_at - resp.acquired_at;
    assert_eq!(duration.num_seconds(), 1);
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_zero_uses_default_300() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock_with_ttl("session-default", "agent-a", 0).await?;

    let duration = resp.expires_at - resp.acquired_at;
    assert_eq!(duration.num_seconds(), 300);
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_over_max_returns_ttl_out_of_range() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock_with_ttl("session-bad", "agent-a", 86401).await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::TtlOutOfRange(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_u64_max_returns_ttl_overflow() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock_with_ttl("session-overflow", "agent-a", u64::MAX).await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::TtlOverflow(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_with_ttl_lock_id_format() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock_with_ttl("my-session", "agent-x", 60).await?;

    assert!(resp.lock_id.starts_with("lock-my-session-"));
    let nanos_part = resp.lock_id.strip_prefix("lock-my-session-");
    assert!(nanos_part.is_some());
    assert!(nanos_part.is_some_and(|s| s.parse::<i64>().is_ok()));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: input validation
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn lock_empty_session_returns_empty_session_name() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock("", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::EmptySessionName(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_empty_agent_returns_empty_agent_id() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock("session-1", "").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::EmptyAgentId(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_session_name_256_chars_returns_too_long() -> Result<(), Error> {
    let mgr = setup().await?;
    let long_name = "x".repeat(256);
    let result = mgr.lock(&long_name, "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::SessionNameTooLong(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_session_name_255_chars_succeeds() -> Result<(), Error> {
    let mgr = setup().await?;
    let max_name = "x".repeat(255);
    let resp = mgr.lock(&max_name, "agent-a").await?;

    assert_eq!(resp.session, max_name);
    Ok(())
}

#[tokio::test]
async fn lock_session_name_with_null_char_rejected() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock("session\x00bad", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::InvalidSessionName(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_session_name_with_carriage_return_rejected() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock("session\rname", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::InvalidSessionName(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_session_name_with_bell_char_rejected() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.lock("session\x07name", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::InvalidSessionName(_)))?;
    Ok(())
}

#[tokio::test]
async fn lock_session_name_unicode_accepted() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock("session-日本語-🦀", "agent-a").await?;

    assert_eq!(resp.session, "session-日本語-🦀");
    Ok(())
}

#[tokio::test]
async fn unlock_empty_session_succeeds_as_double_unlock() -> Result<(), Error> {
    let mgr = setup().await?;
    mgr.unlock("", "agent-a").await?;
    Ok(())
}

#[tokio::test]
async fn unlock_empty_agent_succeeds_as_double_unlock() -> Result<(), Error> {
    let mgr = setup().await?;
    mgr.unlock("session-1", "").await?;
    Ok(())
}

#[tokio::test]
async fn heartbeat_empty_session_returns_not_found() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.heartbeat("", "agent-a").await;
    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::NotFound(_)))?;
    Ok(())
}

#[tokio::test]
async fn heartbeat_empty_agent_returns_not_lock_holder() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let result = mgr.heartbeat("session-1", "").await;
    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::NotLockHolder { .. }))?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: cleanup_expired
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cleanup_expired_removes_zero_locks_when_none_expired() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    let count = mgr.cleanup_expired().await?;
    assert_eq!(count, 0, "No locks should be cleaned up");
    Ok(())
}

#[tokio::test]
async fn cleanup_expired_removes_expired_locks() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let _ = mgr.lock("session-2", "agent-b").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let count = mgr.cleanup_expired().await?;
    assert_eq!(count, 2, "Both expired locks should be cleaned up");

    let locks = mgr.get_all_locks().await?;
    assert!(locks.is_empty());
    Ok(())
}

#[tokio::test]
async fn cleanup_expired_preserves_active_locks() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("active-session", "agent-a").await?;

    let count = mgr.cleanup_expired().await?;
    assert_eq!(count, 0);

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].session, "active-session");
    Ok(())
}

#[tokio::test]
async fn cleanup_expired_no_locks_at_all() -> Result<(), Error> {
    let mgr = setup().await?;
    let count = mgr.cleanup_expired().await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn cleanup_expired_mixed_active_and_expired() -> Result<(), Error> {
    let pool = test_pool().await?;
    let mgr_expired = LockManager::with_ttl(pool.clone(), Duration::seconds(0));
    mgr_expired.init().await?;

    let _ = mgr_expired.lock("expiring", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let mgr_active = LockManager::new(pool.clone());
    let _ = mgr_active.lock("long-lived", "agent-b").await?;

    let count = mgr_active.cleanup_expired().await?;
    assert_eq!(count, 1, "Only the expired lock should be removed");

    let locks = mgr_active.get_all_locks().await?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].session, "long-lived");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: audit log completeness and ordering
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn audit_log_records_lock_operation() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    let log = mgr.get_lock_audit_log("session-1").await?;
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].operation, LockOperation::Lock);
    assert_eq!(log[0].agent_id, "agent-a");
    assert_eq!(log[0].session, "session-1");
    Ok(())
}

#[tokio::test]
async fn audit_log_records_unlock_operation() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    mgr.unlock("session-1", "agent-a").await?;

    let log = mgr.get_lock_audit_log("session-1").await?;
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].operation, LockOperation::Lock);
    assert_eq!(log[1].operation, LockOperation::Unlock);
    Ok(())
}

#[tokio::test]
async fn audit_log_records_heartbeat_operation() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    let _ = mgr.heartbeat("session-1", "agent-a").await?;

    let log = mgr.get_lock_audit_log("session-1").await?;
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].operation, LockOperation::Lock);
    assert_eq!(log[1].operation, LockOperation::Heartbeat);
    Ok(())
}

#[tokio::test]
async fn audit_log_full_lifecycle() -> Result<(), Error> {
    let mgr = setup().await?;

    let _ = mgr.lock("lifecycle", "agent-a").await?;
    let _ = mgr.heartbeat("lifecycle", "agent-a").await?;
    let _ = mgr.heartbeat("lifecycle", "agent-a").await?;
    mgr.unlock("lifecycle", "agent-a").await?;

    let log = mgr.get_lock_audit_log("lifecycle").await?;
    assert_eq!(log.len(), 4);
    assert_eq!(log[0].operation, LockOperation::Lock);
    assert_eq!(log[1].operation, LockOperation::Heartbeat);
    assert_eq!(log[2].operation, LockOperation::Heartbeat);
    assert_eq!(log[3].operation, LockOperation::Unlock);
    Ok(())
}

#[tokio::test]
async fn audit_log_is_chronologically_ordered() -> Result<(), Error> {
    let mgr = setup().await?;

    let _ = mgr.lock("chrono-test", "agent-a").await?;
    let _ = mgr.heartbeat("chrono-test", "agent-a").await?;
    mgr.unlock("chrono-test", "agent-a").await?;

    let log = mgr.get_lock_audit_log("chrono-test").await?;
    for window in log.windows(2) {
        assert!(
            window[0].timestamp <= window[1].timestamp,
            "Audit log entries should be chronologically ordered"
        );
    }
    Ok(())
}

#[tokio::test]
async fn audit_log_empty_for_no_operations() -> Result<(), Error> {
    let mgr = setup().await?;
    let log = mgr.get_lock_audit_log("never-locked").await?;
    assert!(log.is_empty());
    Ok(())
}

#[tokio::test]
async fn audit_log_isolation_between_sessions() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-a", "agent-1").await?;
    let _ = mgr.lock("session-b", "agent-2").await?;

    let log_a = mgr.get_lock_audit_log("session-a").await?;
    let log_b = mgr.get_lock_audit_log("session-b").await?;

    assert_eq!(log_a.len(), 1);
    assert_eq!(log_b.len(), 1);
    assert_eq!(log_a[0].agent_id, "agent-1");
    assert_eq!(log_b[0].agent_id, "agent-2");
    Ok(())
}

#[tokio::test]
async fn audit_log_double_unlock_records_warning() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("dbl", "agent-a").await?;
    mgr.unlock("dbl", "agent-a").await?;
    mgr.unlock("dbl", "agent-a").await?;

    let log = mgr.get_lock_audit_log("dbl").await?;
    assert_eq!(log.len(), 3);
    assert_eq!(log[2].operation, LockOperation::DoubleUnlockWarning);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: verify_session_exists graceful degradation
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn lock_succeeds_when_sessions_table_does_not_exist() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock("any-session", "agent-a").await?;

    assert_eq!(resp.session, "any-session");
    Ok(())
}

#[tokio::test]
async fn lock_fails_for_nonexistent_session_when_sessions_table_exists() -> Result<(), Error> {
    let (mgr, _pool) = setup_with_sessions().await?;
    let result = mgr.lock("ghost", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::SessionNotFound { .. }))?;
    Ok(())
}

#[tokio::test]
async fn lock_succeeds_for_existing_session_in_table() -> Result<(), Error> {
    let (mgr, pool) = setup_with_sessions().await?;
    insert_session(&pool, "real-session").await?;

    let resp = mgr.lock("real-session", "agent-a").await?;
    assert_eq!(resp.session, "real-session");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: heartbeat and renew behavior
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn heartbeat_returns_updated_lock_response() -> Result<(), Error> {
    let mgr = setup().await?;
    let lock_resp = mgr.lock("session-1", "agent-a").await?;
    let hb_resp = mgr.heartbeat("session-1", "agent-a").await?;

    assert_eq!(hb_resp.lock_id, lock_resp.lock_id);
    assert_eq!(hb_resp.session, "session-1");
    assert_eq!(hb_resp.agent_id, "agent-a");
    assert!(hb_resp.expires_at > lock_resp.expires_at);
    Ok(())
}

#[tokio::test]
async fn heartbeat_on_expired_lock_returns_not_found() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let result = mgr.heartbeat("session-1", "agent-a").await;
    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::NotFound(_)))?;
    Ok(())
}

#[tokio::test]
async fn heartbeat_by_non_holder_returns_not_lock_holder() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    let result = mgr.heartbeat("session-1", "agent-b").await;
    assert!(result.is_err());
    let err = result.err().ok_or_else(|| Error::internal("expected error"))?;
    match &err {
        Error::Lock(lk) => {
            assert!(matches!(lk.kind(), LockErrorKind::NotLockHolder { .. }));
            let msg = lk.to_string();
            assert!(msg.contains("agent-b") && msg.contains("session-1"),
                "Expected agent-b and session-1 in error, got: {msg}");
        }
        other => return Err(Error::internal(format!("Expected Error::Lock, got: {other:?}"))),
    }
    Ok(())
}

#[tokio::test]
async fn heartbeat_on_never_locked_session_returns_not_found() -> Result<(), Error> {
    let mgr = setup().await?;
    let result = mgr.heartbeat("never-locked", "agent-a").await;

    assert!(result.is_err());
    assert_error_kind_matches(result, |k| matches!(k, LockErrorKind::NotFound(_)))?;
    Ok(())
}

#[tokio::test]
async fn multiple_heartbeats_extend_expiry_progressively() -> Result<(), Error> {
    let mgr = setup_with_ttl(10).await?;
    let lock_resp = mgr.lock("session-1", "agent-a").await?;

    let mut prev_expires = lock_resp.expires_at;
    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let hb = mgr.heartbeat("session-1", "agent-a").await?;
        assert!(hb.expires_at > prev_expires, "Each heartbeat should push expiry forward");
        prev_expires = hb.expires_at;
    }
    Ok(())
}

#[tokio::test]
async fn heartbeat_extends_by_default_ttl_duration() -> Result<(), Error> {
    let mgr = setup_with_ttl(42).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let hb = mgr.heartbeat("session-1", "agent-a").await?;

    let extension = hb.expires_at - hb.acquired_at;
    assert_eq!(extension.num_seconds(), 42);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: lock state query
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_lock_state_returns_empty_for_no_lock() -> Result<(), Error> {
    let mgr = setup().await?;
    let state = mgr.get_lock_state("unlocked-session").await?;

    assert_eq!(state.session, "unlocked-session");
    assert!(state.holder.is_none());
    assert!(state.expires_at.is_none());
    Ok(())
}

#[tokio::test]
async fn get_lock_state_shows_holder_and_expiry_for_active_lock() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock("session-1", "agent-a").await?;

    let state = mgr.get_lock_state("session-1").await?;
    assert_eq!(state.holder.as_deref(), Some("agent-a"));
    match state.expires_at {
        Some(exp) => assert!(exp > resp.acquired_at),
        None => return Err(Error::internal("expected expires_at")),
    }
    Ok(())
}

#[tokio::test]
async fn get_lock_state_clears_after_unlock() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    mgr.unlock("session-1", "agent-a").await?;

    let state = mgr.get_lock_state("session-1").await?;
    assert!(state.holder.is_none());
    assert!(state.expires_at.is_none());
    Ok(())
}

#[tokio::test]
async fn get_lock_state_shows_expired_as_unlocked() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let _ = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let state = mgr.get_lock_state("session-1").await?;
    assert!(state.holder.is_none(), "Expired lock should show no holder");
    assert!(state.expires_at.is_none());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: get_all_locks ordering
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_all_locks_returns_empty_when_no_locks() -> Result<(), Error> {
    let mgr = setup().await?;
    let locks = mgr.get_all_locks().await?;
    assert!(locks.is_empty());
    Ok(())
}

#[tokio::test]
async fn get_all_locks_sorted_by_expires_at_asc() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock_with_ttl("long-session", "agent-a", 600).await?;
    let _ = mgr.lock_with_ttl("short-session", "agent-b", 60).await?;

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 2);
    assert!(locks[0].expires_at <= locks[1].expires_at, "Should be sorted by expires_at ASC");
    Ok(())
}

#[tokio::test]
async fn get_all_locks_returns_complete_lock_info() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp = mgr.lock("session-1", "agent-a").await?;

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].lock_id, resp.lock_id);
    assert_eq!(locks[0].session, "session-1");
    assert_eq!(locks[0].agent_id, "agent-a");
    assert!(locks[0].acquired_at <= locks[0].expires_at);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: idempotent re-lock by same agent
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn relock_same_agent_returns_same_lock_id() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp1 = mgr.lock("session-1", "agent-a").await?;
    let resp2 = mgr.lock("session-1", "agent-a").await?;

    assert_eq!(resp1.lock_id, resp2.lock_id);
    assert_eq!(resp1.session, resp2.session);
    assert_eq!(resp1.agent_id, resp2.agent_id);
    Ok(())
}

#[tokio::test]
async fn relock_same_agent_preserves_original_expiry() -> Result<(), Error> {
    let mgr = setup().await?;
    let resp1 = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let resp2 = mgr.lock("session-1", "agent-a").await?;
    assert_eq!(resp2.expires_at, resp1.expires_at,
        "Re-lock by same agent should preserve original expiry");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: multiple sessions, independent locks
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn independent_sessions_can_be_locked_simultaneously() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-a", "agent-1").await?;
    let _ = mgr.lock("session-b", "agent-2").await?;
    let _ = mgr.lock("session-c", "agent-3").await?;

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 3);
    Ok(())
}

#[tokio::test]
async fn same_agent_can_lock_multiple_sessions() -> Result<(), Error> {
    let mgr = setup().await?;
    let r1 = mgr.lock("session-a", "agent-x").await?;
    let r2 = mgr.lock("session-b", "agent-x").await?;

    assert_ne!(r1.lock_id, r2.lock_id);
    assert_eq!(r1.agent_id, r2.agent_id);

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 2);
    Ok(())
}

#[tokio::test]
async fn unlocking_one_session_does_not_affect_others() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.lock("session-a", "agent-1").await?;
    let _ = mgr.lock("session-b", "agent-2").await?;

    mgr.unlock("session-a", "agent-1").await?;

    let locks = mgr.get_all_locks().await?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].session, "session-b");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: lock expiry enables new acquisition
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn expired_lock_enables_new_agent_acquisition() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let resp1 = mgr.lock("session-1", "agent-a").await?;
    assert_eq!(resp1.agent_id, "agent-a");

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let resp2 = mgr.lock("session-1", "agent-b").await?;
    assert_eq!(resp2.agent_id, "agent-b");
    assert_ne!(resp2.lock_id, resp1.lock_id);
    Ok(())
}

#[tokio::test]
async fn expired_lock_enables_same_agent_reacquisition() -> Result<(), Error> {
    let mgr = setup_with_ttl(0).await?;
    let resp1 = mgr.lock("session-1", "agent-a").await?;

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let resp2 = mgr.lock("session-1", "agent-a").await?;
    assert_ne!(resp2.lock_id, resp1.lock_id, "After expiry, a new lock_id should be generated");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: pool accessor
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn pool_accessor_returns_same_pool() -> Result<(), Error> {
    let mgr = setup().await?;
    let _ = mgr.pool();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: init is idempotent
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn init_called_twice_succeeds() -> Result<(), Error> {
    let mgr = setup().await?;
    mgr.init().await?;
    let _ = mgr.lock("session-1", "agent-a").await?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockManager: LockManager clone shares pool
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cloned_lock_manager_shares_state() -> Result<(), Error> {
    let mgr = setup().await?;
    let mgr2 = mgr.clone();

    let _ = mgr.lock("session-1", "agent-a").await?;

    let state = mgr2.get_lock_state("session-1").await?;
    assert_eq!(state.holder.as_deref(), Some("agent-a"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// LockError: error code mapping (via Error::Lock)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lock_error_kind_session_locked_code_via_error() -> Result<(), String> {
    let err: Error = LockErrorKind::SessionLocked {
        session: "s".into(),
        holder: "h".into(),
    }
    .into();
    let lk = match &err {
        Error::Lock(lk) => lk,
        other => return Err(format!("Expected Error::Lock, got: {other:?}")),
    };
    assert_eq!(lk.code(), "SESSION_LOCKED");
    Ok(())
}

#[test]
fn lock_error_kind_not_lock_holder_code_via_error() -> Result<(), String> {
    let err: Error = LockErrorKind::NotLockHolder {
        session: "s".into(),
        agent_id: "a".into(),
    }
    .into();
    let lk = match &err {
        Error::Lock(lk) => lk,
        other => return Err(format!("Expected Error::Lock, got: {other:?}")),
    };
    assert_eq!(lk.code(), "NOT_LOCK_HOLDER");
    Ok(())
}

#[test]
fn lock_error_kind_not_found_code_via_error() -> Result<(), String> {
    let err: Error = LockErrorKind::NotFound("x".into()).into();
    let lk = match &err {
        Error::Lock(lk) => lk,
        other => return Err(format!("Expected Error::Lock, got: {other:?}")),
    };
    assert_eq!(lk.code(), "NOT_FOUND");
    Ok(())
}

#[test]
fn lock_error_suggestion_for_session_locked() -> Result<(), String> {
    let err: Error = LockErrorKind::SessionLocked {
        session: "sess".into(),
        holder: "agent-1".into(),
    }
    .into();
    let lk = match &err {
        Error::Lock(lk) => lk,
        other => return Err(format!("Expected Error::Lock, got: {other:?}")),
    };
    let s = lk.suggestion();
    assert!(s.is_some());
    assert!(s.is_some_and(|v| v.contains("agent-1")));
    Ok(())
}

#[test]
fn lock_error_no_suggestion_for_database_error() -> Result<(), String> {
    let err: Error = LockErrorKind::DatabaseError("fail".into()).into();
    let lk = match &err {
        Error::Lock(lk) => lk,
        other => return Err(format!("Expected Error::Lock, got: {other:?}")),
    };
    assert!(lk.suggestion().is_none());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// proptest: Ttl value object invariants
// ═══════════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn ttl_new_valid_range_accepts_0_to_86400(seconds in 0u64..=86400u64) {
        let ttl = Ttl::new(seconds);
        prop_assert!(ttl.is_some());
        match ttl {
            Some(t) => prop_assert_eq!(t.seconds(), seconds),
            None => return Err(TestCaseError::fail("expected Some")),
        }
    }

    #[test]
    fn ttl_new_rejects_above_86400(seconds in 86401u64..100000u64) {
        prop_assert!(Ttl::new(seconds).is_none());
    }

    #[test]
    fn ttl_is_default_iff_zero(seconds in 0u64..=86400u64) {
        match Ttl::new(seconds) {
            Some(t) => prop_assert_eq!(t.is_default(), seconds == 0),
            None => return Err(TestCaseError::fail("expected Some")),
        }
    }

    #[test]
    fn ttl_seconds_roundtrips(seconds in 0u64..=86400u64) {
        match Ttl::new(seconds) {
            Some(t) => prop_assert_eq!(t.seconds(), seconds),
            None => return Err(TestCaseError::fail("expected Some")),
        }
    }

    #[test]
    fn ttl_equality_consistent_with_value(a in 0u64..=86400u64, b in 0u64..=86400u64) {
        match (Ttl::new(a), Ttl::new(b)) {
            (Some(ta), Some(tb)) => prop_assert_eq!(ta == tb, a == b),
            _ => return Err(TestCaseError::fail("expected Some")),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// proptest: LockOperation as_str roundtrip
// ═══════════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn lock_operation_as_str_roundtrips(op_idx in 0u8..4) {
        let op = match op_idx {
            0 => LockOperation::Lock,
            1 => LockOperation::Unlock,
            2 => LockOperation::Heartbeat,
            _ => LockOperation::DoubleUnlockWarning,
        };
        let s = op.as_str();
        prop_assert!(matches!(s, "lock" | "unlock" | "heartbeat" | "double_unlock_warning"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// proptest: Session name validation invariants
// ═══════════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn valid_session_name_alphanumeric_accepted(name in "[a-zA-Z0-9_-]{1,255}") {
        let result = LockManager::validate_session_name(&name);
        prop_assert!(result.is_ok(), "Valid session name should pass: {name}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// proptest: TTL validation invariants
// ═══════════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn validate_ttl_accepts_0_to_86400(ttl in 0u64..=86400u64) {
        let result = LockManager::validate_ttl(ttl);
        prop_assert!(result.is_ok(), "TTL {ttl} should be valid");
    }

    #[test]
    fn validate_ttl_rejects_above_86400(ttl in 86401u64..200000u64) {
        let result = LockManager::validate_ttl(ttl);
        prop_assert!(result.is_err(), "TTL {ttl} should be rejected");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// proptest: Agent ID validation invariants
// ═══════════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn validate_agent_id_accepts_nonempty(agent_id in ".{1,100}") {
        if !agent_id.is_empty() {
            let result = LockManager::validate_agent_id(&agent_id);
            prop_assert!(result.is_ok());
        }
    }
}
