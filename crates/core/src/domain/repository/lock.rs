//! Lock repository trait and in-memory implementation.
//!
//! Provides the repository interface for distributed lock operations,
//! following the same sync + Mutex pattern as QueueRepository.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::error::{RepositoryError, RepositoryResult};

/// Distributed lock entry.
#[derive(Debug, Clone)]
pub struct Lock {
    /// Unique lock identifier
    pub lock_id: String,
    /// Session this lock protects
    pub session: String,
    /// Agent holding the lock
    pub agent_id: String,
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    /// When the lock expires
    pub expires_at: DateTime<Utc>,
}

/// Audit log entry for lock operations.
#[derive(Debug, Clone)]
pub struct LockAudit {
    /// Session name
    pub session: String,
    /// Agent that performed the operation
    pub agent_id: String,
    /// Type of operation
    pub operation: LockOperation,
    /// When the operation occurred
    pub timestamp: DateTime<Utc>,
}

/// Type of lock operation for audit logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOperation {
    /// Lock acquired
    Acquire,
    /// Lock released
    Release,
    /// Lock TTL extended via heartbeat
    Heartbeat,
    /// Double-unlock warning
    DoubleUnlockWarning,
}

/// Repository for distributed lock operations.
///
/// Provides lock lifecycle management with TTL-based expiration,
/// heartbeat extension, and audit logging.
pub trait LockRepository: Send + Sync {
    /// Acquire an exclusive lock on a session.
    ///
    /// Returns the lock if successful, or an error if:
    /// - Session is already locked by another agent (`Conflict`)
    /// - Invalid input (`InvalidInput`)
    ///
    /// If `ttl_seconds` is 0, uses the repository's default TTL (300s).
    fn acquire(
        &self,
        session: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> RepositoryResult<Lock>;

    /// Release a lock held by the caller.
    ///
    /// Returns an error if agent is not the lock holder (`Conflict`).
    /// Logs a double-unlock warning if no active lock exists.
    fn release(&self, session: &str, agent_id: &str) -> RepositoryResult<()>;

    /// Extend lock TTL via heartbeat.
    ///
    /// Returns the updated lock with new expiration.
    /// Returns an error if agent is not the holder (`Conflict`)
    /// or no active lock exists (`NotFound`).
    fn heartbeat(&self, session: &str, agent_id: &str) -> RepositoryResult<Lock>;

    /// Get all active (non-expired) locks.
    fn list_active(&self) -> RepositoryResult<Vec<Lock>>;

    /// Get current lock for a session.
    ///
    /// Returns `None` if no active lock exists.
    fn get_state(&self, session: &str) -> RepositoryResult<Option<Lock>>;

    /// Get audit log entries for a session.
    fn get_audit_log(&self, session: &str) -> RepositoryResult<Vec<LockAudit>>;

    /// Remove all expired locks.
    ///
    /// Returns the number of locks removed.
    fn cleanup_expired(&self) -> RepositoryResult<u64>;

    /// Initialize the lock schema (tables, indexes).
    fn init_schema(&self) -> RepositoryResult<()> {
        Ok(())
    }
}

/// In-memory lock repository for testing.
///
/// Uses `Mutex<HashMap>` for interior mutability, matching the
/// `InMemoryQueueRepository` pattern.
pub struct InMemoryLockRepository {
    locks: Arc<Mutex<HashMap<String, Lock>>>,
    audit: Arc<Mutex<Vec<LockAudit>>>,
    default_ttl: chrono::Duration,
}

impl InMemoryLockRepository {
    /// Create a new in-memory repository with default TTL (300s).
    #[must_use]
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
            audit: Arc::new(Mutex::new(Vec::new())),
            default_ttl: chrono::Duration::seconds(300),
        }
    }

    /// Create with custom default TTL.
    #[must_use]
    pub fn with_ttl(ttl: chrono::Duration) -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
            audit: Arc::new(Mutex::new(Vec::new())),
            default_ttl: ttl,
        }
    }

    fn log_audit(&self, session: &str, agent_id: &str, op: LockOperation) {
        let entry = LockAudit {
            session: session.to_string(),
            agent_id: agent_id.to_string(),
            operation: op,
            timestamp: Utc::now(),
        };
        if let Ok(mut audit) = self.audit.lock() {
            audit.push(entry);
        }
    }
}

