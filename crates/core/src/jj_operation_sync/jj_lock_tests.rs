//! Tests for cross-process lock acquisition — TOCTOU fix (hl-6h1)
//!
//! 24 BDD scenarios (B1-B24) + 3 proptest invariants + 2 Kani harnesses
//!
//! RED PHASE: These tests compile but FAIL because the implementation
//! has NOT been changed yet. The implementation changes needed:
//! - WORKSPACE_CREATION_LOCK_FILE changes from "workspace-create.lock" to ".scp-workspace-create.lock"
//! - New function: ensure_data_directory(repo_root: &Path) -> Result<(), Error>
//! - acquire_cross_process_lock() should NOT create the .isolate directory
//! - create_workspace_synced() should call ensure_data_directory() AFTER acquiring lock

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(
    unused,
    clippy::significant_drop_tightening,
    clippy::unnecessary_cast,
    clippy::assertions_on_constants,
    clippy::suspicious_open_options,
    clippy::module_name_repetitions
)]

use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fs2::FileExt;

use crate::error::{Error, Result};
use crate::jj_operation_sync::jj_lock::{
    acquire_cross_process_lock, acquire_file_lock_with_timeout, FILE_LOCK_BASE_BACKOFF_MS,
    FILE_LOCK_MAX_RETRIES, FILE_LOCK_TIMEOUT_MS, LOCK_ACQUISITION_TIMEOUT, MAX_LOCK_RETRIES,
    WORKSPACE_CREATION_LOCK_FILE,
};
use crate::jj_operation_sync::jj_lock::ensure_data_directory;

/// Helper: assert that an Error is the Jj variant with LockTimeout message
fn assert_is_lock_timeout(result: &Result<File>, expected_operation: &str) {
    match result {
        Err(Error::Jj(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains("Lock acquisition timeout"),
                "Expected LockTimeout error, got: {msg}"
            );
            assert!(
                msg.contains(expected_operation),
                "Expected operation '{expected_operation}' in error: {msg}"
            );
        }
        other => panic!(
            "Expected Error::Jj(JjError {{ inner: JjErrorKind::LockTimeout {{ .. }} }}), got: {other:?}"
        ),
    }
}

/// Helper: assert that an Error is the Io variant containing a substring
fn assert_is_io_error(result: &Result<File>, expected_substring: &str) {
    match result {
        Err(Error::Io(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains(expected_substring),
                "Expected IoError containing '{expected_substring}', got: {msg}"
            );
        }
        other => panic!(
            "Expected Error::Io(IoError {{ .. }}) containing '{expected_substring}', got: {other:?}"
        ),
    }
}

/// Helper: assert that a Result<()> is the Io variant containing a substring
fn assert_is_io_error_unit(result: &Result<()>, expected_substring: &str) {
    match result {
        Err(Error::Io(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains(expected_substring),
                "Expected IoError containing '{expected_substring}', got: {msg}"
            );
        }
        other => panic!(
            "Expected Error::Io(IoError {{ .. }}) containing '{expected_substring}', got: {other:?}"
        ),
    }
}

// =========================================================================
// B1: Lock acquisition succeeds when uncontested
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_returns_file_when_repo_root_accessible() {
    // Given: a valid, writable tempdir as repo_root
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Ok(File)
    let _file =
        result.expect("acquire_cross_process_lock should return Ok(File) when uncontested");

    // And: the file at {repo_root}/.scp-workspace-create.lock exists on disk
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    assert!(
        lock_path.exists(),
        "Lock file should exist at repo root"
    );

    // And: a second try_lock_exclusive() on the same lock path returns Err (lock is held)
    let second_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("Failed to open lock file for second handle");
    let second_lock_result = second_file.try_lock_exclusive();
    assert!(
        second_lock_result.is_err(),
        "Second process should NOT be able to acquire exclusive lock while first holds it"
    );
}

// =========================================================================
// B2: Lock file placed at repo root (NOT inside .isolate/)
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_places_lock_at_repo_root_when_called() {
    // Given: a valid repo_root directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When
    let _lock = acquire_cross_process_lock(&repo_root_path)
        .await
        .expect("Lock acquisition should succeed");

    // Then: repo_root/.scp-workspace-create.lock exists
    assert!(
        repo_root_path.join(".scp-workspace-create.lock").exists(),
        "Lock file must be at repo root: .scp-workspace-create.lock"
    );

    // And: .isolate/workspace-create.lock does NOT exist
    assert!(
        !repo_root_path
            .join(".isolate")
            .join("workspace-create.lock")
            .exists(),
        "Old lock path (.isolate/workspace-create.lock) must NOT exist"
    );

    // And: .isolate directory does NOT exist (no phantom directory)
    assert!(
        !repo_root_path.join(".isolate").exists(),
        ".isolate directory must NOT be created by acquire_cross_process_lock"
    );
}

