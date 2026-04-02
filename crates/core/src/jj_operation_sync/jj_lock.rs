//! Cross-process lock acquisition for workspace creation
//!
//! Ensures only one workspace creation proceeds at a time across
//! all processes to prevent operation graph corruption.

#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use fs2::FileExt;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::error::{Error, Result};

/// In-memory lock for serializing workspace creation within a process
pub static WORKSPACE_CREATION_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

pub const LOCK_ACQUISITION_TIMEOUT: Duration = Duration::from_millis(50);
pub const MAX_LOCK_RETRIES: usize = 5;

pub const WORKSPACE_CREATION_LOCK_FILE: &str = ".scp-workspace-create.lock";
pub const FILE_LOCK_TIMEOUT_MS: u64 = 5000;
pub const FILE_LOCK_MAX_RETRIES: usize = 3;
pub const FILE_LOCK_BASE_BACKOFF_MS: u64 = 25;

/// Maximum backoff duration in milliseconds (cap to prevent overflow).
pub const MAX_BACKOFF_MS: u64 = 5000;

/// Calculate backoff duration in milliseconds for a given retry attempt.
///
/// Uses checked arithmetic and caps at [`MAX_BACKOFF_MS`] to prevent
/// overflow for arbitrarily high attempt counts.
pub fn calculate_backoff_ms(attempt: u32) -> u64 {
    2_u64
        .checked_pow(attempt)
        .and_then(|pow| FILE_LOCK_BASE_BACKOFF_MS.checked_mul(pow))
        .map_or(MAX_BACKOFF_MS, |v| v.min(MAX_BACKOFF_MS))
}

/// Wrapper that impls `DerefMut` for tokio `MutexGuard` to allow dropping.
pub struct MutexGuardClosing<'a, T>(tokio::sync::MutexGuard<'a, T>);

impl<'a, T> std::ops::Deref for MutexGuardClosing<'a, T> {
    type Target = tokio::sync::MutexGuard<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for MutexGuardClosing<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Build a lock timeout error for in-memory workspace creation lock.
fn build_workspace_lock_timeout_error(timeout_ms: u64, retries: usize) -> Error {
    crate::error_jj::JjErrorKind::LockTimeout {
        operation: "workspace creation".to_string(),
        timeout_ms,
        retries,
    }
    .into()
}

/// Acquire in-memory lock with exponential backoff.
pub(super) async fn acquire_lock_with_backoff(
) -> Result<MutexGuardClosing<'static, ()>> {
    let timeout_ms = u64::try_from(LOCK_ACQUISITION_TIMEOUT.as_millis())
        .map_err(|_| Error::internal("timeout ms overflow"))?;
    let mut current_timeout = LOCK_ACQUISITION_TIMEOUT;

    for attempt in 0..MAX_LOCK_RETRIES {
        match timeout(current_timeout, WORKSPACE_CREATION_LOCK.lock()).await {
            Ok(guard) => return Ok(MutexGuardClosing(guard)),
            Err(_) if attempt + 1 < MAX_LOCK_RETRIES => {
                tokio::time::sleep(current_timeout).await;
                current_timeout *= 2;
            }
            Err(_) => {
                return Err(build_workspace_lock_timeout_error(
                    timeout_ms,
                    MAX_LOCK_RETRIES,
                ));
            }
        }
    }

    Err(build_workspace_lock_timeout_error(
        timeout_ms,
        MAX_LOCK_RETRIES,
    ))
}

/// Build a file lock timeout error with total backoff computation.
fn build_file_lock_timeout_error(
    operation: &str,
    max_attempts: usize,
    max_attempts_u32: u32,
) -> Error {
    let total_wait_ms: u64 = (0u32..max_attempts_u32)
        .map(calculate_backoff_ms)
        .fold(0u64, u64::saturating_add);
    crate::error_jj::JjErrorKind::LockTimeout {
        operation: operation.to_string(),
        timeout_ms: total_wait_ms,
        retries: max_attempts,
    }
    .into()
}

/// Sleep with exponential backoff for a given retry attempt number.
fn sleep_with_backoff(attempt: usize) -> Result<()> {
    let attempt_u32 = u32::try_from(attempt).map_err(|_| Error::internal("attempt overflow"))?;
    let backoff = Duration::from_millis(calculate_backoff_ms(attempt_u32));
    std::thread::sleep(backoff);
    Ok(())
}

/// Acquire file lock with exponential backoff.
pub fn acquire_file_lock_with_timeout(file: &File, description: &str) -> Result<()> {
    const MAX_ATTEMPTS: usize = 8;
    let max_u32 = u32::try_from(MAX_ATTEMPTS).map_err(|_| Error::internal("overflow"))?;
    let err = || build_file_lock_timeout_error(description, MAX_ATTEMPTS, max_u32);

    for attempt in 0..MAX_ATTEMPTS {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(_) if attempt + 1 < MAX_ATTEMPTS => sleep_with_backoff(attempt)?,
            Err(_) => return Err(err()),
        }
    }
    Err(err())
}

/// Open the workspace creation lock file.
fn open_lock_file(lock_path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|e| Error::io_error(format!("Failed to open workspace lock file: {e}")))
}

