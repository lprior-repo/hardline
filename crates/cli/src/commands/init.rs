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
        "jj" => init_jj(&cwd),
        "git" => init_git(&cwd),
        _ => Err(scp_core::Error::config_invalid(format!(
            "Unknown VCS type: {}",
            vcs_type
        ))),
    }
}

/// Initialize a JJ repository with cross-process locking.
fn init_jj(cwd: &std::path::Path) -> Result<()> {
    // Check if already initialized BEFORE acquiring the lock
    if cwd.join(".jj").exists() {
        println!("Already initialized with JJ");
        return Ok(());
    }

    // Verify `jj` is available on PATH
    check_jj_installed()?;

    // Acquire cross-process lock to prevent concurrent init
    let lock_file = acquire_init_lock(cwd)?;

    // Lock is held until this function returns (Drop releases it).
    // If init fails, the lock stays held until the process exits,
    // preventing misleading errors in concurrent processes.
    let result = run_jj_init(cwd);

    // Explicitly release the lock on success; on failure we keep it
    // alive until the end of the scope so concurrent processes see it.
    if result.is_ok() {
        release_init_lock(&lock_file);
    }

    result
}

/// Verify that `jj` is installed and reachable on PATH.
fn check_jj_installed() -> Result<()> {
    let output = std::process::Command::new("jj")
        .arg("--version")
        .output()
        .map_err(|e| {
            scp_core::Error::vcs_init_failed(
                "jj",
                "<system>",
                format!("'jj' command not found: {e}. Is jj installed and on your PATH?"),
            )
        })?;

    if !output.status.success() {
        return Err(scp_core::Error::vcs_init_failed(
            "jj",
            "<system>",
            format!(
                "'jj --version' returned non-zero exit code: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    Ok(())
}

/// Run `jj init --name main` and classify the error.
fn run_jj_init(cwd: &std::path::Path) -> Result<()> {
    let output = std::process::Command::new("jj")
        .args(["init", "--name", "main"])
        .current_dir(cwd)
        .output()
        .map_err(|e| {
            scp_core::Error::vcs_init_failed(
                "jj",
                cwd.display().to_string(),
                format!("Failed to execute 'jj init': {e}"),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = classify_jj_init_error(&stderr, cwd);
        return Err(scp_core::Error::vcs_init_failed(
            "jj",
            cwd.display().to_string(),
            reason,
        ));
    }

    println!("Initialized JJ in {:?}", cwd);
    Ok(())
}

/// Classify a JJ init error into a human-readable reason with context.
///
/// This prevents raw JJ stderr (e.g., "error: unrecognized subcommand init")
/// from being shown directly to the user without explanation.
fn classify_jj_init_error(stderr: &str, cwd: &std::path::Path) -> String {
    let stderr_trimmed = stderr.trim();

    // Check for partial .jj directory (concurrent init race aftermath)
    if cwd.join(".jj").exists() {
        return format!(
            "The repository appears to be already initialized (.{vcs}/ directory exists). \
             Another init process may have completed concurrently. \
             Raw JJ output: {raw}",
            vcs = "jj",
            raw = stderr_trimmed,
        );
    }

    // Check for lock-related errors (another process holds the working copy lock)
    let stderr_lower = stderr.to_lowercase();
    if stderr_lower.contains("lock")
        || stderr_lower.contains("concurrent")
        || stderr_lower.contains("already in progress")
    {
        return format!(
            "Another init process is in progress. \
             Wait for it to complete and try again. \
             Raw JJ output: {raw}",
            raw = stderr_trimmed,
        );
    }

    // Check for unrecognized subcommand (JJ version issue or wrong binary)
    if stderr_lower.contains("unrecognized subcommand") || stderr_lower.contains("unknown command")
    {
        return format!(
            "The installed 'jj' version does not support the 'init' command. \
             This may indicate an outdated or non-standard jj binary. \
             Raw JJ output: {raw}",
            raw = stderr_trimmed,
        );
    }

    // Default: include the raw output with context
    format!(
        "JJ initialization command failed (exit code non-zero). \
         Raw JJ output: {raw}",
        raw = stderr_trimmed,
    )
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

    /// Helper to create a temp dir path for testing classify_jj_init_error.
    /// Using a path that does not have .jj directory ensures we test the
    /// stderr-classification branches rather than the "already initialized" branch.
    fn test_cwd() -> std::path::PathBuf {
        std::env::temp_dir().join("scp-test-nonexistent-dir")
    }

    // ---- classify_jj_init_error ----

    #[test]
    fn classify_lock_error() {
        let cwd = test_cwd();
        let stderr = "Error: failed to acquire lock";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("Another init process is in progress"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_concurrent_error() {
        let cwd = test_cwd();
        let stderr = "Error: concurrent operation detected";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("Another init process is in progress"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_already_in_progress_error() {
        let cwd = test_cwd();
        let stderr = "Error: operation already in progress";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("Another init process is in progress"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_lock_error_case_insensitive() {
        let cwd = test_cwd();
        let stderr = "LOCK HELD BY ANOTHER PROCESS";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("Another init process is in progress"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_unrecognized_subcommand_error() {
        let cwd = test_cwd();
        let stderr = "error: unrecognized subcommand init";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("does not support the 'init' command"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_unknown_command_error() {
        let cwd = test_cwd();
        let stderr = "error: unknown command 'init'";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("does not support the 'init' command"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_default_generic_error() {
        let cwd = test_cwd();
        let stderr = "some random error message";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("JJ initialization command failed"),
            "got: {result}"
        );
        assert!(
            result.contains("some random error message"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_empty_stderr() {
        let cwd = test_cwd();
        let stderr = "";
        let result = classify_jj_init_error(stderr, &cwd);
        assert!(
            result.contains("JJ initialization command failed"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_whitespace_stderr() {
        let cwd = test_cwd();
        let stderr = "   \n\t  ";
        let result = classify_jj_init_error(stderr, &cwd);
        // Whitespace is trimmed, but still should match default case
        assert!(
            result.contains("JJ initialization command failed"),
            "got: {result}"
        );
    }

    #[test]
    fn classify_already_initialized_when_jj_dir_exists() {
        // Use the temp dir itself (or current dir) and create .jj
        let temp = tempfile::tempdir().expect("create tempdir");
        let jj_dir = temp.path().join(".jj");
        std::fs::create_dir_all(&jj_dir).expect("create .jj dir");

        let stderr = "Error: already exists";
        let result = classify_jj_init_error(stderr, temp.path());
        assert!(result.contains("already initialized"), "got: {result}");
        assert!(result.contains("Error: already exists"), "got: {result}");
    }

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