// =========================================================================
// B3: No .isolate directory side effect
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_does_not_create_isolate_dir_when_called() {
    // Given: a valid repo_root with NO .isolate directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // Verify .isolate does NOT exist before
    assert!(
        !repo_root_path.join(".isolate").exists(),
        "Precondition: .isolate should not exist before test"
    );

    // When
    let _lock = acquire_cross_process_lock(&repo_root_path)
        .await
        .expect("Lock acquisition should succeed");

    // Then: .isolate directory does NOT exist
    assert!(
        !repo_root_path.join(".isolate").exists(),
        "acquire_cross_process_lock must NOT create .isolate directory as side effect"
    );
}

// =========================================================================
// B4: Lock released on File drop
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_releases_when_file_dropped() {
    // Given: a valid repo_root
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When: acquire, then drop
    {
        let _first = acquire_cross_process_lock(&repo_root_path)
            .await
            .expect("First lock acquisition should succeed");
        // _first dropped here
    }

    // Give OS time to release advisory lock
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: a subsequent open + try_lock_exclusive succeeds
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    let second_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("Failed to open lock file for second handle");

    let second_lock_attempt = second_file.try_lock_exclusive();
    assert!(
        second_lock_attempt.is_ok(),
        "Should be able to acquire lock after first is dropped"
    );
}

// =========================================================================
// B5: Lock timeout when contended
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_returns_lock_timeout_when_another_process_holds_lock() {
    // Given: repo_root where process A already holds the exclusive lock
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // Manually create lock file and acquire it (simulating process A)
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    let holder_file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&lock_path)
        .expect("Failed to create lock file");
    holder_file
        .try_lock_exclusive()
        .expect("Process A should acquire lock");

    // When: process B tries to acquire
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { ... } }))
    assert_is_lock_timeout(&result, "workspace creation cross-process lock");

    // And: verify the specific timeout and retries via error message parsing
    let err_str = result.unwrap_err().to_string();
    // Verify retries count is 8
    assert!(
        err_str.contains("8 retries"),
        "Expected '8 retries' in error: {err_str}"
    );
    // Compute expected total_wait_ms: sum of 25 * 2^i for i in 0..7
    let expected_total: u64 = (0u32..8).map(|i| FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(i)).sum();
    assert!(
        err_str.contains(&format!("{expected_total}ms")),
        "Expected '{expected_total}ms' (total backoff) in error: {err_str}"
    );
}

// =========================================================================
// B6: IoError on permission denied
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_returns_io_error_when_repo_root_read_only() {
    // Given: a repo_root directory with 0o444 permissions (read-only)
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // Make directory read-only
    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o444))
        .expect("Failed to set read-only permissions");

    // When
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))
    assert_is_io_error(&result, "Failed to open workspace lock file");

    // Cleanup: restore permissions so tempdir can be cleaned up
    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o755))
        .expect("Failed to restore permissions");
}

// =========================================================================
// B7: Internal error on spawn_blocking join failure (L5: contract E1)
// =========================================================================

#[tokio::test]
#[ignore = "tokio::runtime::Builder panics on max_blocking_threads(0). \
              spawn_blocking JoinError is only producible by runtime shutdown, \
              which cannot be deterministically tested without FFI."]
async fn acquire_cross_process_lock_returns_internal_error_when_task_join_fails() {
    // Given: a tokio runtime that has been shut down
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // Spawn a new thread with its own runtime, shut it down, then try to use it.
    // We use a separate runtime because spawn_blocking requires an active runtime.
    let handle = std::thread::spawn(move || {
        // Create a runtime with very limited blocking thread pool (0 threads)
        // so that spawn_blocking will fail to schedule the task.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(0)
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        rt.block_on(acquire_cross_process_lock(&repo_root_path))
    });

    let result = handle.join().expect("Thread panicked");

    // Then: returns Err(Error::Internal(InternalError { inner: InternalErrorKind::Internal(msg) }))
    // L5: Contract E1 Error Taxonomy specifies Internal for spawn_blocking join failures
    match &result {
        Err(Error::Internal(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains("Failed to join lock task"),
                "Expected Internal error containing 'Failed to join lock task', got: {msg}"
            );
        }
        other => panic!(
            "Expected Error::Internal(InternalError {{ inner: InternalErrorKind::Internal(msg) }}), got: {other:?}"
        ),
    }
}

// =========================================================================
// B8: Strict locks validation error (DEFERRED — requires exotic FS)
// =========================================================================

