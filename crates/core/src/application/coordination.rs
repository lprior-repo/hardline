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

    fn create_service() -> CoordinationService<InMemoryLockRepository> {
        CoordinationService::new(InMemoryLockRepository::new())
    }

    #[test]
    fn given_service_when_acquire_lock_then_success() {
        let service = create_service();
        let lock = service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("acquire");
        assert_eq!(lock.session, "session-1");
        assert_eq!(lock.agent_id, "agent-a");
    }

    #[test]
    fn given_locked_when_acquire_by_another_then_conflict() {
        let service = create_service();
        service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("first");
        let result = service.acquire_lock("session-1", "agent-b", 300);
        assert!(result.is_err());
    }

    #[test]
    fn given_lock_when_release_then_reacquire() {
        let service = create_service();
        service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("acquire");
        service
            .release_lock("session-1", "agent-a")
            .expect("release");
        let lock = service
            .acquire_lock("session-1", "agent-b", 300)
            .expect("reacquire");
        assert_eq!(lock.agent_id, "agent-b");
    }

    #[test]
    fn given_lock_when_heartbeat_then_extended() {
        let service = create_service();
        let original = service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("acquire");
        let extended = service
            .heartbeat("session-1", "agent-a")
            .expect("heartbeat");
        assert!(extended.expires_at >= original.expires_at);
    }

    #[test]
    fn given_multiple_locks_when_list_then_returns_all() {
        let service = create_service();
        service.acquire_lock("s-1", "a-1", 300).expect("a");
        service.acquire_lock("s-2", "a-2", 300).expect("b");
        let locks = service.list_locks().expect("list");
        assert_eq!(locks.len(), 2);
    }

    #[test]
    fn given_no_lock_when_get_state_then_none() {
        let service = create_service();
        assert!(service
            .get_lock_state("session-1")
            .expect("state")
            .is_none());
    }

    #[test]
    fn given_operations_when_audit_log_then_entries() {
        let service = create_service();
        service
            .acquire_lock("session-1", "agent-a", 300)
            .expect("acquire");
        service
            .release_lock("session-1", "agent-a")
            .expect("release");
        let log = service.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].operation, LockOperation::Acquire);
        assert_eq!(log[1].operation, LockOperation::Release);
    }

    #[test]
    fn given_empty_session_when_acquire_then_error() {
        let service = create_service();
        let result = service.acquire_lock("", "agent-a", 300);
        assert!(result.is_err());
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
}
