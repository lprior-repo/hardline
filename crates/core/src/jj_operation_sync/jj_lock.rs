//! Cross-process lock acquisition for workspace creation
//!
//! Ensures only one workspace creation proceeds at a time across
//! all processes to prevent operation graph corruption.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(unused)]

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use fs2::FileExt;
use tokio::sync::Mutex;
use tokio::time::{error::Elapsed, timeout};

use crate::error::{Error, Result};

/// In-memory lock for serializing workspace creation within a process
pub static WORKSPACE_CREATION_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

pub const LOCK_ACQUISITION_TIMEOUT: Duration = Duration::from_millis(50);
pub const MAX_LOCK_RETRIES: usize = 5;

pub const WORKSPACE_CREATION_LOCK_FILE: &str = "workspace-create.lock";
pub const FILE_LOCK_TIMEOUT_MS: u64 = 5000;
pub const FILE_LOCK_MAX_RETRIES: usize = 3;
pub const FILE_LOCK_BASE_BACKOFF_MS: u64 = 25;

/// Wrapper that impls DerefMut for tokio MutexGuard to allow dropping
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

/// Acquire in-memory lock with exponential backoff
pub(super) fn acquire_lock_with_backoff() -> impl std::future::Future<Output = Result<MutexGuardClosing<'static, ()>>> + Send {
    async move {
        let mut current_timeout = LOCK_ACQUISITION_TIMEOUT;

        for attempt in 0..MAX_LOCK_RETRIES {
            match timeout(current_timeout, WORKSPACE_CREATION_LOCK.lock()).await {
                Ok(guard) => return Ok(MutexGuardClosing(guard)),
                Err(Elapsed { .. }) => {
                    if attempt < MAX_LOCK_RETRIES - 1 {
                        tokio::time::sleep(current_timeout).await;
                        current_timeout *= 2;
                    } else {
                        let timeout_ms =
                            u64::try_from(LOCK_ACQUISITION_TIMEOUT.as_millis()).unwrap_or(u64::MAX);
                        return Err(crate::error_jj::JjErrorKind::LockTimeout {
                            operation: "workspace creation".to_string(),
                            timeout_ms,
                            retries: MAX_LOCK_RETRIES,
                        }
                        .into());
                    }
                }
            }
        }

        let timeout_ms =
            u64::try_from(LOCK_ACQUISITION_TIMEOUT.as_millis()).unwrap_or(u64::MAX);
        Err(crate::error_jj::JjErrorKind::LockTimeout {
            operation: "workspace creation".to_string(),
            timeout_ms,
            retries: MAX_LOCK_RETRIES,
        }
        .into())
    }
}

/// Acquire file lock with exponential backoff
pub fn acquire_file_lock_with_timeout(file: &File, description: &str) -> Result<()> {
    const HIGH_CONTENTION_MAX_ATTEMPTS: usize = 8;

    for attempt in 0..HIGH_CONTENTION_MAX_ATTEMPTS {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(_) if attempt < HIGH_CONTENTION_MAX_ATTEMPTS - 1 => {
                let attempt_u32 = u32::try_from(attempt).map_err(|_| {
                    Error::io_error(format!("Invalid retry attempt: {attempt}"))
                })?;
                let backoff_ms = FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt_u32);
                let backoff = Duration::from_millis(backoff_ms);
                std::thread::sleep(backoff);
            }
            Err(_) => {
                let max_attempts_u32 =
                    u32::try_from(HIGH_CONTENTION_MAX_ATTEMPTS).unwrap_or(8);
                let total_wait_ms: u64 = (0u32..max_attempts_u32)
                    .map(|i| FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(i))
                    .sum();
                return Err(crate::error_jj::JjErrorKind::LockTimeout {
                    operation: description.to_string(),
                    timeout_ms: total_wait_ms,
                    retries: HIGH_CONTENTION_MAX_ATTEMPTS,
                }
                .into());
            }
        }
    }

    let max_attempts_u32 = u32::try_from(HIGH_CONTENTION_MAX_ATTEMPTS).unwrap_or(8);
    let total_wait_ms: u64 = (0u32..max_attempts_u32)
        .map(|i| FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(i))
        .sum();
    Err(crate::error_jj::JjErrorKind::LockTimeout {
        operation: "file lock acquisition".to_string(),
        timeout_ms: total_wait_ms,
        retries: HIGH_CONTENTION_MAX_ATTEMPTS,
    }
    .into())
}

/// Acquire cross-process file lock for workspace creation
pub async fn acquire_cross_process_lock(repo_root: &Path) -> Result<File> {
    let lock_dir = repo_root.join(".isolate");
    tokio::fs::create_dir_all(&lock_dir)
        .await
        .map_err(|e| Error::io_error(format!("Failed to create lock directory: {e}")))?;

    let lock_path = lock_dir.join(WORKSPACE_CREATION_LOCK_FILE);

    tokio::task::spawn_blocking(move || {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::io_error(format!("Failed to open workspace lock file: {e}")))?;

        acquire_file_lock_with_timeout(&file, "workspace creation cross-process lock")?;

        let lock_supported = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::io_error(format!("Failed to open probe lock file: {e}")))
            .and_then(|probe| match probe.try_lock_exclusive() {
                Ok(()) => {
                    let unlock_result = probe.unlock();
                    if let Err(unlock_error) = unlock_result {
                        return Err(Error::io_error(format!(
                            "Failed to unlock probe lock file: {unlock_error}"
                        )));
                    }
                    Ok(false)
                }
                Err(_) => Ok(true),
            })?;

        if !lock_supported {
            let warning = format!(
                "{{\"event\":\"lock_portability_warning\",\"code\":\"LOCK_PORTABILITY_UNSUPPORTED\",\"lock_file\":\"{}\",\"fallback\":\"process_local_only\"}}",
                lock_path.display()
            );
            tracing::warn!("{warning}");

            if std::env::var("Isolate_STRICT_LOCKS").is_ok() {
                return Err(Error::validation_error(format!(
                    "LOCK_PORTABILITY_UNSUPPORTED: {warning}. Unset Isolate_STRICT_LOCKS to continue with process-local lock fallback",
                )));
            }
        }

        Ok::<File, Error>(file)
    })
    .await
    .map_err(|e| Error::io_error(format!("Failed to join lock task: {e}")))?
}