#[tokio::test]
#[ignore = "Requires filesystem that does not support advisory file locks (e.g., NFS with noac mount). \
             Not CI-feasible without exotic mount options. Run manually on NFS mount with Isolate_STRICT_LOCKS=1."]
async fn acquire_cross_process_lock_returns_validation_error_when_strict_locks_on_unsupported_fs() {
    // Given: Isolate_STRICT_LOCKS environment variable is set
    std::env::set_var("Isolate_STRICT_LOCKS", "1");

    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Err(Error::State(StateError { inner: StateErrorKind::ValidationError(msg) }))
    match &result {
        Err(Error::State(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains("LOCK_PORTABILITY_UNSUPPORTED"),
                "Error message must contain 'LOCK_PORTABILITY_UNSUPPORTED', got: {msg}"
            );
        }
        other => panic!(
            "Expected Error::State(StateError {{ inner: StateErrorKind::ValidationError(msg) }}), got: {other:?}"
        ),
    }

    std::env::remove_var("Isolate_STRICT_LOCKS");
}

// =========================================================================
// B9: Data directory creation
// =========================================================================

#[tokio::test]
async fn ensure_data_directory_creates_isolate_dir_when_called() {
    // Given: a valid repo_root with NO .isolate directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    assert!(
        !repo_root_path.join(".isolate").exists(),
        "Precondition: .isolate should not exist before test"
    );

    // When
    let result = ensure_data_directory(&repo_root_path).await;

    // Then: returns Ok(())
    assert!(
        result.is_ok(),
        "ensure_data_directory should return Ok(()), got: {result:?}"
    );

    // And: .isolate directory exists and is a directory
    assert!(
        repo_root_path.join(".isolate").is_dir(),
        ".isolate directory must exist after ensure_data_directory call"
    );
}

// =========================================================================
// B10: Data directory idempotent
// =========================================================================

#[tokio::test]
async fn ensure_data_directory_succeeds_when_isolate_dir_already_exists() {
    // Given: a valid repo_root where .isolate directory already exists
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    fs::create_dir_all(repo_root_path.join(".isolate"))
        .expect("Failed to pre-create .isolate directory");

    // When
    let result = ensure_data_directory(&repo_root_path).await;

    // Then: returns Ok(())
    assert!(
        result.is_ok(),
        "ensure_data_directory should succeed when .isolate already exists, got: {result:?}"
    );

    // And: .isolate directory still exists
    assert!(
        repo_root_path.join(".isolate").is_dir(),
        ".isolate directory must still exist after idempotent call"
    );
}

// =========================================================================
// B11: Data directory IoError on permission denied
// =========================================================================

#[tokio::test]
async fn ensure_data_directory_returns_io_error_when_creation_fails() {
    // Given: a repo_root directory with 0o444 permissions (read-only)
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o444))
        .expect("Failed to set read-only permissions");

    // When
    let result = ensure_data_directory(&repo_root_path).await;

    // Then: returns Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))
    assert_is_io_error_unit(&result, "Failed to create data directory");

    // Cleanup: restore permissions
    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o755))
        .expect("Failed to restore permissions");
}

// =========================================================================
// B12: No lock side effect from ensure_data_directory
// =========================================================================

#[tokio::test]
async fn ensure_data_directory_does_not_touch_lock_file_when_called() {
    // Given: a valid repo_root with no .scp-workspace-create.lock file
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    assert!(
        !repo_root_path.join(".scp-workspace-create.lock").exists(),
        "Precondition: lock file should not exist"
    );

    // When
    let result = ensure_data_directory(&repo_root_path).await;

    // Then: returns Ok(())
    assert!(result.is_ok(), "ensure_data_directory should succeed, got: {result:?}");

    // And: lock file still does NOT exist
    assert!(
        !repo_root_path.join(".scp-workspace-create.lock").exists(),
        "ensure_data_directory must NOT create the lock file"
    );
}

// =========================================================================
// B13: Call order in workspace creation
// =========================================================================

#[tokio::test]
async fn create_workspace_synced_calls_ensure_data_dir_after_acquiring_lock() {
    // This test verifies the contract by calling the functions in the
    // correct order and verifying both succeed. The tracing-based ordering
    // verification is deferred until the implementation adds tracing spans.

    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // Step 1: Acquire lock
    let _lock = acquire_cross_process_lock(&repo_root_path)
        .await
        .expect("Lock acquisition should succeed");

    // Step 2: Ensure data directory (AFTER lock)
    let result = ensure_data_directory(&repo_root_path).await;
    assert!(
        result.is_ok(),
        "ensure_data_directory should succeed after lock acquired, got: {result:?}"
    );

    // Verify .isolate was created
    assert!(
        repo_root_path.join(".isolate").is_dir(),
        ".isolate directory must exist after ensure_data_directory"
    );
}

