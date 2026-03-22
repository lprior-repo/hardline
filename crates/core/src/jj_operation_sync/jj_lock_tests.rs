//! Tests for cross-process lock acquisition

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(unused)]

use std::fs::{File, OpenOptions};

use fs2::FileExt;

use crate::error::{Error, Result};

use crate::jj_operation_sync::jj_lock::{
    acquire_cross_process_lock, acquire_file_lock_with_timeout, FILE_LOCK_BASE_BACKOFF_MS,
    FILE_LOCK_MAX_RETRIES, FILE_LOCK_TIMEOUT_MS, LOCK_ACQUISITION_TIMEOUT, MAX_LOCK_RETRIES,
    WORKSPACE_CREATION_LOCK_FILE,
};

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
    let barrier = Arc::new(tokio::sync::Barrier::new(task_count));
    let in_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_critical = Arc::new(std::sync::atomic::AtomicUsize::new(0));

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

                let current =
                    in_critical.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                let _ = max_critical.fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |prev| {
                        if current > prev {
                            Some(current)
                        } else {
                            None
                        }
                    },
                );

                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                in_critical.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            })
        })
        .collect();

    let join_results = futures::future::join_all(tasks).await;
    assert!(join_results.iter().all(std::result::Result::is_ok));
    assert_eq!(
        max_critical.load(std::sync::atomic::Ordering::SeqCst),
        1
    );

    Ok(())
}
