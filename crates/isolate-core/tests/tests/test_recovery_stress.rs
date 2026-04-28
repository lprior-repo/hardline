// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]

// Recovery Logging Stress Tests
//
// Comprehensive stress tests for recovery logging functionality under
// high concurrency and large data volumes.
//
// PERFORMANCE OPTIMIZATIONS:
// Round 1:
// - Removed global mutex
// - Removed unnecessary sleep() delays (370ms eliminated)
// - Used JoinSet for concurrent workers
// - Batched filesystem operations where possible
// - Used functional patterns for verification
//
// Round 2:
// - Eliminated spawn_blocking overhead for directory changes
// - Removed redundant directory restores (temp_dir cleanup handles it)
// - Replaced Vec/collect with JoinSet in test_concurrent_recovery_logging
// - Reduced path cloning by using references where possible
// - Removed unused futures dependency
//
// Round 3:
// - Added #[serial] attribute to all tests to prevent race conditions caused by
//   std::env::set_current_dir() being a global operation
// - Tests now run sequentially, eliminating flaky I/O failures

use isolate_core::{
    config::{RecoveryConfig, RecoveryPolicy},
    recovery::log_recovery,
    Error,
};
use serial_test::serial;
use tokio::{io::AsyncReadExt, task::JoinSet};