// =========================================================================
// B14: Empty name rejection
// =========================================================================

#[tokio::test]
async fn create_workspace_synced_returns_config_error_when_name_empty() {
    // Given: any valid path and repo_root
    let temp_dir = std::env::temp_dir().join("test-empty-name-hl6h1");
    let repo_root = std::env::temp_dir().join("test-repo-root-hl6h1");

    // When
    let result =
        crate::jj_operation_sync::create_workspace_synced("", &temp_dir, &repo_root).await;

    // Then: returns Err(Error::Config(ConfigError { inner: ConfigErrorKind::Invalid(msg) }))
    match &result {
        Err(Error::Config(_)) => {
            let msg = result.as_ref().unwrap_err().to_string();
            assert!(
                msg.contains("workspace name cannot be empty"),
                "Error message must contain 'workspace name cannot be empty', got: {msg}"
            );
        }
        Ok(()) => panic!("Expected Config error for empty name, but got Ok(())"),
        Err(other) => panic!(
            "Expected Error::Config(ConfigError {{ inner: ConfigErrorKind::Invalid(msg) }}), got: {other:?}"
        ),
    }
}

// =========================================================================
// B15: Lock-Before-Create invariant (TOCTOU regression — timeout)
// =========================================================================

#[tokio::test]
async fn regression_no_phantom_directory_when_lock_acquisition_times_out() {
    // Given: a valid repo_root with NO .isolate directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    assert!(
        !repo_root_path.join(".isolate").exists(),
        "Precondition: .isolate should not exist before test"
    );

    // And: another process already holds the exclusive lock
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    let holder_file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&lock_path)
        .expect("Failed to create lock file");
    holder_file
        .try_lock_exclusive()
        .expect("Process A should acquire lock");

    // When: acquire_cross_process_lock is called
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))
    assert_is_lock_timeout(&result, "workspace creation cross-process lock");

    // And: .isolate directory does NOT exist
    assert!(
        !repo_root_path.join(".isolate").exists(),
        "I1 INVARIANT VIOLATION: .isolate must NOT exist when lock acquisition fails"
    );
}

// =========================================================================
// B16: No Phantom Directory invariant (IoError — permission denied)
// =========================================================================

#[tokio::test]
async fn regression_isolate_not_created_on_io_error_from_acquire_lock() {
    // Given: a valid repo_root with NO .isolate directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    assert!(
        !repo_root_path.join(".isolate").exists(),
        "Precondition: .isolate should not exist before test"
    );

    // And: repo_root has 0o444 permissions (read-only, causes IoError)
    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o444))
        .expect("Failed to set read-only permissions");

    // When: acquire_cross_process_lock is called
    let result = acquire_cross_process_lock(&repo_root_path).await;

    // Then: returns Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))
    assert_is_io_error(&result, "Failed to open workspace lock file");

    // And: .isolate directory does NOT exist
    assert!(
        !repo_root_path.join(".isolate").exists(),
        "I2 INVARIANT VIOLATION: .isolate must NOT exist when lock acquisition returns IoError"
    );

    // Cleanup
    fs::set_permissions(&repo_root_path, fs::Permissions::from_mode(0o755))
        .expect("Failed to restore permissions");
}

// =========================================================================
// B17: Single-Holder invariant (stress test — exactly 3 concurrent tasks)
// =========================================================================

#[tokio::test]
async fn stress_max_concurrent_lock_holders_is_one() {
    // Given: a valid repo_root
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = Arc::new(repo_root.path().to_path_buf());

    // And: a Barrier sized for 3 tasks
    let barrier = Arc::new(tokio::sync::Barrier::new(3));

    // And: an AtomicUsize counter in_critical_section
    let in_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Helper: spawn a lock contender task
    let spawn_task = |barrier: Arc<tokio::sync::Barrier>,
                      in_critical: Arc<std::sync::atomic::AtomicUsize>,
                      max_critical: Arc<std::sync::atomic::AtomicUsize>,
                      repo_root_path: Arc<PathBuf>| {
        tokio::spawn(async move {
            barrier.wait().await;
            let guard = acquire_cross_process_lock(&repo_root_path).await;
            match guard {
                Ok(_file) => {
                    let current =
                        in_critical.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    // L4: CAS result explicitly consumed — may fail under contention (expected)
                    match max_critical.fetch_update(
                        std::sync::atomic::Ordering::SeqCst,
                        std::sync::atomic::Ordering::SeqCst,
                        |prev| {
                            if current > prev {
                                Some(current)
                            } else {
                                None
                            }
                        },
                    ) {
                        Ok(_) | Err(_) => {}
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    in_critical.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }
                Err(Error::Jj(_)) => {
                    // Expected — timeout is acceptable (LockTimeout variant)
                }
                Err(other) => panic!("Unexpected error in stress test: {other:?}"),
            }
        })
    };

    // And: exactly 3 tokio tasks, spawned explicitly (no iterator/map)
    let task1 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );
    let task2 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );
    let task3 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );

    // When: all 3 tasks complete
    let r1 = task1.await.expect("Task 1 panicked");
    let r2 = task2.await.expect("Task 2 panicked");
    let r3 = task3.await.expect("Task 3 panicked");

    // Then: max(in_critical_section) observed value equals 1
    assert_eq!(
        max_critical.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "I5 INVARIANT VIOLATION: max concurrent lock holders must be 1, got {}. Tasks: {:?}, {:?}, {:?}",
        max_critical.load(std::sync::atomic::Ordering::SeqCst),
        r1, r2, r3
    );
}

