#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Lock management types
//!
//! This module uses Railway-Oriented Programming where operations return
//! new instances rather than mutating state. All operations are pure functions
//! with no side effects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Maximum TTL value in seconds (approximately 68 years)
const MAX_TTL_SECONDS: u64 = 2_147_483_647;

/// Minimum TTL value in seconds (1 second)
const MIN_TTL_SECONDS: u64 = 1;

/// Resource identifier - newtype to avoid primitive obsession
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(String);

impl ResourceId {
    /// Create a new resource ID with validation
    ///
    /// # Errors
    /// Returns `Error::InvalidInput` if the ID is empty.
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(Error::InvalidInput(
                "resource_id cannot be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    /// Get the resource ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lock holder identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HolderId(String);

impl HolderId {
    /// Create a new holder ID with validation
    ///
    /// # Errors
    /// Returns `Error::InvalidInput` if the ID is empty.
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(Error::InvalidInput("holder_id cannot be empty".to_string()));
        }
        Ok(Self(id))
    }

    /// Get the holder ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// TTL in seconds - must be positive and within valid range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtlSeconds(u64);

impl TtlSeconds {
    /// Create a new TTL with validation
    ///
    /// # Errors
    /// Returns `ValidationError::BelowMinimum` if TTL is less than minimum.
    /// Returns `ValidationError::ExceedsMaximum` if TTL exceeds maximum.
    pub fn new(seconds: u64) -> Result<Self, Error> {
        if seconds < MIN_TTL_SECONDS {
            return Err(Error::InvalidInput(format!(
                "ttl value {} is below minimum {}",
                seconds, MIN_TTL_SECONDS
            )));
        }
        if seconds > MAX_TTL_SECONDS {
            return Err(Error::InvalidInput(format!(
                "ttl value {} exceeds maximum {}",
                seconds, MAX_TTL_SECONDS
            )));
        }
        Ok(Self(seconds))
    }

    /// Get the TTL value
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Convert to i64 for chrono operations
    ///
    /// # Errors
    /// Returns `LockError::InvalidTtl` if the value cannot be converted.
    pub const fn as_i64(self) -> i64 {
        self.0 as i64 // Safe due to validation in new()
    }
}

impl std::fmt::Display for TtlSeconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A lock on a resource
///
/// Locks are immutable value objects representing a successful lock acquisition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lock {
    /// Resource being locked
    resource: ResourceId,
    /// Lock holder
    holder: HolderId,
    /// When acquired
    acquired_at: DateTime<Utc>,
    /// Lock timeout
    expires_at: DateTime<Utc>,
}

impl Lock {
    /// Create a new lock - pure function, no side effects
    ///
    /// # Errors
    /// Returns `LockError::InvalidTtl` if TTL exceeds maximum value.
    pub fn new(
        resource: ResourceId,
        holder: HolderId,
        ttl: TtlSeconds,
    ) -> Result<Self, crate::Error> {
        let now = Utc::now();
        Ok(Self {
            resource,
            holder,
            acquired_at: now,
            expires_at: now + chrono::Duration::seconds(ttl.as_i64()),
        })
    }

    /// Get the resource ID
    #[must_use]
    pub const fn resource(&self) -> &ResourceId {
        &self.resource
    }

    /// Get the holder ID
    #[must_use]
    pub const fn holder(&self) -> &HolderId {
        &self.holder
    }

    /// Get the acquisition time
    #[must_use]
    pub const fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }

    /// Get the expiration time
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Check if the lock is expired - pure function
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the lock is valid (not expired) - pure function
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}

/// Lock manager - immutable using Railway-Oriented Programming
///
/// All operations return new instances rather than mutating state.
/// This enables easy reasoning about state changes and prevents shared mutable state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockManager {
    locks: Vec<Lock>,
}

impl LockManager {
    /// Create a new lock manager
    #[must_use]
    pub const fn new() -> Self {
        Self { locks: Vec::new() }
    }

    /// Get all locks (including expired)
    #[must_use]
    pub const fn locks(&self) -> &Vec<Lock> {
        &self.locks
    }

    /// Get the number of locks
    #[must_use]
    pub const fn len(&self) -> usize {
        self.locks.len()
    }