impl Default for InMemoryLockRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryLockRepository {
    fn clone(&self) -> Self {
        let locks = self
            .locks
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default();
        let audit = self
            .audit
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default();
        Self {
            locks: Arc::new(Mutex::new(locks)),
            audit: Arc::new(Mutex::new(audit)),
            default_ttl: self.default_ttl,
        }
    }
}

impl LockRepository for InMemoryLockRepository {
    fn acquire(
        &self,
        session: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> RepositoryResult<Lock> {
        if session.is_empty() {
            return Err(RepositoryError::InvalidInput(
                "session cannot be empty".to_string(),
            ));
        }
        if agent_id.is_empty() {
            return Err(RepositoryError::InvalidInput(
                "agent_id cannot be empty".to_string(),
            ));
        }

        let mut locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

        let now = Utc::now();

        // Check for existing active lock
        if let Some(existing) = locks.get(session) {
            if existing.expires_at >= now {
                if existing.agent_id == agent_id {
                    // Same agent: return existing lock (idempotent)
                    self.log_audit(session, agent_id, LockOperation::Acquire);
                    return Ok(existing.clone());
                }
                return Err(RepositoryError::Conflict(format!(
                    "Session '{}' is locked by '{}'",
                    session, existing.agent_id
                )));
            }
            // Expired: remove it
            locks.remove(session);
        }

        let ttl = if ttl_seconds > 0 {
            chrono::Duration::seconds(ttl_seconds as i64)
        } else {
            self.default_ttl
        };

        let lock = Lock {
            lock_id: format!("lock-{session}-{}", now.timestamp_nanos_opt().unwrap_or(0)),
            session: session.to_string(),
            agent_id: agent_id.to_string(),
            acquired_at: now,
            expires_at: now + ttl,
        };

        locks.insert(session.to_string(), lock.clone());
        drop(locks);

        self.log_audit(session, agent_id, LockOperation::Acquire);
        Ok(lock)
    }

    fn release(&self, session: &str, agent_id: &str) -> RepositoryResult<()> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

        let now = Utc::now();

        match locks.get(session) {
            Some(existing) if existing.expires_at >= now => {
                if existing.agent_id == agent_id {
                    locks.remove(session);
                    drop(locks);
                    self.log_audit(session, agent_id, LockOperation::Release);
                    Ok(())
                } else {
                    Err(RepositoryError::Conflict(format!(
                        "Agent '{}' does not hold lock on session '{}'",
                        agent_id, session
                    )))
                }
            }
            _ => {
                // No active lock: log double-unlock warning
                drop(locks);
                self.log_audit(session, agent_id, LockOperation::DoubleUnlockWarning);
                Ok(())
            }
        }
    }

    fn heartbeat(&self, session: &str, agent_id: &str) -> RepositoryResult<Lock> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

        let now = Utc::now();

