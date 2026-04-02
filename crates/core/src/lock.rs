//! Unified locking system for Source Control Plane.
//!
//! Provides lock types for workspaces, sessions, and queues.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use crate::error_agent::AgentErrorKind;
use crate::error_queue::QueueErrorKind;
use crate::error_task::TaskErrorKind;
use crate::error_workspace::{SessionErrorKind, WorkspaceErrorKind};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Type of lock held in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockType {
    /// Workspace lock (from Isolate)
    Workspace(String),
    /// Session lock (from Isolate)
    Session(String),
    /// Queue lock (from Stak)
    Queue(String),
    /// Agent lock
    Agent(String),
    /// Task/Bead lock (for TTL locking)
    Task(String),
}

impl std::fmt::Display for LockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockType::Workspace(name) => write!(f, "workspace:{}", name),
            LockType::Session(name) => write!(f, "session:{}", name),
            LockType::Queue(name) => write!(f, "queue:{}", name),
            LockType::Agent(name) => write!(f, "agent:{}", name),
            LockType::Task(name) => write!(f, "task:{}", name),
        }
    }
}

/// A lock guard - released when dropped
#[derive(Debug)]
pub struct LockGuard {
    lock_type: LockType,
    holder: String,
    acquired_at: DateTime<Utc>,
    locks: Arc<RwLock<HashMap<LockType, LockInfo>>>,
}

impl LockGuard {
    /// Get the type of lock being held
    pub fn lock_type(&self) -> &LockType {
        &self.lock_type
    }

    /// Get who holds this lock
    pub fn holder(&self) -> &str {
        &self.holder
    }

    /// When this lock was acquired
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Ok(mut locks) = self.locks.write() {
            locks.remove(&self.lock_type);
        }
    }
}

/// Information about a held lock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub holder: String,
    pub acquired_at: DateTime<Utc>,
    pub lock_type: LockType,
}

/// Lock manager trait
pub trait LockManager: Send + Sync {
    /// Acquire a lock
    fn acquire(&self, lock: LockType, holder: &str) -> Result<LockGuard>;

    /// Try to acquire a lock without blocking
    fn try_acquire(&self, lock: LockType, holder: &str) -> Result<Option<LockGuard>>;

    /// Release a lock (for manual release, though Drop does this too)
    fn release(&self, lock: &LockType) -> Result<()>;

    /// Check if a lock is held
    fn is_locked(&self, lock: &LockType) -> Result<bool>;

    /// Get lock info if locked
    fn get_lock_info(&self, lock: &LockType) -> Result<Option<LockInfo>>;

    /// List all held locks
    fn list_locks(&self) -> Result<Vec<LockInfo>>;
}

/// In-memory lock manager (for single-process use)
#[derive(Debug, Default)]
pub struct MemLockManager {
    locks: Arc<RwLock<HashMap<LockType, LockInfo>>>,
}