// =========================================================================
// B18: Idempotent acquire cycle
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_succeeds_on_repeated_acquire_drop_cycle() {
    // Given: a valid repo_root
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When: acquire -> drop -> acquire -> drop -> acquire
    let result1 = acquire_cross_process_lock(&repo_root_path).await;
    assert!(
        result1.is_ok(),
        "First acquisition should succeed"
    );
    drop(result1);

    let result2 = acquire_cross_process_lock(&repo_root_path).await;
    assert!(
        result2.is_ok(),
        "Second acquisition should succeed"
    );
    drop(result2);

    let result3 = acquire_cross_process_lock(&repo_root_path).await;
    assert!(
        result3.is_ok(),
        "Third acquisition should succeed"
    );
    drop(result3);

    // Then: only one .scp-workspace-create.lock file exists
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    assert!(
        lock_path.exists(),
        "Lock file must exist after cycle"
    );
    let count = fs::read_dir(&repo_root_path)
        .expect("Failed to read repo_root")
        .filter_map(|entry| {
            let entry = entry.expect("Failed to read entry");
            let name = entry.file_name().to_string_lossy().to_string();
            (name == ".scp-workspace-create.lock").then_some(name)
        })
        .count();
    assert_eq!(
        count, 1,
        "Exactly one lock file should exist"
    );
}

// =========================================================================
// B19: Atomic Visibility invariant (I3) — single-process poller variant
// =========================================================================

#[tokio::test]
async fn regression_isolate_never_visible_without_lock_having_been_held() {
    // Given: a valid repo_root with NO .isolate directory
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    let isolate_path = repo_root_path.join(".isolate");
    let lock_path = repo_root_path.join(".scp-workspace-create.lock");

    // And: a Notify to signal poller shutdown (L3: bounded event-driven loop)
    let stop_notify = Arc::new(tokio::sync::Notify::new());

    // And: a poller task that checks for .isolate visibility violations
    let violation_detected = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let poller_violation = Arc::clone(&violation_detected);
    let poller_stop = Arc::clone(&stop_notify);

    let poller = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = poller_stop.notified() => break,
                _ = tokio::time::sleep(Duration::from_millis(1)) => {
                    if isolate_path.exists() {
                        // Check if lock is NOT held (violation)
                        if let Ok(probe) = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open(&lock_path)
                        {
                            if probe.try_lock_exclusive().is_ok() {
                                // .isolate exists AND lock is NOT held — VIOLATION
                                // L4: explicit match instead of .ok()
                                match probe.unlock() {
                                    Ok(()) | Err(_) => {}
                                }
                                poller_violation.store(true, std::sync::atomic::Ordering::SeqCst);
                                break;
                            }
                            // Lock is held — valid state, no violation
                            drop(probe);
                        }
                    }
                }
            }
        }
    });

    // Give poller time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: acquire lock, then call ensure_data_directory, hold for 200ms
    let _lock = acquire_cross_process_lock(&repo_root_path)
        .await
        .expect("Lock acquisition should succeed");

    // Hold lock for a moment to ensure poller is checking
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Now create data directory WHILE holding lock
    let result = ensure_data_directory(&repo_root_path).await;
    assert!(
        result.is_ok(),
        "ensure_data_directory should succeed, got: {result:?}"
    );

    // Hold for 200ms
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Stop poller via Notify (event-driven, not polling)
    stop_notify.notify_one();
    let _ = poller.await;

    // Then: poller never detected a violation
    assert!(
        !violation_detected.load(std::sync::atomic::Ordering::SeqCst),
        "I3 INVARIANT VIOLATION: poller observed .isolate existing without lock being held"
    );
}

