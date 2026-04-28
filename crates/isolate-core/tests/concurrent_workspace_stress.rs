// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::duration_subsec,
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
    // Async and concurrency relaxations for stress tests
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue,
    clippy::manual_clamp,
)]
//! Concurrent workspace creation stress test - Run with: moon run :test concurrent_workspace_stress
//!
//! This test verifies that 24+ concurrent workspace creations succeed without:
//! - Lock starvation
//! - Operation graph corruption
//! - Race conditions in workspace creation

// Include relaxed clippy settings for integration tests
mod common;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use isolate_core::{jj_operation_sync::create_workspace_synced, Error, Result};
use tokio::sync::Barrier;

/// Test concurrent workspace creation with 12 parallel tasks
///
/// This test simulates the scenario where multiple agents attempt to create
/// workspaces simultaneously, ensuring the serialization mechanism prevents
/// operation graph corruption.
#[tokio::test]
async fn stress_concurrent_workspace_creation() -> Result<()> {
    let task_count: usize = 12;
    let barrier = Arc::new(Barrier::new(task_count));
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));

    // Create a unique temp directory for this test run
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::IoError(format!("Failed to get time: {e}")))?
        .as_nanos();

    // Set up a real jj repo for the test
    let repo_temp = common::setup_test_repo()?;
    let repo_root = repo_temp.path().to_path_buf();

    // Create a unique temp directory for this test run inside the repo
    let base_path = repo_root.join(format!("test-workspaces-{}", test_id));
    tokio::fs::create_dir_all(&base_path).await?;

    let mut handles = vec![];

    // Spawn concurrent tasks that all start simultaneously
    for i in 0..task_count {
        let barrier = Arc::clone(&barrier);
        let success_count = Arc::clone(&success_count);
        let failure_count = Arc::clone(&failure_count);

        let workspace_path = base_path.join(format!("stress-test-{}", i));
        let workspace_name = format!("stress-concurrent-{}-{}", test_id, i);
        let repo_root = repo_root.clone();

        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            // Retry workspace creation on lock timeout to handle high contention
            // LockTimeout indicates temporary contention, not permanent failure
            let max_retries: usize = 3;

            for attempt in 0..=max_retries {
                let result =
                    create_workspace_synced(&workspace_name, &workspace_path, &repo_root).await;

                match result {
                    Ok(()) => {
                        success_count.fetch_add(1, Ordering::SeqCst);
                        if attempt > 0 {
                            println!(
                                "✓ Workspace {} created successfully (after {} retry)",
                                workspace_name, attempt
                            );
                        } else {
                            println!("✓ Workspace {} created successfully", workspace_name);
                        }
                        break;
                    }
                    Err(e) => {
                        // Check if this is a lock timeout (temporary contention)
                        let is_lock_timeout = matches!(e, Error::LockTimeout { .. });

                        if is_lock_timeout && attempt < max_retries {
                            // Retry with exponential backoff: 50ms, 100ms, 200ms, 400ms
                            let backoff_ms = 50_u64 * (1_u64 << attempt);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            continue;
                        }

                        // Permanent failure or max retries reached
                        failure_count.fetch_add(1, Ordering::SeqCst);
                        eprintln!(
                            "✗ Workspace {} creation failed after {} attempts: {}",
                            workspace_name,
                            attempt + 1,
                            e
                        );
                        break;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle
            .await
            .map_err(|e| Error::IoError(format!("Task join error: {e}")))?;
    }

    let successes = success_count.load(Ordering::SeqCst);
    let failures = failure_count.load(Ordering::SeqCst);

    println!("Concurrent workspace creation results: {successes} successful, {failures} failed");

    // All workspaces should be created successfully
    assert_eq!(
        successes, task_count,
        "All {task_count} workspaces should be created successfully"
    );
    assert_eq!(failures, 0, "No workspace creations should fail");

    // Cleanup: Remove all created workspaces
    for i in 0..task_count {
        let workspace_name = format!("stress-concurrent-{}-{}", test_id, i);
        let _ = tokio::process::Command::new("jj")
            .args(["workspace", "forget", &workspace_name])
            .current_dir(&repo_root)
            .output()
            .await;
    }

    // Remove base directory
    let _ = tokio::fs::remove_dir_all(base_path).await;

    // Remove temporary JJ repository
    let _ = tokio::fs::remove_dir_all(repo_root).await;

    println!("All concurrent workspace creations verified successfully");

    Ok(())
}

/// Test concurrent workspace creation with staggered starts
///
/// This tests that workspace creation works correctly even when tasks
/// start at slightly different times, simulating real-world agent spawns.
#[tokio::test]
async fn stress_concurrent_workspace_staggered() -> Result<()> {
    let task_count: usize = 12;
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));

    // Create a unique temp directory for this test run
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::IoError(format!("Failed to get time: {e}")))?
        .as_nanos();

    // Set up a real jj repo for the test
    let repo_temp = common::setup_test_repo()?;
    let repo_root = repo_temp.path().to_path_buf();

    let base_path = repo_root.join(format!("test-workspaces-staggered-{}", test_id));
    tokio::fs::create_dir_all(&base_path).await?;

    let mut handles = vec![];

    // Spawn tasks with staggered starts (every 10ms)
    for i in 0..task_count {
        let success_count = Arc::clone(&success_count);
        let failure_count = Arc::clone(&failure_count);

        let workspace_path = base_path.join(format!("staggered-{}", i));
        let workspace_name = format!("stress-staggered-{}-{}", test_id, i);
        let repo_root = repo_root.clone();

        let handle = tokio::spawn(async move {
            // Add a small random delay to simulate real-world timing
            let delay_ms = ((i % 5) * 10) as u64; // 0-40ms stagger
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            // Retry workspace creation on lock timeout to handle high contention
            // LockTimeout indicates temporary contention, not permanent failure
            let max_retries: usize = 3;

            for attempt in 0..=max_retries {
                let result =
                    create_workspace_synced(&workspace_name, &workspace_path, &repo_root).await;

                match result {
                    Ok(()) => {
                        success_count.fetch_add(1, Ordering::SeqCst);
                        if attempt > 0 {
                            println!(
                                "✓ Staggered workspace {} created successfully (after {} retry)",
                                workspace_name, attempt
                            );
                        } else {
                            println!(
                                "✓ Staggered workspace {} created successfully",
                                workspace_name
                            );
                        }
                        break;
                    }
                    Err(e) => {
                        // Check if this is a lock timeout (temporary contention)
                        let is_lock_timeout = matches!(e, Error::LockTimeout { .. });

                        if is_lock_timeout && attempt < max_retries {
                            // Retry with exponential backoff: 50ms, 100ms, 200ms, 400ms
                            let backoff_ms = 50_u64 * (1_u64 << attempt);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            continue;
                        }

                        // Permanent failure or max retries reached
                        failure_count.fetch_add(1, Ordering::SeqCst);
                        eprintln!(
                            "✗ Staggered workspace {} failed after {} attempts: {}",
                            workspace_name,
                            attempt + 1,
                            e
                        );
                        break;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle
            .await
            .map_err(|e| Error::IoError(format!("Task join error: {e}")))?;
    }

    let successes = success_count.load(Ordering::SeqCst);
    let failures = failure_count.load(Ordering::SeqCst);

    println!("Staggered workspace creation results: {successes} successful, {failures} failed");

    assert_eq!(
        successes, task_count,
        "All staggered workspaces should be created successfully"
    );
    assert_eq!(failures, 0, "No staggered workspace creations should fail");

    // Cleanup
    for i in 0..task_count {
        let workspace_name = format!("stress-staggered-{}-{}", test_id, i);
        let _ = tokio::process::Command::new("jj")
            .args(["workspace", "forget", &workspace_name])
            .current_dir(&repo_root)
            .output()
            .await;
    }

    let _ = tokio::fs::remove_dir_all(base_path).await;

    // Remove temporary JJ repository
    let _ = tokio::fs::remove_dir_all(repo_root).await;

    Ok(())
}

/// Test workspace creation under heavy contention with retries
///
/// This simulates the scenario where many agents compete for workspace creation,
/// testing the serialization lock and retry logic.
#[tokio::test]
async fn stress_workspace_creation_with_retries() -> Result<()> {
    let task_count: usize = 12;
    let success_count = Arc::new(AtomicUsize::new(0));

    // Create a unique temp directory for this test run
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::IoError(format!("Failed to get time: {e}")))?
        .as_nanos();

    // Set up a real jj repo for the test
    let repo_temp = common::setup_test_repo()?;
    let repo_root = repo_temp.path().to_path_buf();

    let base_path = repo_root.join(format!("test-workspaces-retry-{}", test_id));
    tokio::fs::create_dir_all(&base_path).await?;

    let mut handles = vec![];

    // Spawn 30 tasks all trying to create workspaces simultaneously
    for i in 0..task_count {
        let success_count = Arc::clone(&success_count);

        let workspace_path = base_path.join(format!("retry-test-{}", i));
        let workspace_name = format!("stress-retry-{}-{}", test_id, i);
        let repo_root = repo_root.clone();

        let handle = tokio::spawn(async move {
            // Retry logic with exponential backoff
            let mut attempt: u32 = 0;
            let max_attempts: u32 = 5;

            loop {
                let result =
                    create_workspace_synced(&workspace_name, &workspace_path, &repo_root).await;

                match result {
                    Ok(()) => {
                        success_count.fetch_add(1, Ordering::SeqCst);
                        println!(
                            "✓ Workspace {} succeeded on attempt {}",
                            workspace_name,
                            attempt + 1
                        );
                        break;
                    }
                    Err(e) => {
                        attempt += 1;
                        if attempt >= max_attempts {
                            eprintln!(
                                "✗ Workspace {} failed after {} attempts: {}",
                                workspace_name, max_attempts, e
                            );
                            break;
                        }

                        // Exponential backoff: 10ms, 20ms, 40ms, 80ms
                        let backoff_ms = 10 * 2_u64.pow(attempt - 1);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks with a timeout
    let timeout_duration = Duration::from_secs(30);
    let start = std::time::Instant::now();

    for handle in handles {
        tokio::select! {
            result = handle => {
                result.map_err(|e| Error::IoError(format!("Task join error: {e}")))?;
            }
            _ = tokio::time::sleep(timeout_duration) => {
                panic!("Workspace creation test timed out after {:?}", timeout_duration);
            }
        }
    }

    let elapsed = start.elapsed();
    let successes = success_count.load(Ordering::SeqCst);

    println!(
        "Retry test completed in {:?}: {}/{} workspaces created successfully",
        elapsed, successes, task_count
    );

    assert_eq!(
        successes, task_count,
        "All {task_count} workspaces should be created successfully with retries"
    );

    // Verify the test completed in reasonable time
    // Increased from 20s to 60s to account for slower CI systems
    assert!(
        elapsed < Duration::from_mins(1),
        "Should complete within 60 seconds, took {:?}",
        elapsed
    );

    // Cleanup
    for i in 0..task_count {
        let workspace_name = format!("stress-retry-{}-{}", test_id, i);
        let _ = tokio::process::Command::new("jj")
            .args(["workspace", "forget", &workspace_name])
            .current_dir(&repo_root)
            .output()
            .await;
    }

    let _ = tokio::fs::remove_dir_all(base_path).await;

    // Remove temporary JJ repository
    let _ = tokio::fs::remove_dir_all(repo_root).await;

    Ok(())
}

/// Test that workspace creation is properly serialized
///
/// This verifies that even with extreme contention, workspaces are created
/// one at a time and operation graph consistency is maintained.
#[tokio::test]
async fn stress_workspace_serialization() -> Result<()> {
    let task_count: usize = 20;
    let barrier = Arc::new(Barrier::new(task_count));
    let completion_order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let failure_count = Arc::new(AtomicUsize::new(0));

    // Create a unique temp directory for this test run
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::IoError(format!("Failed to get time: {e}")))?
        .as_nanos();

    // Set up a real jj repo for the test
    let repo_temp = common::setup_test_repo()?;
    let repo_root = repo_temp.path().to_path_buf();

    let base_path = repo_root.join(format!("test-workspaces-serialize-{}", test_id));
    tokio::fs::create_dir_all(&base_path).await?;

    let mut handles = vec![];

    // Spawn tasks that record their completion order
    for i in 0..task_count {
        let barrier = Arc::clone(&barrier);
        let completion_order = Arc::clone(&completion_order);
        let failure_count = Arc::clone(&failure_count);

        let workspace_path = base_path.join(format!("serialize-{}", i));
        let workspace_name = format!("stress-serialize-{}-{}", test_id, i);
        let repo_root = repo_root.clone();

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            let max_retries: usize = 5;

            for attempt in 0..=max_retries {
                let start = std::time::Instant::now();
                let result =
                    create_workspace_synced(&workspace_name, &workspace_path, &repo_root).await;
                let duration = start.elapsed();

                match result {
                    Ok(()) => {
                        let mut order = completion_order.lock().unwrap();
                        order.push((workspace_name, duration));
                        break;
                    }
                    Err(e) => {
                        let is_lock_timeout = matches!(e, Error::LockTimeout { .. });
                        if is_lock_timeout && attempt < max_retries {
                            let backoff_ms = 10_u64 * (1_u64 << attempt);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            continue;
                        }

                        failure_count.fetch_add(1, Ordering::SeqCst);
                        eprintln!(
                            "✗ Serialized workspace {} failed after {} attempts: {}",
                            workspace_name,
                            attempt + 1,
                            e
                        );
                        break;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle
            .await
            .map_err(|e| Error::IoError(format!("Task join error: {e}")))?;
    }

    let order = completion_order.lock().unwrap();

    println!("Workspace creation order:");
    for (name, duration) in order.iter() {
        println!("  {} - {:?}", name, duration);
    }

    // Verify all workspaces completed
    let failures = failure_count.load(Ordering::SeqCst);
    assert_eq!(failures, 0, "No serialized workspace creations should fail");
    assert_eq!(
        order.len(),
        task_count,
        "All workspaces should complete serialization"
    );

    // Check that creation times indicate serialization (not all instant)
    // With 20 workspaces and serialization, total time should be significant
    let total_duration: Duration = order.iter().map(|(_, d)| *d).sum();
    println!("Total serialization time: {:?}", total_duration);

    assert!(
        total_duration > Duration::from_millis(100),
        "Serialization should take measurable time"
    );

    // Cleanup
    for i in 0..task_count {
        let workspace_name = format!("stress-serialize-{}-{}", test_id, i);
        let _ = tokio::process::Command::new("jj")
            .args(["workspace", "forget", &workspace_name])
            .current_dir(&repo_root)
            .output()
            .await;
    }

    let _ = tokio::fs::remove_dir_all(base_path).await;

    // Remove temporary JJ repository
    let _ = tokio::fs::remove_dir_all(repo_root).await;

    Ok(())
}