        match locks.get_mut(session) {
            Some(existing) if existing.expires_at >= now => {
                if existing.agent_id == agent_id {
                    existing.expires_at = now + self.default_ttl;
                    let updated = existing.clone();
                    drop(locks);
                    self.log_audit(session, agent_id, LockOperation::Heartbeat);
                    Ok(updated)
                } else {
                    Err(RepositoryError::Conflict(format!(
                        "Agent '{}' does not hold lock on session '{}'",
                        agent_id, session
                    )))
                }
            }
            _ => Err(RepositoryError::NotFound(format!(
                "No active lock for session '{session}'"
            ))),
        }
    }

    fn list_active(&self) -> RepositoryResult<Vec<Lock>> {
        let locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;
        let now = Utc::now();
        Ok(locks
            .values()
            .filter(|l| l.expires_at >= now)
            .cloned()
            .collect())
    }

    fn get_state(&self, session: &str) -> RepositoryResult<Option<Lock>> {
        let locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;
        let now = Utc::now();
        Ok(locks
            .get(session)
            .filter(|l| l.expires_at >= now)
            .cloned())
    }

    fn get_audit_log(&self, session: &str) -> RepositoryResult<Vec<LockAudit>> {
        let audit = self
            .audit
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;
        Ok(audit
            .iter()
            .filter(|e| e.session == session)
            .cloned()
            .collect())
    }

    fn cleanup_expired(&self) -> RepositoryResult<u64> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;
        let now = Utc::now();
        let before = locks.len();
        locks.retain(|_, l| l.expires_at >= now);
        let removed = before - locks.len();
        Ok(removed as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_repo_when_acquire_then_lock_returned() {
        let repo = InMemoryLockRepository::new();
        let lock = repo.acquire("session-1", "agent-a", 300).expect("acquire");
        assert_eq!(lock.session, "session-1");
        assert_eq!(lock.agent_id, "agent-a");
    }

    #[test]
    fn given_locked_session_when_acquire_by_another_then_conflict() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("first");
        let result = repo.acquire("session-1", "agent-b", 300);
        assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    }

    #[test]
    fn given_same_agent_when_acquire_then_idempotent() {
        let repo = InMemoryLockRepository::new();
        let first = repo.acquire("session-1", "agent-a", 300).expect("first");
        let second = repo.acquire("session-1", "agent-a", 300).expect("second");
        assert_eq!(first.lock_id, second.lock_id);
    }

    #[test]
    fn given_lock_when_release_then_can_reacquire() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        repo.release("session-1", "agent-a").expect("release");
        let lock = repo.acquire("session-1", "agent-b", 300).expect("reacquire");
        assert_eq!(lock.agent_id, "agent-b");
    }

    #[test]
    fn given_non_holder_when_release_then_conflict() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        let result = repo.release("session-1", "agent-b");
        assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    }

    #[test]
    fn given_no_lock_when_release_then_double_unlock_warning() {
        let repo = InMemoryLockRepository::new();
        repo.release("session-1", "agent-a").expect("double unlock ok");
        let log = repo.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].operation, LockOperation::DoubleUnlockWarning);
    }

    #[test]
    fn given_lock_when_heartbeat_then_expiration_extended() {
        let repo = InMemoryLockRepository::new();
        let original = repo.acquire("session-1", "agent-a", 300).expect("acquire");
        let extended = repo.heartbeat("session-1", "agent-a").expect("heartbeat");
        assert!(extended.expires_at >= original.expires_at);
    }

    #[test]
    fn given_non_holder_when_heartbeat_then_conflict() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        let result = repo.heartbeat("session-1", "agent-b");
        assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    }

    #[test]
    fn given_no_lock_when_heartbeat_then_not_found() {
        let repo = InMemoryLockRepository::new();
        let result = repo.heartbeat("session-1", "agent-a");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn given_multiple_locks_when_list_active_then_returns_all() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("a");
        repo.acquire("session-2", "agent-b", 300).expect("b");
        let locks = repo.list_active().expect("list");
        assert_eq!(locks.len(), 2);
    }

    #[test]
    fn given_no_lock_when_get_state_then_none() {
        let repo = InMemoryLockRepository::new();
        assert!(repo.get_state("session-1").expect("state").is_none());
    }

    #[test]
    fn given_lock_when_get_state_then_some() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        let state = repo.get_state("session-1").expect("state");
        assert!(state.is_some());
        assert_eq!(state.map_or(String::new(), |l| l.agent_id), "agent-a");
    }

    #[test]
    fn given_operations_when_get_audit_log_then_returns_entries() {
        let repo = InMemoryLockRepository::new();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        repo.release("session-1", "agent-a").expect("release");
        let log = repo.get_audit_log("session-1").expect("audit");
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].operation, LockOperation::Acquire);
        assert_eq!(log[1].operation, LockOperation::Release);
    }

    #[test]
    fn given_empty_session_when_acquire_then_invalid_input() {
        let repo = InMemoryLockRepository::new();
        let result = repo.acquire("", "agent-a", 300);
        assert!(matches!(result, Err(RepositoryError::InvalidInput(_))));
    }

    #[test]
    fn given_empty_agent_when_acquire_then_invalid_input() {
        let repo = InMemoryLockRepository::new();
        let result = repo.acquire("session-1", "", 300);
        assert!(matches!(result, Err(RepositoryError::InvalidInput(_))));
    }

    #[test]
    fn given_clone_when_operations_then_independent() {
        let repo = InMemoryLockRepository::new();
        let cloned = repo.clone();
        repo.acquire("session-1", "agent-a", 300).expect("acquire");
        // Clone has independent state
        assert!(cloned.get_state("session-1").expect("state").is_none());
    }

    #[test]
    fn given_default_impl_when_init_schema_then_ok() {
        let repo = InMemoryLockRepository::new();
        assert!(repo.init_schema().is_ok());
    }
}