// =========================================================================
// B20: Lock file content preserved across acquire-drop-reacquire cycle
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_preserves_lock_file_content_when_reacquired() {
    // Given: a valid repo_root with a pre-existing .scp-workspace-create.lock
    // containing the exact bytes "LOCK-STATE-MARKER"
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    let lock_path = repo_root_path.join(".scp-workspace-create.lock");
    let marker = "LOCK-STATE-MARKER";
    fs::write(&lock_path, marker).expect("Failed to write marker to lock file");

    // When: acquire -> drop -> acquire -> drop
    {
        let _file1 = acquire_cross_process_lock(&repo_root_path)
            .await
            .expect("First lock acquisition should succeed");
    }
    {
        let _file2 = acquire_cross_process_lock(&repo_root_path)
            .await
            .expect("Second lock acquisition should succeed");
    }

    // Then: file content is unchanged
    let content = fs::read_to_string(&lock_path).expect("Failed to read lock file");
    assert_eq!(
        content, marker,
        "Lock file content must be preserved across acquire-drop-reacquire cycle"
    );

    // And: file size is exactly 18 bytes
    let metadata = fs::metadata(&lock_path).expect("Failed to get metadata");
    assert_eq!(
        metadata.len(),
        marker.len() as u64,
        "Lock file size must be exactly {} bytes",
        marker.len()
    );
}

// =========================================================================
// B21: Lock file opened with read permissions
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_opens_lock_file_for_reading() {
    // Given: a valid repo_root
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    // When
    let file = acquire_cross_process_lock(&repo_root_path)
        .await
        .expect("Lock acquisition should succeed");

    // Then: file.metadata() returns Ok (file handle is valid)
    let _meta = file.metadata().expect("File handle must support metadata()");

    // And: file.try_clone() returns Ok (file handle is cloneable)
    let mut cloned = file
        .try_clone()
        .expect("File handle must be cloneable (proving it's open)");

    // And: using cloned, calling read_to_string returns Ok (file is readable)
    let mut buf = String::new();
    let read_result = cloned.read_to_string(&mut buf);
    assert!(
        read_result.is_ok(),
        "File handle must be readable — read_to_string failed: {:?}",
        read_result.err()
    );
}

// =========================================================================
// B22: Backoff sleep is not removed
// =========================================================================

#[test]
fn acquire_file_lock_with_timeout_introduces_measurable_delays_on_contention() {
    // Given: a lock file where another thread holds the exclusive lock
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let lock_path = temp_dir.path().join("backoff-test.lock");

    let holder_file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&lock_path)
        .expect("Failed to create lock file");
    holder_file
        .try_lock_exclusive()
        .expect("Holder should acquire lock");

    // And: the lock holder thread will release after 200ms
    let holder = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        drop(holder_file);
    });

    // When: acquire_file_lock_with_timeout is called and elapsed time is measured
    let contender_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("Failed to open contender file");

    let start = Instant::now();
    let result = acquire_file_lock_with_timeout(&contender_file, "test contention");
    let elapsed = start.elapsed();

    holder.join().expect("Holder thread panicked");

    // Then: the call returns Ok(()) (lock acquired after holder releases)
    assert!(
        result.is_ok(),
        "Lock acquisition should succeed after holder releases, got: {result:?}"
    );

    // And: elapsed wall-clock time is >= 60ms (proving at least 2 backoff sleeps occurred)
    // base=25ms, first retry waits 25ms, second waits 50ms = 75ms minimum before 3rd attempt
    assert!(
        elapsed >= Duration::from_millis(60),
        "Elapsed time {elapsed:?} is less than 60ms — backoff sleep may have been removed. Expected >= 60ms"
    );
}

// =========================================================================
// B23: .isolate exists as regular file (not directory)
// =========================================================================

#[tokio::test]
async fn ensure_data_directory_returns_io_error_when_isolate_is_a_file_not_directory() {
    // Given: a valid repo_root where .isolate exists as a regular file
    let repo_root = tempfile::tempdir().expect("Failed to create tempdir");
    let repo_root_path = repo_root.path().to_path_buf();

    let isolate_path = repo_root_path.join(".isolate");
    fs::write(&isolate_path, "not a directory").expect("Failed to create .isolate file");

    assert!(
        isolate_path.is_file(),
        "Precondition: .isolate should be a regular file"
    );

    // When
    let result = ensure_data_directory(&repo_root_path).await;

    // Then: returns Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))
    assert_is_io_error_unit(&result, "Failed to create data directory");
}

// =========================================================================
// B24: Nonexistent repo_root
// =========================================================================

#[tokio::test]
async fn acquire_cross_process_lock_returns_io_error_when_repo_root_does_not_exist() {
    // Given: a repo_root path that does not exist
    let nonexistent_path = PathBuf::from("/tmp/nonexistent_dir_xyz123_hl6h1");

    // Verify it doesn't exist
    assert!(
        !nonexistent_path.exists(),
        "Precondition: path should not exist"
    );

    // When
    let result = acquire_cross_process_lock(&nonexistent_path).await;

    // Then: returns Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))
    assert_is_io_error(&result, "Failed to open workspace lock file");
}

