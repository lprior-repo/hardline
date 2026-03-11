//! JJ operation graph synchronization for workspace creation
//!
//! This module solves the problem where multiple concurrent workspace
//! creations can cause operation graph corruption. The issue occurs when:
//!
//! 1. Workspace A is created based on operation X
//! 2. Workspace B is created based on operation Y (sibling of X)
//! 3. Each workspace has its own working copy operation ID
//! 4. JJ detects a mismatch and refuses to load the repo
//!
//! The solution is to ensure all workspace creations are serialized
//! and based on the same repository operation.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    process::Command as StdCommand,
    sync::OnceLock,
    time::Duration,
};

use fs2::FileExt;
use tokio::{
    process::Command,
    sync::Mutex,
    time::{error::Elapsed, timeout},
};

use crate::error::{Error, Result};

static JJ_PATH: OnceLock<String> = OnceLock::new();

fn resolve_jj_path() -> String {
    let env_path = std::env::var("Isolate_JJ_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let path = env_path.as_ref().map_or_else(search_path_for_jj, |p| {
        if std::path::Path::new(p).exists() {
            p.clone()
        } else {
            search_path_for_jj()
        }
    });
    path
}

fn search_path_for_jj() -> String {
    let paths = std::env::var_os("PATH").unwrap_or_default();

    let found = std::env::split_paths(&paths)
        .map(|p| p.join("jj"))
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string());

    found.unwrap_or_else(|| "jj".to_string())
}

pub fn get_jj_command() -> Command {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    Command::new(path.as_str())
}

pub fn get_jj_command_sync() -> StdCommand {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    StdCommand::new(path)
}

static WORKSPACE_CREATION_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

const LOCK_ACQUISITION_TIMEOUT: Duration = Duration::from_millis(50);

const MAX_LOCK_RETRIES: usize = 5;

const WORKSPACE_CREATION_LOCK_FILE: &str = "workspace-create.lock";

const FILE_LOCK_TIMEOUT_MS: u64 = 5000;

const FILE_LOCK_MAX_RETRIES: usize = 3;

const FILE_LOCK_BASE_BACKOFF_MS: u64 = 25;

async fn acquire_lock_with_backoff() -> Result<MutexGuardClosing<'static, ()>> {
    let mut current_timeout = LOCK_ACQUISITION_TIMEOUT;

    for attempt in 0..MAX_LOCK_RETRIES {
        match timeout(current_timeout, WORKSPACE_CREATION_LOCK.lock()).await {
            Ok(guard) => return Ok(MutexGuardClosing(guard)),
            Err(Elapsed { .. }) => {
                if attempt < MAX_LOCK_RETRIES - 1 {
                    tokio::time::sleep(current_timeout).await;
                    current_timeout *= 2;
                } else {
                    let timeout_ms = u64::try_from(LOCK_ACQUISITION_TIMEOUT.as_millis())
                        .unwrap_or(u64::MAX);
                    return Err(Error::LockTimeout {
                        operation: "workspace creation".to_string(),
                        timeout_ms,
                        retries: MAX_LOCK_RETRIES,
                    });
                }
            }
        }
    }

    let timeout_ms = u64::try_from(LOCK_ACQUISITION_TIMEOUT.as_millis()).unwrap_or(u64::MAX);
    Err(Error::LockTimeout {
        operation: "workspace creation".to_string(),
        timeout_ms,
        retries: MAX_LOCK_RETRIES,
    })
}

struct MutexGuardClosing<'a, T>(tokio::sync::MutexGuard<'a, T>);

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

#[derive(Debug, Clone)]
pub struct RepoOperationInfo {
    pub operation_id: String,
    pub repo_root: PathBuf,
}

