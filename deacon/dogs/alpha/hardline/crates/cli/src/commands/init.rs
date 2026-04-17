//! Initialize command
//!
//! Handles VCS initialization with cross-process locking to prevent
//! concurrent init operations from corrupting repository state.

use fs2::FileExt;
use scp_core::Result;
use scp_vcs::gix::repository;

/// Lock file used to prevent concurrent init operations.
const INIT_LOCK_FILE: &str = ".scp-init.lock";

/// Maximum time (ms) to wait for the init lock before giving up.
const INIT_LOCK_TIMEOUT_MS: u64 = 5000;

/// Maximum number of lock acquisition retries.
const INIT_LOCK_MAX_RETRIES: usize = 10;

/// Base backoff interval in milliseconds for lock retries.
const INIT_LOCK_BASE_BACKOFF_MS: u64 = 50;

/// Initialize SCP in current directory
pub fn run(vcs_type: &str) -> Result<()> {
    println!("Initializing Source Control Plane...");

    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::io_error(e.to_string()))?;

    match vcs_type {
        "git" => init_git(&cwd),
        _ => Err(scp_core::Error::config_invalid(format!(
            "Unknown VCS type: {}",
            vcs_type
        ))),
    }
}

/// Initialize a Git repository with cross-process locking.
fn init_git(cwd: &std::path::Path) -> Result<()> {
    // Check if already initialized BEFORE acquiring the lock
    match repository::open(cwd) {
        Ok(_) => {
            println!("Already initialized with Git");
            return Ok(());
        }
        Err(_) => {
            // Repository doesn't exist, proceed with initialization
        }
    }

    // Acquire cross-process lock to prevent concurrent init
    let lock_file = acquire_init_lock(cwd)?;

    let result = run_git_init(cwd);

    if result.is_ok() {
        release_init_lock(&lock_file);
    }

    result
}

/// Run `git init` via gix and classify the error.
fn run_git_init(cwd: &std::path::Path) -> Result<()> {
    repository::init(cwd).map_err(|e| {
        scp_core::Error::vcs_init_failed("git", cwd.display().to_string(), e.to_string())
    })?;

    println!("Initialized Git in {:?}", cwd);
    Ok(())
}

/// Acquire the cross-process init lock file with retry and exponential backoff.
///
/// Returns the open `File` handle. The lock is released when the `File` is
/// dropped (or explicitly via `release_init_lock`).
pub(crate) fn acquire_init_lock(cwd: &std::path::Path) -> Result<std::fs::File> {
    let lock_path = cwd.join(INIT_LOCK_FILE);

    // Security: refuse to follow symlinks for lock files
    if let Ok(meta) = std::fs::symlink_metadata(&lock_path) {
        if meta.file_type().is_symlink() {
            return Err(scp_core::Error::io_error(format!(
                "Lock file at {} is a symlink (refusing to follow for security reasons)",
                lock_path.display()
            )));
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| {
            scp_core::Error::io_error(format!(
                "Failed to open init lock file {}: {e}",
                lock_path.display()
            ))
        })?;

    for attempt in 0..INIT_LOCK_MAX_RETRIES {
        match file.try_lock_exclusive() {
            Ok(()) => {
                // Write PID for diagnostics
                let pid = std::process::id();
                use std::io::Write;
                if let Err(e) = (|| -> std::io::Result<()> {
                    file.set_len(0)?;
                    write!(file, "{pid}")
                })() {
                    tracing::warn!("Failed to write PID to lock file: {e}");
                }
                // Re-lock after write (write may have been seen as a release by some systems)
                let _ = file.try_lock_exclusive();
                return Ok(file);
            }
            Err(_) if attempt + 1 < INIT_LOCK_MAX_RETRIES => {
                let backoff_ms = INIT_LOCK_BASE_BACKOFF_MS.saturating_mul(1 << attempt.min(5));
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
            }
            Err(_) => {
                return Err(scp_core::Error::vcs_init_failed(
                    "init",
                    cwd.display().to_string(),
                    format!(
                        "Another init process is in progress (lock file: {}). \
                         Wait for it to complete, or remove the lock file if the \
                         process has crashed.",
                        lock_path.display()
                    ),
                ));
            }
        }
    }

    Err(scp_core::Error::vcs_init_failed(
        "init",
        cwd.display().to_string(),
        format!(
            "Timed out waiting for init lock after {}ms (lock file: {}). \
             Another init process may be running.",
            INIT_LOCK_TIMEOUT_MS,
            lock_path.display()
        ),
    ))
}

/// Explicitly release the init lock and clean up the lock file.
fn release_init_lock(file: &std::fs::File) {
    if let Err(e) = file.unlock() {
        tracing::warn!("Failed to release init lock: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Constants ----

    #[test]
    fn lock_file_name() {
        assert_eq!(INIT_LOCK_FILE, ".scp-init.lock");
    }

    #[test]
    fn timeout_is_reasonable() {
        assert!(INIT_LOCK_TIMEOUT_MS >= 1000);
        assert!(INIT_LOCK_TIMEOUT_MS <= 30_000);
    }

    #[test]
    fn max_retries_is_positive() {
        assert!(INIT_LOCK_MAX_RETRIES >= 1);
    }

    #[test]
    fn backoff_base_is_reasonable() {
        assert!(INIT_LOCK_BASE_BACKOFF_MS >= 10);
        assert!(INIT_LOCK_BASE_BACKOFF_MS <= 1000);
    }

    // ---- acquire_init_lock ----

    #[test]
    fn acquire_lock_creates_lock_file() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let result = acquire_init_lock(temp.path());
        assert!(result.is_ok(), "Should acquire lock in temp dir");
        // Lock file should exist
        assert!(temp.path().join(INIT_LOCK_FILE).exists());
    }

    #[test]
    fn acquire_lock_fails_on_symlink() {
        let temp = tempfile::tempdir().expect("create tempdir");
        // Create a symlink to a non-existent file
        let symlink_target = temp.path().join("target");
        let symlink = temp.path().join(INIT_LOCK_FILE);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&symlink_target, &symlink).expect("create symlink");

        #[cfg(unix)]
        {
            let result = acquire_init_lock(temp.path());
            assert!(result.is_err(), "Should reject symlink lock file");
            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains("symlink"),
                "Expected symlink error, got: {err_msg}"
            );
        }
    }
}