/// Verify that the filesystem supports advisory file locks.
///
/// Returns `Ok(true)` if locks are supported (probe fails to acquire),
/// `Ok(false)` if locks are NOT supported (probe succeeds — lock not exclusive).
fn verify_lock_support(lock_path: &Path) -> Result<bool> {
    let probe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|e| Error::io_error(format!("Failed to open probe lock file: {e}")))?;

    match probe.try_lock_exclusive() {
        Ok(()) => {
            probe
                .unlock()
                .map_err(|e| Error::io_error(format!("Failed to unlock probe lock file: {e}")))?;
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}

/// Enforce strict lock mode when filesystem doesn't support advisory locks.
fn enforce_strict_locks(lock_path: &Path, lock_supported: bool) -> Result<()> {
    if lock_supported {
        return Ok(());
    }

    let warning = format!(
        "{{\"event\":\"lock_portability_warning\",\"code\":\"LOCK_PORTABILITY_UNSUPPORTED\",\"lock_file\":\"{}\",\"fallback\":\"process_local_only\"}}",
        lock_path.display()
    );
    tracing::warn!("{warning}");

    // Prefer the current env var name; fall back to legacy name with a warning.
    let strict = std::env::var("SCP_STRICT_LOCKS").is_ok()
        || std::env::var("Isolate_STRICT_LOCKS")
            .is_ok_and(|_| {
                tracing::warn!("Isolate_STRICT_LOCKS is deprecated; use SCP_STRICT_LOCKS");
                true
            });

    if strict {
        return Err(Error::validation_error(format!(
            "LOCK_PORTABILITY_UNSUPPORTED: {warning}. Unset SCP_STRICT_LOCKS to continue with process-local lock fallback",
        )));
    }

    Ok(())
}

/// Acquire cross-process file lock for workspace creation.
///
/// Lock file path: `{repo_root}/.scp-workspace-create.lock`
/// (NOT inside `.isolate/` to avoid chicken-and-egg TOCTOU).
///
/// Post-condition: Returns `Ok(File)` with exclusive lock held.
/// Does NOT create `.isolate` directory. Caller must call
/// `ensure_data_directory()` AFTER acquiring the lock.
///
/// # Errors
///
/// Returns an error if the lock file cannot be opened, the lock cannot be
/// acquired within the timeout, or the blocking task panics.
pub async fn acquire_cross_process_lock(repo_root: &Path) -> Result<File> {
    let lock_path = repo_root.join(WORKSPACE_CREATION_LOCK_FILE);

    tokio::task::spawn_blocking(move || {
        let file = open_lock_file(&lock_path)?;
        acquire_file_lock_with_timeout(&file, "workspace creation cross-process lock")?;
        let lock_supported = verify_lock_support(&lock_path)?;
        enforce_strict_locks(&lock_path, lock_supported)?;
        Ok::<File, Error>(file)
    })
    .await
    .map_err(|e| Error::internal(format!("Failed to join lock task: {e}")))?
}

/// Create `.isolate` data directory.
///
/// # Safety — Unenforced Precondition
///
/// **WARNING:** Caller MUST hold the cross-process lock (from
/// `acquire_cross_process_lock`) before calling this function. There is
/// no runtime guard — violating this precondition silently reintroduces
/// the TOCTOU race condition this module was designed to eliminate.
///
/// The call site in `create_workspace_synced()` satisfies this precondition
/// because it acquires the lock on the line immediately before calling
/// this function. Any new caller MUST follow the same pattern.
///
/// # Postcondition
///
/// `.isolate` directory exists at `{repo_root}/.isolate`.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub async fn ensure_data_directory(repo_root: &Path) -> Result<()> {
    let data_dir = repo_root.join(".isolate");
    tokio::fs::create_dir_all(&data_dir)
        .await
        .map_err(|e| Error::io_error(format!("Failed to create data directory: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_attempt_zero_returns_base() {
        assert_eq!(calculate_backoff_ms(0), FILE_LOCK_BASE_BACKOFF_MS);
    }

    #[test]
    fn backoff_attempt_one_doubles() {
        assert_eq!(calculate_backoff_ms(1), FILE_LOCK_BASE_BACKOFF_MS * 2);
    }

    #[test]
    fn backoff_attempt_two_quadruples() {
        assert_eq!(calculate_backoff_ms(2), FILE_LOCK_BASE_BACKOFF_MS * 4);
    }

    #[test]
    fn backoff_is_exponential() {
        let b0 = calculate_backoff_ms(0);
        let b1 = calculate_backoff_ms(1);
        let b2 = calculate_backoff_ms(2);
        let b3 = calculate_backoff_ms(3);
        assert!(b1 == b0 * 2);
        assert!(b2 == b0 * 4);
        assert!(b3 == b0 * 8);
    }

    #[test]
    fn backoff_caps_at_max() {
        let capped = calculate_backoff_ms(100);
        assert_eq!(capped, MAX_BACKOFF_MS);
    }

    #[test]
    fn backoff_large_attempt_still_caps() {
        let capped = calculate_backoff_ms(u32::MAX);
        assert_eq!(capped, MAX_BACKOFF_MS);
    }

    #[test]
    fn workspace_creation_lock_file_is_dotfile() {
        assert!(WORKSPACE_CREATION_LOCK_FILE.starts_with('.'));
        assert!(WORKSPACE_CREATION_LOCK_FILE.contains("workspace"));
        assert!(WORKSPACE_CREATION_LOCK_FILE.contains("lock"));
    }

    #[test]
    fn lock_constants_are_sensible() {
        assert!(LOCK_ACQUISITION_TIMEOUT.as_millis() > 0);
        assert!(MAX_LOCK_RETRIES > 0);
        assert!(FILE_LOCK_TIMEOUT_MS > 0);
        assert!(FILE_LOCK_MAX_RETRIES > 0);
        assert!(FILE_LOCK_BASE_BACKOFF_MS > 0);
        assert!(MAX_BACKOFF_MS > 0);
        assert!(MAX_BACKOFF_MS >= FILE_LOCK_BASE_BACKOFF_MS);
    }
}