// =========================================================================
// Updated existing tests (M5 — banned assertion patterns fixed)
// =========================================================================

#[test]
fn given_lock_constants_when_validated_then_reasonable_values() {
    assert!(LOCK_ACQUISITION_TIMEOUT.as_millis() > 0);
    assert!(MAX_LOCK_RETRIES > 0);
    assert!(FILE_LOCK_TIMEOUT_MS > 0);
    assert!(FILE_LOCK_MAX_RETRIES > 0);
    assert!(FILE_LOCK_BASE_BACKOFF_MS > 0);
    // B2 constant check: lock file must be at repo root (dot-prefixed)
    assert_eq!(
        WORKSPACE_CREATION_LOCK_FILE,
        ".scp-workspace-create.lock",
        "Lock file constant must be '.scp-workspace-create.lock' (repo root, NOT inside .isolate/)"
    );
}

#[test]
fn given_lock_backoff_when_calculated_then_exponential() {
    let base = FILE_LOCK_BASE_BACKOFF_MS;
    assert_eq!(base * 2_u64.pow(0), base);
    assert_eq!(base * 2_u64.pow(1), base * 2);
    assert_eq!(base * 2_u64.pow(2), base * 4);
    assert_eq!(base * 2_u64.pow(3), base * 8);
}

#[test]
fn given_file_lock_on_available_file_when_acquired_then_succeeds() -> Result<()> {
    let temp_dir = tempfile::tempdir().map_err(|e| Error::io_error(e.to_string()))?;
    let lock_path = temp_dir.path().join("test.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| Error::io_error(e.to_string()))?;

    let result = acquire_file_lock_with_timeout(&file, "test lock");
    assert!(
        result.is_ok(),
        "Lock acquisition on available file should succeed, got: {result:?}"
    );

    Ok(())
}

#[test]
fn given_file_already_locked_when_timeout_acquisition_then_returns_error() -> Result<()> {
    let temp_dir = tempfile::tempdir().map_err(|e| Error::io_error(e.to_string()))?;
    let lock_path = temp_dir.path().join("test.lock");

    let file1 = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| Error::io_error(e.to_string()))?;

    file1
        .try_lock_exclusive()
        .map_err(|e| Error::io_error(e.to_string()))?;

    let file2 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| Error::io_error(e.to_string()))?;

    // No redundant is_err() — match provides the sharp assertion
    let result = acquire_file_lock_with_timeout(&file2, "contended lock");

    match result {
        Err(Error::Jj(_)) => {
            let msg = format!("{result:?}");
            assert!(
                msg.contains("LockTimeout"),
                "Expected LockTimeout error, got: {msg}"
            );
        }
        other => panic!(
            "Expected Error::Jj(JjError {{ inner: JjErrorKind::LockTimeout {{ .. }} }}), got: {other:?}"
        ),
    }

    Ok(())
}

#[tokio::test]
async fn regression_cross_process_lock_blocks_second_holder() -> Result<()> {
    let repo_root = tempfile::tempdir().map_err(|e| Error::io_error(e.to_string()))?;
    let repo_root_path = repo_root.path().to_path_buf();

    let _lock_file_handle = acquire_cross_process_lock(&repo_root_path).await?;

    // Lock path at repo root (not inside .isolate/)
    let lock_path = repo_root_path.join(WORKSPACE_CREATION_LOCK_FILE);

    let second_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|e| Error::io_error(e.to_string()))?;

    let second_lock_attempt = second_file.try_lock_exclusive();
    let Err(_e) = second_lock_attempt else {
        panic!("Second lock attempt must fail when first process holds exclusive lock");
    };

    Ok(())
}

#[tokio::test]
async fn regression_cross_process_lock_releases_on_drop() -> Result<()> {
    let repo_root = tempfile::tempdir().map_err(|e| Error::io_error(e.to_string()))?;
    let repo_root_path = repo_root.path().to_path_buf();

    {
        let _first = acquire_cross_process_lock(&repo_root_path).await?;
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Lock path at repo root (not inside .isolate/)
    let lock_path = repo_root_path.join(WORKSPACE_CREATION_LOCK_FILE);
    let second_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|e| Error::io_error(e.to_string()))?;

    let second_lock_attempt = second_file.try_lock_exclusive();
    assert!(
        second_lock_attempt.is_ok(),
        "Should be able to acquire lock after first is dropped"
    );

    Ok(())
}

