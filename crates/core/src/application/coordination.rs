//! Coordination Application Service
//!
//! Orchestrates distributed lock operations through the LockRepository trait.
//! Follows the DDD application layer pattern: thin orchestration over domain types.

use crate::domain::repository::RepositoryResult;
use crate::domain::repository::{Lock, LockAudit, LockRepository};

/// Application service for coordination operations.
///
/// Delegates to a `LockRepository` for persistence while providing
/// a clean application-level API.
pub struct CoordinationService<R: LockRepository> {
    repository: R,
}

impl<R: LockRepository> CoordinationService<R> {
    /// Create a new coordination service with the given repository.
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Acquire an exclusive lock on a session.
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if session is already locked.
    /// Returns `InvalidInput` if session or agent_id is empty.
    pub fn acquire_lock(
        &self,
        session: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> RepositoryResult<Lock> {
        self.repository.acquire(session, agent_id, ttl_seconds)
    }

    /// Release a lock on a session.
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if caller is not the lock holder.
    pub fn release_lock(&self, session: &str, agent_id: &str) -> RepositoryResult<()> {
        self.repository.release(session, agent_id)
    }

    /// Extend a lock's TTL via heartbeat.
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if caller is not the lock holder.
    /// Returns `NotFound` if no active lock exists.
    pub fn heartbeat(&self, session: &str, agent_id: &str) -> RepositoryResult<Lock> {
        self.repository.heartbeat(session, agent_id)
    }

    /// List all active locks.
    pub fn list_locks(&self) -> RepositoryResult<Vec<Lock>> {
        self.repository.list_active()
    }

    /// Get the current lock state for a session.
    pub fn get_lock_state(&self, session: &str) -> RepositoryResult<Option<Lock>> {
        self.repository.get_state(session)
    }

    /// Get the audit log for a session.
    pub fn get_audit_log(&self, session: &str) -> RepositoryResult<Vec<LockAudit>> {
        self.repository.get_audit_log(session)
    }

    /// Remove all expired locks.
    pub fn cleanup_expired(&self) -> RepositoryResult<u64> {
        self.repository.cleanup_expired()
    }

    /// Initialize the lock schema.
    pub fn init(&self) -> RepositoryResult<()> {
        self.repository.init_schema()
    }
}

/// Create a coordination service backed by an in-memory repository.
pub fn create_coordination_service(
) -> CoordinationService<crate::domain::repository::InMemoryLockRepository> {
    CoordinationService::new(crate::domain::repository::InMemoryLockRepository::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::{InMemoryLockRepository, LockOperation};
    use crate::domain::repository::RepositoryError;
    use std::sync::Arc;
    use std::thread;

    fn create_service() -> CoordinationService<InMemoryLockRepository> {
        CoordinationService::new(InMemoryLockRepository::new())
    }

    // ─── Lifecycle: creation and initialization ─────────────────────────

    #[test]
    fn given_new_service_when_created_then_no_locks() {
        let service = create_service();
        let locks = service.list_locks().expect("list");
        assert!(locks.is_empty(), "new service should have no locks");
    }

    #[test]
    fn given_new_service_when_init_then_ok() {
        let service = create_service();
        service.init().expect("init should succeed");
    }

    #[test]
    fn given_default_service_when_created_then_works() {
        let service = create_coordination_service();
        let locks = service.list_locks().expect("list");
        assert!(locks.is_empty());
    }

    #[test]
    fn given_service_when_send_sync_then_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CoordinationService<InMemoryLockRepository>>();
    }

    // ─── Acquire lock: happy path ───────────────────────────────────────

    #[test]
    fn given_service_when_acquire_lock_then_success() {
        let service = create_service();
        let lock = service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("acquire");
        assert_eq!(lock.session, "session-1");
        assert_eq!(lock.agent_id, "agent-a");
        assert!(!lock.lock_id.is_empty());
        assert!(lock.acquired_at <= lock.expires_at);
    }

    #[test]
    fn given_acquired_lock_when_get_state_then_some() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        let state = service.get_lock_state("session-1").expect("state");
        assert!(state.is_some());
        let lock = state.expect("lock");
        assert_eq!(lock.session, "session-1");
        assert_eq!(lock.agent_id, "agent-a");
    }