/// Test concurrent recovery logging with multiple writers
#[tokio::test]
#[serial]
async fn test_concurrent_recovery_logging() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    // Change to temp directory so log_recovery creates the file there
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Spawn 20 concurrent loggers using JoinSet for better performance
    let mut join_set = JoinSet::new();
    for i in 0..20 {
        join_set.spawn(async move {
            let result = log_recovery(&format!("Concurrent message {i}"), &config).await;
            (i, result)
        });
    }

    // Verify all succeeded (while still in temp_dir)
    let mut errors = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((_i, Ok(()))) => {}
            Ok((i, Err(e))) => errors.push(format!("Task {i} failed: {e}")),
            Err(e) => errors.push(format!("Task join failed: {e}")),
        }
    }

    if !errors.is_empty() {
        return Err(Error::IoError(format!(
            "{count} tasks failed: {errors:?}",
            count = errors.len(),
            errors = errors
        )));
    }

    // Verify log file contains all messages (using absolute path while temp_dir still alive)
    let log_path = isolate_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    // Batch verify all messages with early exit
    let missing: Vec<_> = (0..20)
        .filter(|i| !content.contains(&format!("Concurrent message {i}")))
        .collect();

    assert!(missing.is_empty(), "Missing messages in log: {missing:?}");

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test handling of large recovery logs
#[tokio::test]
#[serial]
async fn test_large_recovery_log_handling() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Log 1000 entries (sequential but fast without sleeps)
    for i in 0..1000 {
        let message = format!("Entry {i}: Some relatively long message to increase log size",);
        log_recovery(&message, &config).await?;
    }

    // Verify log file exists and is reasonable size (< 1MB for 1000 entries)
    let log_path = isolate_dir.join("recovery.log");
    let metadata = tokio::fs::metadata(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to get metadata: {e}")))?;
    let file_size = metadata.len();

    assert!(
        file_size < 1_000_000,
        "Log file too large: {file_size} bytes (expected < 1MB for 1000 entries)"
    );

    assert!(file_size > 0, "Log file is empty");

    // Verify we can read the entire file
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    // Spot check some entries
    assert!(content.contains("Entry 0:"));
    assert!(content.contains("Entry 500:"));
    assert!(content.contains("Entry 999:"));

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test concurrent logging with integrity checks
#[tokio::test]
#[serial]
async fn test_concurrent_logging_integrity() -> Result<(), Error> {
    const NUM_WORKERS: usize = 50;
    const ENTRIES_PER_WORKER: usize = 10;

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Spawn 50 workers, each logging 10 entries (parallel)
    let mut join_set = JoinSet::new();
    for worker_id in 0..NUM_WORKERS {
        join_set.spawn(async move {
            for entry_id in 0..ENTRIES_PER_WORKER {
                let message = format!("Worker {worker_id} entry {entry_id}");
                log_recovery(&message, &config).await?;
            }
            Result::<(), Error>::Ok(())
        });
    }

    // Wait for all workers and collect errors
    let mut errors = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(Error::IoError(format!("Worker join failed: {e}"))),
            Ok(Ok(())) => {}
        }
    }

    // Fail if any worker had errors
    if !errors.is_empty() {
        return Err(Error::IoError(format!("{} workers failed", errors.len())));
    }

    // Verify integrity: all messages present (parallel verification)
    let log_path = isolate_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    // Batch verify all entries (collect missing messages)
    let missing: Vec<String> = (0..NUM_WORKERS)
        .flat_map(|worker_id| {
            (0..ENTRIES_PER_WORKER)
                .map(move |entry_id| format!("Worker {worker_id} entry {entry_id}"))
                .filter(|message| !content.contains(message.as_str()))
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(
        missing.is_empty(),
        "Missing {} messages in log, examples: {:?}",
        missing.len(),
        missing.iter().take(5).collect::<Vec<_>>()
    );

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test recovery logging when disabled
#[tokio::test]
#[serial]
async fn test_recovery_log_disabled() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: false.into(), // Disabled
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Logging should succeed even if disabled
    log_recovery("This should not be logged", &config).await?;

    // No sleep needed - we check immediately
    // Verify log file was NOT created
    let log_path = isolate_dir.join("recovery.log");
    let log_exists = tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check existence: {e}")))?;
    assert!(
        !log_exists,
        "Log file should not exist when recovery logging is disabled"
    );

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test that recovery logging appends rather than overwrites
#[tokio::test]
#[serial]
async fn test_recovery_log_append_behavior() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Log three messages (no sleeps needed - fsync is handled by tokio)
    log_recovery("First message", &config).await?;
    log_recovery("Second message", &config).await?;
    log_recovery("Third message", &config).await?;

    // Verify all messages are present
    let log_path = isolate_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    assert!(content.contains("First message"), "Missing first message");
    assert!(content.contains("Second message"), "Missing second message");
    assert!(content.contains("Third message"), "Missing third message");

    // Verify messages appear in order (first before second before third)
    let first_pos = content
        .find("First message")
        .unwrap_or_else(|| panic!("First message should be in log"));
    let second_pos = content
        .find("Second message")
        .unwrap_or_else(|| panic!("Second message should be in log"));
    let third_pos = content
        .find("Third message")
        .unwrap_or_else(|| panic!("Third message should be in log"));

    assert!(
        first_pos < second_pos && second_pos < third_pos,
        "Messages not in correct order"
    );

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test recovery logging with special characters and unicode
#[tokio::test]
#[serial]
async fn test_recovery_log_with_special_characters() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Test various special characters (no sleeps needed)
    let test_messages = vec![
        "Unicode: 你好世界 🎉",
        "Special chars: \\n \\t \\r \\\\ '",
        "Quotes: double and single",
        "Path separators: /home/user/test\\path",
        "Emoji: 🚀 🔥 💻 🦀",
        "Math: ∑ ∫ ∞ √ ≠ ≤ ≥",
        "Currency: $ £ € ¥ ₹",
        "Arrows: ← → ↑ ↓ ↔",
        "Box drawing: ┌─┐ │ └─┘",
        "Mixed: Test with emoji 🦀 and unicode 你好 and $pecials!",
    ];

    for message in &test_messages {
        log_recovery(message, &config).await?;
    }

    // Verify all messages are preserved correctly
    let log_path = isolate_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    // Batch verify all messages
    let test_patterns = [
        "你好世界",
        "🎉",
        "\\n",
        "double",
        "🚀",
        "∑ ∫",
        "£ € ¥",
        "← →",
        "┌─┐",
        "🦀",
    ];
    let missing: Vec<_> = test_patterns
        .iter()
        .filter(|&&pattern| !content.contains(pattern))
        .collect();

    assert!(missing.is_empty(), "Missing patterns in log: {missing:?}");

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}

/// Test recovery logging with empty and whitespace messages
#[tokio::test]
#[serial]
async fn test_recovery_log_empty_message_handling() -> Result<(), Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let isolate_dir = temp_dir.path().join(".isolate");
    tokio::fs::create_dir(&isolate_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .isolate: {e}")))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true.into(),
        auto_recover_corrupted_wal: true.into(),
        delete_corrupted_database: false.into(),
    };

    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {e}")))?;

    // Test edge cases (no sleeps needed)
    let test_cases = vec!["", " ", "  ", "\t", "\n", "\r\n", "   \t\t   "];

    for message in test_cases {
        let result = log_recovery(message, &config).await;

        // Empty/whitespace messages should either succeed or fail gracefully
        // We don't prescribe the exact behavior, just ensure no panics
        match result {
            Ok(()) | Err(Error::ValidationError { .. }) => {
                // Acceptable: success or empty messages rejected
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    // Verify log file is readable (even if empty)
    let log_path = isolate_dir.join("recovery.log");
    let log_exists = tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check existence: {e}")))?;
    if log_exists {
        let mut content = String::new();
        let mut file = tokio::fs::File::open(&log_path)
            .await
            .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
        file.read_to_string(&mut content)
            .await
            .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

        // File should be readable without errors
        assert!(content.len() < 10_000_000, "Log file unexpectedly large");
    }

    // No need to restore directory - temp_dir cleanup handles it
    Ok(())
}