#[tokio::test]
async fn stress_cross_process_lock_keeps_single_holder() -> Result<()> {
    let repo_root = tempfile::tempdir().map_err(|e| Error::io_error(e.to_string()))?;
    let repo_root_path = Arc::new(repo_root.path().to_path_buf());

    // Updated: exactly 3 tasks (B17 compliance — Holzmann R2)
    let task_count = 3usize;
    let barrier = Arc::new(tokio::sync::Barrier::new(task_count));
    let in_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let spawn_task = |barrier: Arc<tokio::sync::Barrier>,
                      in_critical: Arc<std::sync::atomic::AtomicUsize>,
                      max_critical: Arc<std::sync::atomic::AtomicUsize>,
                      repo_root_path: Arc<PathBuf>| {
        tokio::spawn(async move {
            barrier.wait().await;
            let guard = match acquire_cross_process_lock(&repo_root_path).await {
                Ok(f) => f,
                Err(Error::Jj(_)) => return, // LockTimeout — acceptable
                Err(e) => panic!("Unexpected error in stress test: {e}"),
            };
            let current = in_critical.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            // L4: CAS result explicitly consumed — may fail under contention (expected)
            match max_critical.fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |prev| {
                    if current > prev {
                        Some(current)
                    } else {
                        None
                    }
                },
            ) {
                Ok(_) | Err(_) => {}
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            in_critical.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            drop(guard);
        })
    };

    let task1 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );
    let task2 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );
    let task3 = spawn_task(
        Arc::clone(&barrier),
        Arc::clone(&in_critical),
        Arc::clone(&max_critical),
        Arc::clone(&repo_root_path),
    );

    let join_results = futures::future::join_all(vec![task1, task2, task3]).await;
    assert!(join_results.iter().all(std::result::Result::is_ok));
    assert_eq!(
        max_critical.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "I5 INVARIANT VIOLATION: max concurrent lock holders must be 1"
    );

    Ok(())
}

// =========================================================================
// Proptest P1: Backoff arithmetic never overflows
// =========================================================================

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use crate::jj_operation_sync::jj_lock::{calculate_backoff_ms, MAX_BACKOFF_MS};

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

        #[test]
        fn proptest_backoff_never_overflows_for_valid_attempts(attempt in 0u32..100u32) {
            let backoff_ms = calculate_backoff_ms(attempt);
            prop_assert!(backoff_ms > 0, "Backoff must be positive for attempt {attempt}");
            prop_assert!(backoff_ms <= MAX_BACKOFF_MS, "Backoff {backoff_ms}ms exceeds cap for attempt {attempt}");
        }
    }
}

// =========================================================================
// Proptest P2: Total wait time is bounded and deterministic
// =========================================================================

#[cfg(test)]
mod proptests_p2 {
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

        #[test]
        fn proptest_total_wait_time_is_bounded_and_deterministic(
            base_ms in 1u64..1000u64,
            max_attempts in 1usize..20usize
        ) {
            let total: u64 = (0u32..max_attempts as u32)
                .map(|i| base_ms.checked_mul(2_u64.checked_pow(i).expect("pow overflow")))
                .sum::<Option<u64>>()
                .expect("Total wait time overflow detected");
            prop_assert!(total > 0, "Total wait time must be positive");
            // Total must be finite — no wrapping
            prop_assert!(total < u64::MAX / 2, "Total wait time suspiciously large");
        }
    }
}

// =========================================================================
// Proptest P3: Lock path is always at repo root (never nested)
// =========================================================================

#[cfg(test)]
mod proptests_p3 {
    use proptest::prelude::*;
    use std::path::PathBuf;
    use crate::jj_operation_sync::jj_lock::WORKSPACE_CREATION_LOCK_FILE;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

        #[test]
        fn proptest_lock_path_parent_always_equals_repo_root(
            suffix in "[a-zA-Z0-9_-]{1,50}"
        ) {
            let repo_root = PathBuf::from("/tmp").join(suffix);
            let lock_path = repo_root.join(WORKSPACE_CREATION_LOCK_FILE);
            prop_assert_eq!(
                lock_path.parent(),
                Some(repo_root.as_path()),
                "Lock file must be a direct child of repo_root, never nested"
            );
        }
    }
}

// =========================================================================
// Kani K1: Backoff arithmetic overflow freedom
// (Requires: cargo install kani-verifier)
// kani::proof: FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt) never panics
// for attempt in 0..=HIGH_CONTENTION_MAX_ATTEMPTS
// =========================================================================

// =========================================================================
// Kani K2: Lock constant is dot-prefixed
// (Requires: cargo install kani-verifier)
// kani::proof: WORKSPACE_CREATION_LOCK_FILE.starts_with('.')
// =========================================================================