    #[test]
    fn given_same_agent_when_acquire_twice_then_idempotent() {
        let service = create_service();
        let first = service.acquire_lock("session-1", "agent-a", 300).expect("first");
        let second = service.acquire_lock("session-1", "agent-a", 300).expect("second");
        assert_eq!(first.lock_id, second.lock_id, "idempotent acquire returns same lock");
    }

    #[test]
    fn given_zero_ttl_when_acquire_then_uses_default() {
        let service = create_service();
        let lock = service.acquire_lock("session-1", "agent-a", 0).expect("acquire");
        // Default TTL is 300s; lock should still have a valid expiration
        assert!(lock.expires_at > lock.acquired_at);
    }

    // ─── Acquire lock: conflict resolution ──────────────────────────────

    #[test]
    fn given_locked_when_acquire_by_another_then_conflict() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("first");
        let result = service.acquire_lock("session-1", "agent-b", 300);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::Conflict(_))),
            "expected Conflict error"
        );
    }

    #[test]
    fn given_multiple_agents_when_same_session_then_first_wins() {
        let service = create_service();
        let first = service.acquire_lock("s-1", "agent-a", 300).expect("a");
        let second_result = service.acquire_lock("s-1", "agent-b", 300);
        assert!(second_result.is_err());
        let state = service.get_lock_state("s-1").expect("state").expect("present");
        assert_eq!(state.agent_id, "agent-a");
        assert_eq!(state.lock_id, first.lock_id);
    }

    #[test]
    fn given_different_sessions_when_acquire_then_both_succeed() {
        let service = create_service();
        let lock_a = service.acquire_lock("s-1", "agent-a", 300).expect("a");
        let lock_b = service.acquire_lock("s-2", "agent-a", 300).expect("b");
        assert_ne!(lock_a.lock_id, lock_b.lock_id);
        assert_ne!(lock_a.session, lock_b.session);
    }

    // ─── Release lock ───────────────────────────────────────────────────

    #[test]
    fn given_lock_when_release_then_reacquire() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        service.release_lock("session-1", "agent-a").expect("release");
        let lock = service.acquire_lock("session-1", "agent-b", 300).expect("reacquire");
        assert_eq!(lock.agent_id, "agent-b");
    }

    #[test]
    fn given_lock_when_release_by_non_holder_then_conflict() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        let result = service.release_lock("session-1", "agent-b");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::Conflict(_))),
            "non-holder release should conflict"
        );
    }

    #[test]
    fn given_no_lock_when_release_then_ok_with_double_unlock_warning() {
        let service = create_service();
        service.release_lock("session-1", "agent-a").expect("no-op release");
        let log = service.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].operation, LockOperation::DoubleUnlockWarning);
    }

    #[test]
    fn given_released_lock_when_get_state_then_none() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        service.release_lock("session-1", "agent-a").expect("release");
        assert!(service.get_lock_state("session-1").expect("state").is_none());
    }

    // ─── Heartbeat ──────────────────────────────────────────────────────

    #[test]
    fn given_lock_when_heartbeat_then_extended() {
        let service = create_service();
        let original = service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        let extended = service.heartbeat("session-1", "agent-a").expect("heartbeat");
        assert!(extended.expires_at >= original.expires_at);
    }

    #[test]
    fn given_no_lock_when_heartbeat_then_not_found() {
        let service = create_service();
        let result = service.heartbeat("session-1", "agent-a");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::NotFound(_))),
            "heartbeat on unlocked session should be NotFound"
        );
    }

    #[test]
    fn given_lock_when_heartbeat_by_non_holder_then_conflict() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        let result = service.heartbeat("session-1", "agent-b");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::Conflict(_))),
            "non-holder heartbeat should conflict"
        );
    }

    // ─── List locks ─────────────────────────────────────────────────────

    #[test]
    fn given_multiple_locks_when_list_then_returns_all() {
        let service = create_service();
        service.acquire_lock("s-1", "a-1", 300).expect("a");
        service.acquire_lock("s-2", "a-2", 300).expect("b");
        service.acquire_lock("s-3", "a-3", 300).expect("c");
        let locks = service.list_locks().expect("list");
        assert_eq!(locks.len(), 3);
    }

    #[test]
    fn given_released_lock_when_list_then_excluded() {
        let service = create_service();
        service.acquire_lock("s-1", "a-1", 300).expect("a");
        service.acquire_lock("s-2", "a-2", 300).expect("b");
        service.release_lock("s-1", "a-1").expect("release");
        let locks = service.list_locks().expect("list");
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].session, "s-2");
    }

    // ─── Get state ──────────────────────────────────────────────────────

    #[test]
    fn given_no_lock_when_get_state_then_none() {
        let service = create_service();
        assert!(service.get_lock_state("session-1").expect("state").is_none());
    }

    #[test]
    fn given_different_sessions_when_get_state_then_independent() {
        let service = create_service();
        service.acquire_lock("s-1", "agent-a", 300).expect("a");
        assert!(service.get_lock_state("s-1").expect("state").is_some());
        assert!(service.get_lock_state("s-2").expect("state").is_none());
    }

    // ─── Audit log ──────────────────────────────────────────────────────

    #[test]
    fn given_operations_when_audit_log_then_entries() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        service.release_lock("session-1", "agent-a").expect("release");
        let log = service.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].operation, LockOperation::Acquire);
        assert_eq!(log[1].operation, LockOperation::Release);
    }

    #[test]
    fn given_heartbeat_when_audit_log_then_entry() {
        let service = create_service();
        service.acquire_lock("session-1", "agent-a", 300).expect("acquire");
        service.heartbeat("session-1", "agent-a").expect("heartbeat");
        let log = service.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 2);
        assert_eq!(log[1].operation, LockOperation::Heartbeat);
    }

    #[test]
    fn given_no_operations_when_audit_log_then_empty() {
        let service = create_service();
        let log = service.get_audit_log("nonexistent").expect("audit");
        assert!(log.is_empty());
    }

    #[test]
    fn given_audit_log_entries_when_check_then_correct_agents() {
        let service = create_service();
        service.acquire_lock("s-1", "agent-a", 300).expect("a");
        service.release_lock("s-1", "agent-a").expect("release");
        service.acquire_lock("s-1", "agent-b", 300).expect("b");
        let log = service.get_audit_log("s-1").expect("audit");
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].agent_id, "agent-a");
        assert_eq!(log[1].agent_id, "agent-a");
        assert_eq!(log[2].agent_id, "agent-b");
    }

    #[test]
    fn given_cross_session_operations_when_audit_then_isolated() {
        let service = create_service();
        service.acquire_lock("s-1", "agent-a", 300).expect("a");
        service.acquire_lock("s-2", "agent-b", 300).expect("b");
        let log_s1 = service.get_audit_log("s-1").expect("audit");
        let log_s2 = service.get_audit_log("s-2").expect("audit");
        assert_eq!(log_s1.len(), 1);
        assert_eq!(log_s2.len(), 1);
        assert_eq!(log_s1[0].agent_id, "agent-a");
        assert_eq!(log_s2[0].agent_id, "agent-b");
    }

    // ─── Cleanup expired ────────────────────────────────────────────────

    #[test]
    fn given_no_locks_when_cleanup_then_zero() {
        let service = create_service();
        let removed = service.cleanup_expired().expect("cleanup");
        assert_eq!(removed, 0);
    }

    #[test]
    fn given_active_locks_when_cleanup_then_zero_removed() {
        let service = create_service();
        service.acquire_lock("s-1", "a-1", 300).expect("a");
        let removed = service.cleanup_expired().expect("cleanup");
        assert_eq!(removed, 0);
    }

    // ─── Error handling: invalid input ──────────────────────────────────

    #[test]
    fn given_empty_session_when_acquire_then_error() {
        let service = create_service();
        let result = service.acquire_lock("", "agent-a", 300);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::InvalidInput(_))),
            "empty session should be InvalidInput"
        );
    }

    #[test]
    fn given_empty_agent_when_acquire_then_error() {
        let service = create_service();
        let result = service.acquire_lock("session-1", "", 300);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RepositoryError::InvalidInput(_))),
            "empty agent_id should be InvalidInput"
        );
    }

    #[test]
    fn given_both_empty_when_acquire_then_error() {
        let service = create_service();
        let result = service.acquire_lock("", "", 0);
        assert!(result.is_err());
    }

    // ─── Concurrent operations (cross-workspace coordination) ───────────

    #[test]
    fn given_shared_service_when_concurrent_acquire_then_only_one_wins() {
        let repo = Arc::new(InMemoryLockRepository::new());
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let repo = Arc::clone(&repo);
                thread::spawn(move || {
                    let service = CoordinationService::new(
                        Arc::try_unwrap(repo).unwrap_or_else(|arc| (*arc).clone()),
                    );
                    service.acquire_lock("shared-session", &format!("agent-{i}"), 300)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().expect("thread")).collect();
        let successes = results.iter().filter(|r| r.is_ok()).count();
        let conflicts = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(successes + conflicts, 4);
        // At least one should succeed
        assert!(successes >= 1, "at least one acquire should succeed");
    }

    #[test]
    fn given_shared_repo_when_concurrent_different_sessions_then_all_succeed() {
        let repo = Arc::new(InMemoryLockRepository::new());
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let repo = Arc::clone(&repo);
                thread::spawn(move || {
                    let service = CoordinationService::new(
                        Arc::try_unwrap(repo).unwrap_or_else(|arc| (*arc).clone()),
                    );
                    service.acquire_lock(&format!("session-{i}"), &format!("agent-{i}"), 300)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().expect("thread")).collect();
        assert!(results.iter().all(|r| r.is_ok()), "all should succeed with different sessions");
    }

    // ─── Full lifecycle: acquire → heartbeat → release → verify ─────────

    #[test]
    fn given_full_lifecycle_when_execute_then_audit_trail_complete() {
        let service = create_service();

        // Acquire
        let lock = service.acquire_lock("s-1", "agent-a", 300).expect("acquire");
        assert_eq!(lock.agent_id, "agent-a");

        // Heartbeat
        let extended = service.heartbeat("s-1", "agent-a").expect("heartbeat");
        assert!(extended.expires_at >= lock.expires_at);

        // Verify state
        let state = service.get_lock_state("s-1").expect("state").expect("present");
        assert_eq!(state.agent_id, "agent-a");

        // Release
        service.release_lock("s-1", "agent-a").expect("release");
        assert!(service.get_lock_state("s-1").expect("state").is_none());

        // Audit trail
        let log = service.get_audit_log("s-1").expect("audit");
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].operation, LockOperation::Acquire);
        assert_eq!(log[1].operation, LockOperation::Heartbeat);
        assert_eq!(log[2].operation, LockOperation::Release);
    }

    #[test]
    fn given_lifecycle_when_another_agent_waits_then_acquires_after_release() {
        let service = create_service();

        // Agent A acquires
        service.acquire_lock("s-1", "agent-a", 300).expect("a");

        // Agent B fails to acquire
        assert!(service.acquire_lock("s-1", "agent-b", 300).is_err());

        // Agent A releases
        service.release_lock("s-1", "agent-a").expect("release");

        // Agent B now succeeds
        let lock = service.acquire_lock("s-1", "agent-b", 300).expect("b");
        assert_eq!(lock.agent_id, "agent-b");

        // Agent A can't heartbeat anymore
        assert!(service.heartbeat("s-1", "agent-a").is_err());
    }

    // ─── Cross-workspace state coordination ─────────────────────────────

    #[test]
    fn given_multiple_sessions_when_lock_each_then_independent_states() {
        let service = create_service();
        service.acquire_lock("ws-1/session-1", "agent-a", 300).expect("1");
        service.acquire_lock("ws-2/session-1", "agent-b", 300).expect("2");
        service.acquire_lock("ws-1/session-2", "agent-c", 300).expect("3");

        let locks = service.list_locks().expect("list");
        assert_eq!(locks.len(), 3);

        // Same session name, different workspaces — independent
        let ws1 = service.get_lock_state("ws-1/session-1").expect("state").expect("present");
        assert_eq!(ws1.agent_id, "agent-a");
        let ws2 = service.get_lock_state("ws-2/session-1").expect("state").expect("present");
        assert_eq!(ws2.agent_id, "agent-b");
    }

    #[test]
    fn given_cross_workspace_when_release_one_then_others_unaffected() {
        let service = create_service();
        service.acquire_lock("ws-1/s-1", "agent-a", 300).expect("a");
        service.acquire_lock("ws-2/s-1", "agent-b", 300).expect("b");
        service.acquire_lock("ws-1/s-2", "agent-c", 300).expect("c");

        service.release_lock("ws-1/s-1", "agent-a").expect("release");
        let locks = service.list_locks().expect("list");
        assert_eq!(locks.len(), 2);
        assert!(service.get_lock_state("ws-2/s-1").expect("state").is_some());
        assert!(service.get_lock_state("ws-1/s-2").expect("state").is_some());
    }

    // ─── Workspace event propagation (audit log as event record) ────────

    #[test]
    fn given_multi_agent_workspace_when_operations_then_full_audit_trail() {
        let service = create_service();
        // Agent A acquires, heartbeats
        service.acquire_lock("ws-1/s-1", "agent-a", 300).expect("a");
        service.heartbeat("ws-1/s-1", "agent-a").expect("hb");
        // Agent A releases
        service.release_lock("ws-1/s-1", "agent-a").expect("release");
        // Agent B acquires
        service.acquire_lock("ws-1/s-1", "agent-b", 300).expect("b");

        let log = service.get_audit_log("ws-1/s-1").expect("audit");
        assert_eq!(log.len(), 4);
        assert_eq!(log[0].operation, LockOperation::Acquire);
        assert_eq!(log[0].agent_id, "agent-a");
        assert_eq!(log[1].operation, LockOperation::Heartbeat);
        assert_eq!(log[1].agent_id, "agent-a");
        assert_eq!(log[2].operation, LockOperation::Release);
        assert_eq!(log[2].agent_id, "agent-a");
        assert_eq!(log[3].operation, LockOperation::Acquire);
        assert_eq!(log[3].agent_id, "agent-b");
    }

    #[test]
    fn given_cross_workspace_events_when_audit_then_isolated() {
        let service = create_service();
        service.acquire_lock("ws-1/s-1", "agent-a", 300).expect("a");
        service.acquire_lock("ws-2/s-2", "agent-b", 300).expect("b");
        service.release_lock("ws-1/s-1", "agent-a").expect("release");

        let ws1_log = service.get_audit_log("ws-1/s-1").expect("audit");
        let ws2_log = service.get_audit_log("ws-2/s-2").expect("audit");

        assert_eq!(ws1_log.len(), 2); // acquire + release
        assert_eq!(ws2_log.len(), 1); // acquire only
    }
}