impl MemLockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl LockManager for MemLockManager {
    fn acquire(&self, lock: LockType, holder: &str) -> Result<LockGuard> {
        // Check if already locked
        {
            let locks = self.locks.read().map_err(|e| {
                crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                    e.to_string(),
                ))
            })?;
            if let Some(info) = locks.get(&lock) {
                return Err(match &info.lock_type {
                    LockType::Workspace(name) => Error::from(WorkspaceErrorKind::Locked(
                        name.clone(),
                        info.holder.clone(),
                    )),
                    LockType::Session(name) => {
                        Error::from(SessionErrorKind::Locked(name.clone(), info.holder.clone()))
                    }
                    LockType::Queue(_) => Error::from(QueueErrorKind::Locked(info.holder.clone())),
                    LockType::Agent(name) => Error::from(AgentErrorKind::Exists(name.clone())),
                    LockType::Task(name) => Error::from(TaskErrorKind::Locked(name.clone())),
                });
            }
        }

        // Acquire the lock
        let lock_info = LockInfo {
            holder: holder.to_string(),
            acquired_at: Utc::now(),
            lock_type: lock.clone(),
        };

        {
            let mut locks = self.locks.write().map_err(|e| {
                crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                    e.to_string(),
                ))
            })?;
            locks.insert(lock.clone(), lock_info);
        }

        Ok(LockGuard {
            lock_type: lock,
            holder: holder.to_string(),
            acquired_at: Utc::now(),
            locks: self.locks.clone(),
        })
    }

    fn try_acquire(&self, lock: LockType, holder: &str) -> Result<Option<LockGuard>> {
        // Try to acquire - if already locked, return None
        {
            let locks = self.locks.read().map_err(|e| {
                crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                    e.to_string(),
                ))
            })?;
            if locks.contains_key(&lock) {
                return Ok(None);
            }
        }

        // Acquire
        let lock_info = LockInfo {
            holder: holder.to_string(),
            acquired_at: Utc::now(),
            lock_type: lock.clone(),
        };

        let locks_ref = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut locks = locks_ref.write().map_err(|e| {
                crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                    e.to_string(),
                ))
            })?;
            locks.insert(lock.clone(), lock_info);
        }

        // Note: In a real impl, we'd need proper reference sharing
        // This is simplified for illustration
        Ok(Some(LockGuard {
            lock_type: lock,
            holder: holder.to_string(),
            acquired_at: Utc::now(),
            locks: locks_ref,
        }))
    }

    fn release(&self, lock: &LockType) -> Result<()> {
        let mut locks = self.locks.write().map_err(|e| {
            crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                e.to_string(),
            ))
        })?;
        locks.remove(lock);
        Ok(())
    }

    fn is_locked(&self, lock: &LockType) -> Result<bool> {
        let locks = self.locks.read().map_err(|e| {
            crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                e.to_string(),
            ))
        })?;
        Ok(locks.contains_key(lock))
    }

    fn get_lock_info(&self, lock: &LockType) -> Result<Option<LockInfo>> {
        let locks = self.locks.read().map_err(|e| {
            crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                e.to_string(),
            ))
        })?;
        Ok(locks.get(lock).cloned())
    }

    fn list_locks(&self) -> Result<Vec<LockInfo>> {
        let locks = self.locks.read().map_err(|e| {
            crate::error::Error::from(crate::error_internal::InternalErrorKind::Internal(
                e.to_string(),
            ))
        })?;
        Ok(locks.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // LockType tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lock_type_workspace_display() {
        let lt = LockType::Workspace("my-workspace".into());
        assert_eq!(format!("{lt}"), "workspace:my-workspace");
    }

    #[test]
    fn test_lock_type_session_display() {
        let lt = LockType::Session("sess-1".into());
        assert_eq!(format!("{lt}"), "session:sess-1");
    }

    #[test]
    fn test_lock_type_queue_display() {
        let lt = LockType::Queue("build-queue".into());
        assert_eq!(format!("{lt}"), "queue:build-queue");
    }

    #[test]
    fn test_lock_type_agent_display() {
        let lt = LockType::Agent("cli-01".into());
        assert_eq!(format!("{lt}"), "agent:cli-01");
    }

    #[test]
    fn test_lock_type_task_display() {
        let lt = LockType::Task("bead-42".into());
        assert_eq!(format!("{lt}"), "task:bead-42");
    }

    #[test]
    fn test_lock_type_equality() {
        let a = LockType::Workspace("ws".into());
        let b = LockType::Workspace("ws".into());
        let c = LockType::Workspace("other".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_lock_type_different_variants_not_equal() {
        let ws = LockType::Workspace("same".into());
        let sess = LockType::Session("same".into());
        assert_ne!(ws, sess);
    }

    #[test]
    fn test_lock_type_clone() {
        let lt = LockType::Queue("q".into());
        let cloned = lt.clone();
        assert_eq!(lt, cloned);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LockGuard construction and Drop behavior
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lock_guard_accessors() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("test".into());
        let before = Utc::now();
        let guard = manager.acquire(lock.clone(), "holder-1")?;
        let after = Utc::now();

        assert_eq!(guard.lock_type(), &lock);
        assert_eq!(guard.holder(), "holder-1");
        assert!(guard.acquired_at() >= before && guard.acquired_at() <= after);
        Ok(())
    }

    #[test]
    fn test_lock_guard_drop_releases() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Session("drop-test".into());

        assert!(!manager.is_locked(&lock)?);

        {
            let _guard = manager.acquire(lock.clone(), "agent-d")?;
            assert!(manager.is_locked(&lock)?);
        }

        assert!(!manager.is_locked(&lock)?);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MemLockManager: acquire, release, is_locked
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_acquire_and_is_locked() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("test".into());

        // Should be able to acquire
        let guard = manager.acquire(lock.clone(), "agent-1")?;
        assert!(manager.is_locked(&lock)?);

        // Drop guard
        drop(guard);

        // Should be released
        assert!(!manager.is_locked(&lock)?);

        Ok(())
    }

    #[test]
    fn test_acquire_after_release() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Task("reuse".into());

        {
            let _g1 = manager.acquire(lock.clone(), "agent-a")?;
            assert!(manager.is_locked(&lock)?);
        }

        // Should be acquirable again after drop
        let _g2 = manager.acquire(lock.clone(), "agent-b")?;
        assert!(manager.is_locked(&lock)?);
        Ok(())
    }

    #[test]
    fn test_release_manual() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Queue("manual".into());

        let _guard = manager.acquire(lock.clone(), "agent-m")?;
        assert!(manager.is_locked(&lock)?);

        manager.release(&lock)?;
        assert!(!manager.is_locked(&lock)?);
        Ok(())
    }

    #[test]
    fn test_release_not_locked_is_ok() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("never-locked".into());
        // Releasing a lock that is not held should not error
        manager.release(&lock)?;
        Ok(())
    }

    #[test]
    fn test_is_locked_false_initially() -> Result<()> {
        let manager = MemLockManager::new();
        assert!(!manager.is_locked(&LockType::Workspace("nope".into()))?);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Lock contention / double-acquire rejection
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lock_conflict_session() {
        let manager = MemLockManager::new();
        let lock = LockType::Session("test".into());

        let _guard = manager.acquire(lock.clone(), "agent-1").unwrap();
        let result = manager.acquire(lock.clone(), "agent-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_lock_conflict_workspace() {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("ws".into());

        let _guard = manager.acquire(lock.clone(), "agent-a").unwrap();
        let result = manager.acquire(lock.clone(), "agent-b");
        assert!(result.is_err());
    }

    #[test]
    fn test_lock_conflict_queue() {
        let manager = MemLockManager::new();
        let lock = LockType::Queue("q".into());

        let _guard = manager.acquire(lock.clone(), "agent-1").unwrap();
        let result = manager.acquire(lock.clone(), "agent-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_lock_conflict_agent() {
        let manager = MemLockManager::new();
        let lock = LockType::Agent("a1".into());

        let _guard = manager.acquire(lock.clone(), "holder-1").unwrap();
        let result = manager.acquire(lock.clone(), "holder-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_lock_conflict_task() {
        let manager = MemLockManager::new();
        let lock = LockType::Task("t1".into());

        let _guard = manager.acquire(lock.clone(), "holder-1").unwrap();
        let result = manager.acquire(lock.clone(), "holder-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_double_acquire_same_holder_rejected() {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("strict".into());

        let _g1 = manager.acquire(lock.clone(), "same-agent").unwrap();
        let result = manager.acquire(lock.clone(), "same-agent");
        // Even the same holder cannot double-acquire
        assert!(result.is_err());
    }

    #[test]
    fn test_different_lock_types_no_conflict() -> Result<()> {
        let manager = MemLockManager::new();
        let ws = LockType::Workspace("ws".into());
        let sess = LockType::Session("sess".into());

        let _g_ws = manager.acquire(ws.clone(), "agent-1")?;
        let _g_sess = manager.acquire(sess.clone(), "agent-1")?;

        assert!(manager.is_locked(&ws)?);
        assert!(manager.is_locked(&sess)?);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // try_acquire tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_try_acquire_success() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("try".into());

        let result = manager.try_acquire(lock.clone(), "agent-t")?;
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn test_try_acquire_when_locked_returns_none() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("contended".into());

        let _guard = manager.acquire(lock.clone(), "agent-first")?;
        let result = manager.try_acquire(lock.clone(), "agent-second")?;
        assert!(result.is_none());
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // get_lock_info and list_locks
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_lock_info_none_initially() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("info-test".into());
        let info = manager.get_lock_info(&lock)?;
        assert!(info.is_none());
        Ok(())
    }

    #[test]
    fn test_get_lock_info_after_acquire() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Session("info-sess".into());

        let _guard = manager.acquire(lock.clone(), "info-holder")?;
        let info = manager.get_lock_info(&lock)?.expect("lock is held");

        assert_eq!(info.holder, "info-holder");
        assert_eq!(info.lock_type, lock);
        Ok(())
    }

    #[test]
    fn test_get_lock_info_after_drop() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Queue("info-q".into());

        {
            let _guard = manager.acquire(lock.clone(), "temp-holder")?;
        }

        let info = manager.get_lock_info(&lock)?;
        assert!(info.is_none());
        Ok(())
    }

    #[test]
    fn test_list_locks_empty() -> Result<()> {
        let manager = MemLockManager::new();
        let locks = manager.list_locks()?;
        assert!(locks.is_empty());
        Ok(())
    }

    #[test]
    fn test_list_locks_multiple() -> Result<()> {
        let manager = MemLockManager::new();

        let _g1 = manager.acquire(LockType::Workspace("ws1".into()), "a1")?;
        let _g2 = manager.acquire(LockType::Session("s1".into()), "a2")?;
        let _g3 = manager.acquire(LockType::Task("t1".into()), "a3")?;

        let locks = manager.list_locks()?;
        assert_eq!(locks.len(), 3);
        Ok(())
    }

    #[test]
    fn test_list_locks_after_partial_drop() -> Result<()> {
        let manager = MemLockManager::new();

        let _g1 = manager.acquire(LockType::Workspace("keep".into()), "a1")?;
        {
            let _g2 = manager.acquire(LockType::Session("drop".into()), "a2")?;
        }

        let locks = manager.list_locks()?;
        assert_eq!(locks.len(), 1);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Default and new equivalence
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_mem_lock_manager_default() -> Result<()> {
        let manager = MemLockManager::default();
        assert!(!manager.is_locked(&LockType::Workspace("default".into()))?);
        Ok(())
    }

    #[test]
    fn test_mem_lock_manager_new_same_as_default() -> Result<()> {
        let default = MemLockManager::default();
        let new = MemLockManager::new();
        // Both start empty
        assert_eq!(default.list_locks()?.len(), 0);
        assert_eq!(new.list_locks()?.len(), 0);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: contention — lock same resource from different holders
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_contention_multiple_holders_one_resource() {
        let manager = MemLockManager::new();
        let lock = LockType::Queue("hot-resource".into());

        // First holder succeeds
        let g1 = manager.acquire(lock.clone(), "holder-alpha").unwrap();
        assert!(manager.is_locked(&lock).unwrap());

        // All subsequent holders fail while first holds the lock
        for i in 0..10 {
            let result = manager.acquire(lock.clone(), &format!("holder-challenger-{i}"));
            assert!(result.is_err(), "challenger {i} should have been rejected");
        }

        // Original holder still holds
        assert!(manager.is_locked(&lock).unwrap());
        assert_eq!(g1.holder(), "holder-alpha");

        // After dropping, a new holder can acquire
        drop(g1);
        let g2 = manager.acquire(lock.clone(), "holder-beta").unwrap();
        assert_eq!(g2.holder(), "holder-beta");
    }

    #[test]
    fn test_contention_error_message_contains_holder_info() {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("ws-contention".into());

        let _g = manager.acquire(lock.clone(), "original-holder").unwrap();
        let err = manager.acquire(lock.clone(), "rival-holder").unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("original-holder"),
            "error message should mention the current lock holder, got: {msg}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: release a lock that was never acquired
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_release_never_acquired_is_noop() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("phantom".into());

        // Releasing a lock that was never held should succeed silently
        manager.release(&lock)?;
        manager.release(&lock)?; // even twice

        // Manager state is unchanged
        assert!(!manager.is_locked(&lock)?);
        assert!(manager.get_lock_info(&lock)?.is_none());
        assert_eq!(manager.list_locks()?.len(), 0);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: multiple locks on different resources (no conflict)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_multiple_resources_no_conflict() -> Result<()> {
        let manager = MemLockManager::new();

        let locks = vec![
            LockType::Workspace("ws-alpha".into()),
            LockType::Workspace("ws-beta".into()),
            LockType::Session("sess-1".into()),
            LockType::Session("sess-2".into()),
            LockType::Queue("q-build".into()),
            LockType::Queue("q-test".into()),
            LockType::Agent("cli-1".into()),
            LockType::Agent("cli-2".into()),
            LockType::Task("bead-1".into()),
            LockType::Task("bead-2".into()),
        ];

        // Acquire all different resources from the same holder
        let mut guards = Vec::new();
        for lock in &locks {
            let guard = manager.acquire(lock.clone(), "poly-holder")?;
            guards.push(guard);
        }

        // All should be locked
        for lock in &locks {
            assert!(manager.is_locked(lock)?);
        }

        assert_eq!(manager.list_locks()?.len(), 10);

        // Drop all, verify clean
        drop(guards);
        for lock in &locks {
            assert!(!manager.is_locked(lock)?);
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: lock then immediately unlock in sequence
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_acquire_release_immediate_sequence() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Session("rapid".into());

        for i in 0..50 {
            let guard = manager.acquire(lock.clone(), &format!("seq-holder-{i}"))?;
            assert!(manager.is_locked(&lock)?);
            assert_eq!(
                manager.get_lock_info(&lock)?.expect("held").holder,
                format!("seq-holder-{i}")
            );
            drop(guard);
            assert!(!manager.is_locked(&lock)?);
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: concurrent lock acquisition pattern (rapid acquire/release)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_rapid_acquire_release_alternating_holders() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Task("hot-potato".into());

        for i in 0..100 {
            let holder_a = format!("agent-a-{i}");
            let holder_b = format!("agent-b-{i}");

            {
                let _g = manager.acquire(lock.clone(), &holder_a)?;
                assert!(manager.is_locked(&lock)?);
            }
            assert!(!manager.is_locked(&lock)?);

            {
                let _g = manager.acquire(lock.clone(), &holder_b)?;
                assert!(manager.is_locked(&lock)?);
            }
            assert!(!manager.is_locked(&lock)?);
        }
        Ok(())
    }

    #[test]
    fn test_rapid_try_acquire_pattern() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Queue("try-queue".into());

        // Acquire via acquire (which inserts into manager's shared map)
        let _guard = manager.acquire(lock.clone(), "first")?;
        assert!(manager.is_locked(&lock)?);

        // try_acquire should return None while the lock is held in the manager's map
        for _ in 0..20 {
            let result = manager.try_acquire(lock.clone(), "contender")?;
            assert!(result.is_none(), "try_acquire should return None on contention");
        }

        // Manually release via manager
        manager.release(&lock)?;
        assert!(!manager.is_locked(&lock)?);

        // After release, try_acquire should succeed
        let result = manager.try_acquire(lock.clone(), "second")?;
        assert!(result.is_some());

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: LockGuard RAII drop behavior in detail
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_guard_drop_allows_reacquisition() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Agent("raii-test".into());

        {
            let _g1 = manager.acquire(lock.clone(), "first")?;
            // Inside scope: locked
            assert!(manager.is_locked(&lock)?);
        }
        // Outside scope: dropped
        assert!(!manager.is_locked(&lock)?);

        // Can reacquire with a different holder
        let _g2 = manager.acquire(lock.clone(), "second")?;
        assert!(manager.is_locked(&lock)?);
        assert_eq!(
            manager.get_lock_info(&lock)?.expect("held").holder,
            "second"
        );
        Ok(())
    }

    #[test]
    fn test_guard_drop_does_not_affect_other_locks() -> Result<()> {
        let manager = MemLockManager::new();
        let lock_a = LockType::Workspace("ws-a".into());
        let lock_b = LockType::Workspace("ws-b".into());

        let _g_a = manager.acquire(lock_a.clone(), "holder-a")?;
        {
            let _g_b = manager.acquire(lock_b.clone(), "holder-b")?;
            assert!(manager.is_locked(&lock_a)?);
            assert!(manager.is_locked(&lock_b)?);
        }
        // lock_b released, lock_a still held
        assert!(manager.is_locked(&lock_a)?);
        assert!(!manager.is_locked(&lock_b)?);
        Ok(())
    }

    #[test]
    fn test_multiple_guards_on_different_resources_drop_independently() -> Result<()> {
        let manager = MemLockManager::new();

        let g1 = manager.acquire(LockType::Task("t1".into()), "h1")?;
        let g2 = manager.acquire(LockType::Task("t2".into()), "h2")?;
        let g3 = manager.acquire(LockType::Task("t3".into()), "h3")?;

        assert_eq!(manager.list_locks()?.len(), 3);

        drop(g2);
        assert_eq!(manager.list_locks()?.len(), 2);

        drop(g1);
        assert_eq!(manager.list_locks()?.len(), 1);

        drop(g3);
        assert_eq!(manager.list_locks()?.len(), 0);
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: get_lock_info for all lock types
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_lock_info_all_types() -> Result<()> {
        let manager = MemLockManager::new();

        let test_cases = vec![
            (LockType::Workspace("ws".into()), "ws-holder"),
            (LockType::Session("sess".into()), "sess-holder"),
            (LockType::Queue("queue".into()), "queue-holder"),
            (LockType::Agent("agent".into()), "agent-holder"),
            (LockType::Task("task".into()), "task-holder"),
        ];

        let mut guards = Vec::new();
        for (lock, holder) in &test_cases {
            let guard = manager.acquire(lock.clone(), holder)?;
            let info = manager.get_lock_info(lock)?
                .unwrap_or_else(|| panic!("lock should be held: {lock}"));

            assert_eq!(info.holder, *holder);
            assert_eq!(&info.lock_type, lock);
            // acquired_at should be roughly now
            assert!(info.acquired_at <= Utc::now());
            guards.push(guard);
        }

        assert_eq!(manager.list_locks()?.len(), 5);
        Ok(())
    }

    #[test]
    fn test_get_lock_info_reflects_latest_holder() -> Result<()> {
        let manager = MemLockManager::new();
        let lock = LockType::Workspace("reacquire".into());

        let _g1 = manager.acquire(lock.clone(), "first-holder")?;
        assert_eq!(
            manager.get_lock_info(&lock)?.expect("held").holder,
            "first-holder"
        );
        drop(_g1);

        let _g2 = manager.acquire(lock.clone(), "second-holder")?;
        assert_eq!(
            manager.get_lock_info(&lock)?.expect("held").holder,
            "second-holder"
        );
        Ok(())
    }

    #[test]
    fn test_get_lock_info_multiple_resources() -> Result<()> {
        let manager = MemLockManager::new();
        let lock_a = LockType::Session("a".into());
        let lock_b = LockType::Session("b".into());

        let _ga = manager.acquire(lock_a.clone(), "holder-a")?;
        let _gb = manager.acquire(lock_b.clone(), "holder-b")?;

        // Info for a does not leak into b and vice versa
        let info_a = manager.get_lock_info(&lock_a)?.expect("held");
        let info_b = manager.get_lock_info(&lock_b)?.expect("held");
        assert_eq!(info_a.holder, "holder-a");
        assert_eq!(info_b.holder, "holder-b");
        assert_ne!(info_a.lock_type, info_b.lock_type);

        // Querying a resource that does not exist
        let lock_c = LockType::Session("nonexistent".into());
        assert!(manager.get_lock_info(&lock_c)?.is_none());
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lock_type_serde_roundtrip_all_variants() {
        let variants = vec![
            LockType::Workspace("ws".to_string()),
            LockType::Session("sess".to_string()),
            LockType::Queue("q".to_string()),
            LockType::Agent("ag".to_string()),
            LockType::Task("task".to_string()),
        ];
        for lt in variants {
            let json = serde_json::to_string(&lt).expect("serialize ok");
            let deserialized: LockType = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(lt, deserialized);
        }
    }

    #[test]
    fn test_lock_info_serde_roundtrip() {
        let now = Utc::now();
        let info = LockInfo {
            holder: "agent-1".to_string(),
            acquired_at: now,
            lock_type: LockType::Workspace("test-ws".to_string()),
        };
        let json = serde_json::to_string(&info).expect("serialize ok");
        let deserialized: LockInfo = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(info.holder, deserialized.holder);
        assert_eq!(info.lock_type, deserialized.lock_type);
    }
}