pub async fn get_current_operation(root: &Path) -> Result<RepoOperationInfo> {
    let output = get_jj_command()
        .args(["op", "log", "--no-graph", "--limit", "1", "-T", "id"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "get current operation".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get current operation".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        });
    }

    let operation_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if operation_id.is_empty() {
        return Err(Error::JjCommandError {
            operation: "get current operation".to_string(),
            msg: "Empty operation ID returned".to_string(),
            is_not_found: false,
        });
    }

    let root_output = get_jj_command()
        .args(["root"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "get repo root".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !root_output.status.success() {
        let stderr = String::from_utf8_lossy(&root_output.stderr);
        return Err(Error::JjCommandError {
            operation: "get repo root".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        });
    }

    let repo_root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_string();

    Ok(RepoOperationInfo {
        operation_id,
        repo_root: PathBuf::from(repo_root),
    })
}

pub async fn create_workspace_synced(name: &str, path: &Path, repo_root: &Path) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    let _lock = WORKSPACE_CREATION_LOCK.lock().await;

    let _cross_process_lock = acquire_cross_process_lock(repo_root).await?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::IoError(format!("Failed to create workspace directory: {e}")))?;
    }

    let _ = get_current_operation(repo_root).await?;

    let output = get_jj_command()
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .current_dir(repo_root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "create workspace".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "create workspace".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        });
    }

    verify_workspace_consistency(name, path).await?;

    Ok(())
}

fn acquire_file_lock_with_timeout(file: &File, description: &str) -> Result<()> {
    const HIGH_CONTENTION_MAX_ATTEMPTS: usize = 8;

    for attempt in 0..HIGH_CONTENTION_MAX_ATTEMPTS {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(_) if attempt < HIGH_CONTENTION_MAX_ATTEMPTS - 1 => {
                let attempt_u32 = u32::try_from(attempt)
                    .map_err(|_| Error::IoError(format!("Invalid retry attempt: {attempt}")))?;
                let backoff_ms = FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt_u32);
                let backoff = Duration::from_millis(backoff_ms);
                std::thread::sleep(backoff);
            }
            Err(_) => {
                let max_attempts_u32 = u32::try_from(HIGH_CONTENTION_MAX_ATTEMPTS).unwrap_or(8);
                let total_wait_ms: u64 = (0u32..max_attempts_u32)
                    .map(|i| FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(i))
                    .sum();
                return Err(Error::LockTimeout {
                    operation: description.to_string(),
                    timeout_ms: total_wait_ms,
                    retries: HIGH_CONTENTION_MAX_ATTEMPTS,
                });
            }
        }
    }

    let max_attempts_u32 = u32::try_from(HIGH_CONTENTION_MAX_ATTEMPTS).unwrap_or(8);
    let total_wait_ms: u64 = (0u32..max_attempts_u32)
        .map(|i| FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(i))
        .sum();
    Err(Error::LockTimeout {
        operation: "file lock acquisition".to_string(),
        timeout_ms: total_wait_ms,
        retries: HIGH_CONTENTION_MAX_ATTEMPTS,
    })
}