    /// Check if the manager has no locks
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.locks.is_empty()
    }

    /// Try to acquire a lock on a resource
    ///
    /// Returns a new LockManager with the lock added, and the Lock itself.
    /// This is a pure function - no mutation occurs.
    ///
    /// # Errors
    /// Returns `LockError::AlreadyLocked` if resource is already locked.
    /// Returns `LockError::InvalidTtl` if TTL value is invalid.
    pub fn acquire(
        self,
        resource: ResourceId,
        holder: HolderId,
        ttl: TtlSeconds,
    ) -> Result<(Self, Lock), crate::Error> {
        // Check if resource is already locked (Railway - fail fast)
        if self.is_locked(&resource) {
            return Err(crate::Error::InvalidState(format!(
                "Resource already locked: {}",
                resource
            )));
        }

        // Create the lock
        let lock = Lock::new(resource.clone(), holder, ttl)?;

        // Return new manager with lock added
        let mut new_locks = self.locks;
        new_locks.push(lock.clone());

        Ok((Self { locks: new_locks }, lock))
    }

    /// Release a lock on a resource
    ///
    /// Returns a new LockManager with the lock removed.
    /// This is a pure function - no mutation occurs.
    ///
    /// # Errors
    /// Returns `LockError::NotHeld` if the lock is not held by the specified holder.
    pub fn release(self, resource: &ResourceId, holder: &HolderId) -> Result<Self, crate::Error> {
        let initial_len = self.locks.len();
        let new_locks: Vec<Lock> = self
            .locks
            .iter()
            .filter(|l| !(l.resource() == resource && l.holder() == holder))
            .cloned()
            .collect();

        if new_locks.len() == initial_len {
            Err(crate::Error::InvalidState(format!(
                "Lock not held: {} by {}",
                resource, holder
            )))
        } else {
            Ok(Self { locks: new_locks })
        }
    }

    /// Check if a resource is locked - pure function
    #[must_use]
    pub fn is_locked(&self, resource: &ResourceId) -> bool {
        self.locks
            .iter()
            .any(|l| l.resource() == resource && l.is_valid())
    }

    /// Get the holder of a lock on a resource - pure function
    #[must_use]
    pub fn get_holder(&self, resource: &ResourceId) -> Option<&HolderId> {
        self.locks
            .iter()
            .find(|l| l.resource() == resource && l.is_valid())
            .map(|l| l.holder())
    }

    /// Get all active locks - pure function
    #[must_use]
    pub fn active_locks(&self) -> Vec<&Lock> {
        self.locks.iter().filter(|l| l.is_valid()).collect()
    }

    /// Get all active locks as owned values
    #[must_use]
    pub fn active_locks_owned(&self) -> Vec<Lock> {
        self.locks
            .iter()
            .filter(|l| l.is_valid())
            .cloned()
            .collect()
    }

    /// Clean up expired locks
    ///
    /// Returns a new LockManager with expired locks removed,
    /// and the count of removed locks.
    /// This is a pure function - no mutation occurs.
    #[must_use]
    pub fn cleanup_expired(self) -> (Self, usize) {
        let initial_len = self.locks.len();
        let new_locks: Vec<Lock> = self.locks.into_iter().filter(Lock::is_valid).collect();
        let removed = initial_len - new_locks.len();

        (Self { locks: new_locks }, removed)
    }

    /// Force acquire a lock, releasing any existing lock on the resource
    ///
    /// Returns a new LockManager with the new lock added.
    /// This is a pure function - no mutation occurs.
    ///
    /// # Errors
    /// Returns `LockError::InvalidTtl` if TTL value is invalid.
    pub fn force_acquire(
        self,
        resource: ResourceId,
        holder: HolderId,
        ttl: TtlSeconds,
    ) -> Result<(Self, Lock), crate::Error> {
        // Create the lock
        let lock = Lock::new(resource.clone(), holder, ttl)?;

        // Remove any existing lock for this resource and add the new one
        let new_locks: Vec<Lock> = self
            .locks
            .into_iter()
            .filter(|l| l.resource() != &resource)
            .chain(std::iter::once(lock.clone()))
            .collect();

        Ok((Self { locks: new_locks }, lock))
    }

    /// Renew a lock (extend its TTL)
    ///
    /// Returns a new LockManager with the lock renewed.
    /// This is a pure function - no mutation occurs.
    ///
    /// # Errors
    /// Returns `LockError::NotHeld` if the lock is not held by the specified holder.
    /// Returns `LockError::InvalidTtl` if TTL value is invalid.
    pub fn renew(
        self,
        resource: &ResourceId,
        holder: &HolderId,
        ttl: TtlSeconds,
    ) -> Result<(Self, Lock), crate::Error> {
        // Find and validate the existing lock
        let existing = self
            .locks
            .iter()
            .find(|l| l.resource() == resource && l.holder() == holder && l.is_valid());

        match existing {
            Some(_) => {
                // Create a new lock with renewed TTL
                let new_lock = Lock::new(resource.clone(), holder.clone(), ttl)?;

                // Replace the old lock with the new one
                let new_locks: Vec<Lock> = self
                    .locks
                    .into_iter()
                    .map(|l| {
                        if l.resource() == resource && l.holder() == holder {
                            new_lock.clone()
                        } else {
                            l
                        }
                    })
                    .collect();

                Ok((Self { locks: new_locks }, new_lock))
            }
            None => Err(crate::Error::InvalidState(format!(
                "Lock not held: {} by {}",
                resource, holder
            ))),
        }
    }

    /// Check if a holder owns a lock on a resource - pure function
    #[must_use]
    pub fn holds_lock(&self, resource: &ResourceId, holder: &HolderId) -> bool {
        self.locks
            .iter()
            .any(|l| l.resource() == resource && l.holder() == holder && l.is_valid())
    }

    /// Get all locks held by a specific holder - pure function
    #[must_use]
    pub fn locks_by_holder(&self, holder: &HolderId) -> Vec<&Lock> {
        self.locks
            .iter()
            .filter(|l| l.holder() == holder && l.is_valid())
            .collect()
    }

    /// Get all locks held by a specific holder as owned values
    #[must_use]
    pub fn locks_by_holder_owned(&self, holder: &HolderId) -> Vec<Lock> {
        self.locks
            .iter()
            .filter(|l| l.holder() == holder && l.is_valid())
            .cloned()
            .collect()
    }

    /// Release all locks held by a specific holder
    ///
    /// Returns a new LockManager with all of the holder's locks removed.
    /// This is a pure function - no mutation occurs.
    #[must_use]
    pub fn release_all_by_holder(self, holder: &HolderId) -> (Self, usize) {
        let initial_len = self.locks.len();
        let new_locks: Vec<Lock> = self
            .locks
            .into_iter()
            .filter(|l| l.holder() != holder)
            .collect();
        let released = initial_len - new_locks.len();

        (Self { locks: new_locks }, released)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_resource(id: &str) -> ResourceId {
        ResourceId::new(id).expect("valid resource id")
    }

    fn create_holder(id: &str) -> HolderId {
        HolderId::new(id).expect("valid holder id")
    }

    fn create_ttl(seconds: u64) -> TtlSeconds {
        TtlSeconds::new(seconds).expect("valid ttl")
    }

    #[test]
    fn test_resource_id_new_with_valid_id() {
        let result = ResourceId::new("my-resource");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "my-resource");
    }

    #[test]
    fn test_holder_id_new_with_valid_id() {
        let result = HolderId::new("agent-1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "agent-1");
    }

    #[test]
    fn test_ttl_new_with_valid_value() {
        let result = TtlSeconds::new(60);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), 60);
    }

    #[test]
    fn test_lock_manager_new_is_empty() {
        let manager = LockManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_acquire_returns_new_manager_and_lock() {
        let manager = LockManager::new();
        let resource = create_resource("my-resource");
        let holder = create_holder("agent-1");
        let ttl = create_ttl(60);

        let (new_manager, lock) = manager
            .acquire(resource, holder, ttl)
            .expect("lock acquired");

        assert_eq!(new_manager.len(), 1);
        assert_eq!(lock.resource().as_str(), "my-resource");
    }

    #[test]
    fn test_acquire_fails_when_resource_already_locked() {
        let manager = LockManager::new();
        let resource = create_resource("my-resource");
        let holder1 = create_holder("agent-1");
        let holder2 = create_holder("agent-2");
        let ttl = create_ttl(60);

        let (manager, _lock) = manager
            .acquire(resource.clone(), holder1, ttl)
            .expect("first lock");

        let result = manager.acquire(resource.clone(), holder2, ttl);
        assert!(result.is_err());
    }

    #[test]
    fn test_release_removes_lock() {
        let manager = LockManager::new();
        let resource = create_resource("my-resource");
        let holder1 = create_holder("agent-1");
        let holder2 = create_holder("agent-1");
        let ttl = create_ttl(60);

        let (manager, _lock) = manager
            .acquire(resource.clone(), holder1, ttl)
            .expect("lock acquired");

        let new_manager = manager.release(&resource, &holder2).expect("lock released");

        assert!(!new_manager.is_locked(&resource));
        assert_eq!(new_manager.len(), 0);
    }

    #[test]
    fn test_cleanup_expired_returns_new_manager() {
        let manager = LockManager::new();
        let resource = create_resource("my-resource");
        let holder = create_holder("agent-1");
        let ttl = create_ttl(60);

        let (manager, _lock) = manager
            .acquire(resource, holder, ttl)
            .expect("lock acquired");

        let (new_manager, count) = manager.cleanup_expired();

        // Since lock hasn't expired, count should be 0
        assert_eq!(count, 0);
        assert_eq!(new_manager.len(), 1);
    }

    #[test]
    fn test_force_acquire_overwrites_existing_lock() {
        let manager = LockManager::new();
        let resource = create_resource("my-resource");
        let holder1 = create_holder("agent-1");
        let holder2 = create_holder("agent-2");
        let ttl = create_ttl(60);

        let (manager, _lock) = manager
            .acquire(resource.clone(), holder1, ttl)
            .expect("first lock");

        let (manager, lock) = manager
            .force_acquire(resource.clone(), holder2.clone(), ttl)
            .expect("force acquire");

        assert_eq!(manager.len(), 1);
        assert_eq!(
            manager.get_holder(&resource).map(|h| h.as_str()),
            Some("agent-2")
        );
        assert_eq!(lock.holder().as_str(), "agent-2");
    }
}