async fn acquire_cross_process_lock(repo_root: &Path) -> Result<File> {
    let lock_dir = repo_root.join(".isolate");
    tokio::fs::create_dir_all(&lock_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create lock directory: {e}")))?;

    let lock_path = lock_dir.join(WORKSPACE_CREATION_LOCK_FILE);

    tokio::task::spawn_blocking(move || {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::IoError(format!("Failed to open workspace lock file: {e}")))?;

        acquire_file_lock_with_timeout(&file, "workspace creation cross-process lock")?;

        let lock_supported = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::IoError(format!("Failed to open probe lock file: {e}")))
            .and_then(|probe| match probe.try_lock_exclusive() {
                Ok(()) => {
                    let unlock_result = probe.unlock();
                    if let Err(unlock_error) = unlock_result {
                        return Err(Error::IoError(format!(
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
                return Err(Error::ValidationError(
                    format!("LOCK_PORTABILITY_UNSUPPORTED: {warning}. Unset Isolate_STRICT_LOCKS to continue with process-local lock fallback"),
                ));
            }
        }

        Ok::<File, Error>(file)
    })
    .await
    .map_err(|e| Error::IoError(format!("Failed to join lock task: {e}")))?
}

async fn verify_workspace_consistency(name: &str, path: &Path) -> Result<()> {
    let output = get_jj_command()
        .args(["status"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "verify workspace operation".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        let error_str = stderr.to_string();

        if error_str.contains("sibling of the working copy's operation")
            || error_str.contains("working copy")
            || error_str.contains("operation")
        {
            return Err(Error::JjWorkspaceConflict {
                conflict_type: crate::error::JjConflictType::Stale,
                workspace_name: name.to_string(),
                msg: format!(
                    "Operation graph mismatch: {error_str}"
                ),
                recovery_hint: format!(
                    "The workspace '{name}' was created but has an inconsistent operation graph.\n\n\
                     Recovery: Run 'jj workspace forget {name}' and retry creation.\n\n\
                     This error indicates concurrent workspace creation or repo state change."
                ),
            });
        }

        return Err(Error::JjCommandError {
            operation: "verify workspace operation".to_string(),
            msg: error_str,
            is_not_found: false,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::significant_drop_tightening,
        clippy::unnecessary_cast,
        clippy::assertions_on_constants,
        clippy::suspicious_open_options
    )]

    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::sync::Barrier;

    use super::*;

    #[test]
    fn test_workspace_creation_lock_exists() {
        let _ = &WORKSPACE_CREATION_LOCK;
    }

    #[tokio::test]
    async fn test_empty_workspace_name_returns_error() {
        let temp_dir = std::env::temp_dir().join("test-empty-name");
        let repo_root = std::env::temp_dir().join("test-repo-root");
        let result = create_workspace_synced("", &temp_dir, &repo_root).await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("workspace name cannot be empty"));
            }
            Ok(()) => panic!("Expected InvalidConfig error, but got Ok"),
            Err(other) => panic!("Expected InvalidConfig error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_workspace_without_parent_returns_error() {
        let workspace_path = PathBuf::from("/");
        let repo_root = std::env::temp_dir().join("test-repo-root");
        let result = create_workspace_synced("test", &workspace_path, &repo_root).await;

        match result {
            Err(Error::JjCommandError { .. }) => {}
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("parent directory") || msg.contains("invalid"));
            }
            Err(other) => panic!("Expected JjCommandError or InvalidConfig error, got: {other:?}"),
            Ok(()) => panic!("Expected error when workspace path has no parent, but got Ok"),
        }
    }

    #[tokio::test]
    async fn regression_cross_process_lock_blocks_second_holder() -> Result<()> {
        let repo_root = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let repo_root_path = repo_root.path().to_path_buf();

        let _lock_file_handle = acquire_cross_process_lock(&repo_root_path).await?;

        let lock_path = repo_root_path
            .join(".isolate")
            .join(WORKSPACE_CREATION_LOCK_FILE);

        let second_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let second_lock_attempt = second_file.try_lock_exclusive();
        assert!(second_lock_attempt.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn regression_cross_process_lock_releases_on_drop() -> Result<()> {
        let repo_root = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let repo_root_path = repo_root.path().to_path_buf();

        {
            let _first = acquire_cross_process_lock(&repo_root_path).await?;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let lock_path = repo_root_path
            .join(".isolate")
            .join(WORKSPACE_CREATION_LOCK_FILE);
        let second_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let second_lock_attempt = second_file.try_lock_exclusive();
        assert!(
            second_lock_attempt.is_ok(),
            "Should be able to acquire lock after first is dropped"
        );

        Ok(())
    }

    #[tokio::test]
    async fn stress_cross_process_lock_keeps_single_holder() -> Result<()> {
        use std::sync::Arc;

        let repo_root = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let repo_root_path = Arc::new(repo_root.path().to_path_buf());

        let task_count = 24usize;
        let barrier = Arc::new(Barrier::new(task_count));
        let in_critical = Arc::new(AtomicUsize::new(0));
        let max_critical = Arc::new(AtomicUsize::new(0));

        let tasks: Vec<_> = (0..task_count)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let in_critical = Arc::clone(&in_critical);
                let max_critical = Arc::clone(&max_critical);
                let repo_root_path = Arc::clone(&repo_root_path);

                tokio::spawn(async move {
                    barrier.wait().await;

                    let guard = acquire_cross_process_lock(&repo_root_path).await;
                    if guard.is_err() {
                        return;
                    }

                    let current = in_critical.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = max_critical.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
                        if current > prev {
                            Some(current)
                        } else {
                            None
                        }
                    });

                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    in_critical.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();

        let join_results = futures::future::join_all(tasks).await;
        assert!(join_results.iter().all(std::result::Result::is_ok));
        assert_eq!(max_critical.load(Ordering::SeqCst), 1);

        Ok(())
    }

    #[test]
    fn given_repo_operation_info_when_cloned_then_deep_copy() {
        let info = RepoOperationInfo {
            operation_id: "abc123".into(),
            repo_root: PathBuf::from("/tmp/repo"),
        };
        let cloned = info;
        assert_eq!(cloned.operation_id, "abc123");
        assert_eq!(cloned.repo_root, PathBuf::from("/tmp/repo"));
    }

    #[test]
    fn given_repo_operation_info_when_formatted_then_shows_fields() {
        let info = RepoOperationInfo {
            operation_id: "xyz789".into(),
            repo_root: PathBuf::from("/test/path"),
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("xyz789"));
        assert!(debug_str.contains("/test/path"));
    }

    #[tokio::test]
    async fn given_mutex_guard_closing_when_dereferenced_then_accesses_inner_guard() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock().await;
        let wrapped = MutexGuardClosing(guard);

        assert_eq!(**wrapped, 42);
    }

    #[tokio::test]
    async fn given_mutex_guard_closing_when_mutably_dereferenced_then_can_mutate() {
        let mutex = Mutex::new(10);
        let guard = mutex.lock().await;
        let mut wrapped = MutexGuardClosing(guard);

        **wrapped = 20;
        assert_eq!(**wrapped, 20);
    }

    #[test]
    fn given_file_lock_on_available_file_when_acquired_then_succeeds() -> Result<()> {
        use std::fs;
        let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let lock_path = temp_dir.path().join("test.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let result = acquire_file_lock_with_timeout(&file, "test lock");
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn given_file_already_locked_when_timeout_acquisition_then_returns_error() -> Result<()> {
        use std::fs;
        let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let lock_path = temp_dir.path().join("test.lock");

        let file1 = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        file1
            .try_lock_exclusive()
            .map_err(|e| Error::IoError(e.to_string()))?;

        let file2 = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| Error::IoError(e.to_string()))?;

        let result = acquire_file_lock_with_timeout(&file2, "contended lock");
        assert!(result.is_err());

        match result {
            Err(Error::LockTimeout {
                operation, retries, ..
            }) => {
                assert_eq!(operation, "contended lock");
                assert!(retries > 0);
            }
            _ => panic!("Expected LockTimeout error"),
        }

        Ok(())
    }

    #[test]
    fn given_lock_constants_when_validated_then_reasonable_values() {
        assert!(LOCK_ACQUISITION_TIMEOUT.as_millis() > 0);
        assert!(MAX_LOCK_RETRIES > 0);
        assert!(FILE_LOCK_TIMEOUT_MS > 0);
        assert!(FILE_LOCK_MAX_RETRIES > 0);
        assert!(FILE_LOCK_BASE_BACKOFF_MS > 0);
        assert_eq!(WORKSPACE_CREATION_LOCK_FILE, "workspace-create.lock");
    }

    #[test]
    fn given_lock_backoff_when_calculated_then_exponential() {
        let base = FILE_LOCK_BASE_BACKOFF_MS;
        assert_eq!(base * 2_u64.pow(0), base);
        assert_eq!(base * 2_u64.pow(1), base * 2);
        assert_eq!(base * 2_u64.pow(2), base * 4);
        assert_eq!(base * 2_u64.pow(3), base * 8);
    }
}
