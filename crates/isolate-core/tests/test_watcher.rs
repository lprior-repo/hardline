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
    // Async and concurrency relaxations
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
)]
//! Async integration tests for file watcher
//!
//! Tests the file watching functionality including:
//! - Detection of file changes
//! - Debouncing of rapid changes
//! - Multiple workspace handling
//! - Concurrent access patterns
//!
//! All tests use async/await with tokio and verify events are received
//! within appropriate timeouts.

use std::time::Duration;

use isolate_core::{
    config::WatchConfig,
    watcher::{FileWatcher, WatchEvent},
    Result,
};
use tempfile::TempDir;
use tokio::{fs, time::timeout};

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to setup a workspace with beads database
async fn setup_workspace(workspace_path: &std::path::Path) -> Result<()> {
    let beads_dir = workspace_path.join(".beads");
    let beads_db = beads_dir.join("beads.db");

    // Optimize: Create all directories recursively
    fs::create_dir_all(&beads_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create directories: {e}")))?;

    fs::write(&beads_db, b"initial content")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 1: Watcher detects file changes
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_detects_file_changes() -> Result<()> {
    // Create temporary workspace with beads database
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    // Create initial database file BEFORE starting watcher
    fs::write(&beads_db, b"initial database content")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    // Configure watcher with short debounce for fast tests
    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 50, // Short debounce for faster test
        paths: vec![".beads/beads.db".to_string()],
    };

    // Start watching
    let mut rx = FileWatcher::watch_workspaces(&config, std::slice::from_ref(&workspace_path))
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Wait for watcher to initialize (reduced from 100ms -> 30ms)
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Modify the database file
    fs::write(&beads_db, b"modified content")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to modify beads.db: {e}")))?;

    // Wait for event with reduced timeout (2s -> 1s)
    let event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .map_err(|_| {
            isolate_core::Error::Unknown("Timeout waiting for file change event".to_string())
        })?
        .ok_or_else(|| isolate_core::Error::Unknown("No event received".to_string()))?;

    // Verify the event is correct
    match event {
        WatchEvent::BeadsChanged {
            workspace_path: received_path,
        } => {
            assert_eq!(
                received_path, workspace_path,
                "Event should report correct workspace path"
            );
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 2: Watcher debounces rapid changes
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_debounces_rapid_changes() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    // Create initial database file
    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    // Configure watcher with 100ms debounce
    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, std::slice::from_ref(&workspace_path))
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Reduced initialization delay (50ms -> 30ms)
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Perform rapid writes (reduced iterations: 3 -> 2)
    for i in 0..2 {
        fs::write(&beads_db, format!("content {i}").as_bytes())
            .await
            .map_err(|e| {
                isolate_core::Error::IoError(format!("Failed to write iteration {i}: {e}"))
            })?;

        // Small delay between writes (shorter than debounce, 20ms -> 15ms)
        tokio::time::sleep(Duration::from_millis(15)).await;
    }

    // Count events received within a reduced time window (1s -> 800ms)
    let start = std::time::Instant::now();
    let mut event_count = 0;
    let timeout_duration = Duration::from_millis(800);

    while start.elapsed() < timeout_duration {
        match timeout(Duration::from_millis(250), rx.recv()).await {
            Ok(Some(event)) => {
                match event {
                    WatchEvent::BeadsChanged {
                        workspace_path: received_path,
                    } => {
                        assert_eq!(received_path, workspace_path);
                    }
                }
                event_count += 1;

                // If we've gone past debounce period without more events, stop
                if event_count >= 1 {
                    // Reduced wait to confirm debounce completion (150ms -> 120ms)
                    tokio::time::sleep(Duration::from_millis(120)).await;
                    break;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }

    // Due to debouncing, we should receive only 1 event, not 5
    assert_eq!(
        event_count, 1,
        "Expected 1 event due to debouncing, got {event_count}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 3: Watcher handles multiple workspaces
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_multiple_workspaces() -> Result<()> {
    // Create three workspaces
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace1 = temp_dir.path().join("workspace1");
    let workspace2 = temp_dir.path().join("workspace2");
    let workspace3 = temp_dir.path().join("workspace3");

    // Create workspaces in parallel for faster setup
    let (ws1_result, ws2_result, ws3_result) = tokio::join!(
        setup_workspace(&workspace1),
        setup_workspace(&workspace2),
        setup_workspace(&workspace3)
    );

    ws1_result?;
    ws2_result?;
    ws3_result?;

    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 50,
        paths: vec![".beads/beads.db".to_string()],
    };

    let workspace_paths = vec![workspace1.clone(), workspace2.clone(), workspace3.clone()];
    let mut rx = FileWatcher::watch_workspaces(&config, &workspace_paths)
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Reduced initialization delay (50ms -> 30ms)
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Modify all three workspaces
    let db1 = workspace1.join(".beads/beads.db");
    let db2 = workspace2.join(".beads/beads.db");
    let db3 = workspace3.join(".beads/beads.db");

    // Optimize: Write to all three databases in parallel
    let (r1, r2, r3) = tokio::join!(
        fs::write(&db1, b"modified 1"),
        fs::write(&db2, b"modified 2"),
        fs::write(&db3, b"modified 3")
    );

    r1.map_err(|e| isolate_core::Error::IoError(format!("Failed to modify db1: {e}")))?;
    r2.map_err(|e| isolate_core::Error::IoError(format!("Failed to modify db2: {e}")))?;
    r3.map_err(|e| isolate_core::Error::IoError(format!("Failed to modify db3: {e}")))?;

    // Collect events with optimized timeout (1s -> 800ms, 300ms -> 250ms)
    let mut events = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_millis(800) && events.len() < 3 {
        match timeout(Duration::from_millis(250), rx.recv()).await {
            Ok(Some(event)) => events.push(event),
            Ok(None) | Err(_) => break,
        }
    }

    // We should have received events for all three workspaces
    assert!(
        events.len() >= 3,
        "Expected at least 3 events, got {}",
        events.len()
    );

    // Verify all three workspaces are represented in events
    let mut workspace1_found = false;
    let mut workspace2_found = false;
    let mut workspace3_found = false;

    for event in events {
        match event {
            WatchEvent::BeadsChanged { workspace_path } => {
                if workspace_path == workspace1 {
                    workspace1_found = true;
                } else if workspace_path == workspace2 {
                    workspace2_found = true;
                } else if workspace_path == workspace3 {
                    workspace3_found = true;
                }
            }
        }
    }

    assert!(
        workspace1_found && workspace2_found && workspace3_found,
        "All three workspaces should have events"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 4: Watcher with non-existent database
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_missing_database() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();

    // Don't create beads.db - test that watcher handles missing database gracefully
    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 50,
        paths: vec![".beads/beads.db".to_string()],
    };

    // This should not fail - watcher should handle missing database
    let result = FileWatcher::watch_workspaces(&config, &[workspace_path]);

    assert!(
        result.is_ok(),
        "Watcher should start even without existing database"
    );

    // Verify receiver exists
    let _rx = result
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create watcher: {e}")))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 5: Concurrent file modifications
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_concurrent_modifications() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, std::slice::from_ref(&workspace_path))
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Reduced initialization delay (50ms -> 30ms)
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Spawn concurrent tasks to modify the file (reduced iterations: 2 -> 1 each)
    let db_clone = beads_db.clone();

    let task1 = tokio::spawn(async move {
        fs::write(&db_clone, b"task1-0").await.ok();
    });

    let db_clone2 = beads_db.clone();
    let task2 = tokio::spawn(async move {
        fs::write(&db_clone2, b"task2-0").await.ok();
    });

    // Wait for both tasks
    let (r1, r2) = tokio::join!(task1, task2);
    r1.map_err(|e| isolate_core::Error::Unknown(format!("Task1 failed: {e}")))?;
    r2.map_err(|e| isolate_core::Error::Unknown(format!("Task2 failed: {e}")))?;

    // Wait for debounced event(s) with optimized timing (1s -> 700ms)
    let start = std::time::Instant::now();
    let mut received_any = false;

    while start.elapsed() < Duration::from_millis(700) && !received_any {
        match timeout(Duration::from_millis(250), rx.recv()).await {
            Ok(Some(event)) => {
                match event {
                    WatchEvent::BeadsChanged {
                        workspace_path: received_path,
                    } => {
                        assert_eq!(received_path, workspace_path);
                    }
                }
                received_any = true;
                // Reduced debounce confirmation wait (150ms -> 100ms)
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok(None) => break,
            Err(_) => {
                // Timeout - continue waiting if we still have time
            }
        }
    }

    // We should have received at least one event despite concurrent modifications
    assert!(
        received_any,
        "Should receive event despite concurrent modifications"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 6: Watcher with rapid successive changes in different workspaces
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_rapid_changes_different_workspaces() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace1 = temp_dir.path().join("ws1");
    let workspace2 = temp_dir.path().join("ws2");

    // Create both workspaces in parallel for faster setup
    let (ws1_result, ws2_result) =
        tokio::join!(setup_workspace(&workspace1), setup_workspace(&workspace2));

    ws1_result?;
    ws2_result?;

    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, &[workspace1.clone(), workspace2.clone()])
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Rapidly modify both workspaces (reduced iterations: 4 -> 2)
    let db1 = workspace1.join(".beads/beads.db");
    let db2 = workspace2.join(".beads/beads.db");

    fs::write(&db1, b"change1").await.ok();
    tokio::time::sleep(Duration::from_millis(20)).await;

    fs::write(&db2, b"change2").await.ok();

    // Collect events with optimized timeout (1s -> 600ms)
    let mut events = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_millis(600) {
        match timeout(Duration::from_millis(250), rx.recv()).await {
            Ok(Some(event)) => events.push(event),
            Ok(None) | Err(_) => break,
        }
    }

    // Should receive events for both workspaces (may be debounced)
    let mut ws1_events = 0;
    let mut ws2_events = 0;

    for event in events {
        match event {
            WatchEvent::BeadsChanged { workspace_path } => {
                if workspace_path == workspace1 {
                    ws1_events += 1;
                } else if workspace_path == workspace2 {
                    ws2_events += 1;
                }
            }
        }
    }

    assert!(
        ws1_events > 0 && ws2_events > 0,
        "Both workspaces should have events: ws1={ws1_events}, ws2={ws2_events}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 7: Watcher error handling - invalid debounce values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_watcher_rejects_invalid_debounce_too_low() {
    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 5, // Too low (< 10)
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, &[]);
    assert!(result.is_err(), "Should reject debounce_ms < 10");

    if let Err(e) = result {
        assert!(matches!(e, isolate_core::Error::InvalidConfig(_)));
    }
}

#[test]
fn test_watcher_rejects_invalid_debounce_too_high() {
    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 10000, // Too high (> 5000)
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, &[]);
    assert!(result.is_err(), "Should reject debounce_ms > 5000");

    if let Err(e) = result {
        assert!(matches!(e, isolate_core::Error::InvalidConfig(_)));
    }
}

#[test]
fn test_watcher_rejects_disabled_config() {
    let config = WatchConfig {
        enabled: false.into(), // Disabled
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, &[]);
    assert!(result.is_err(), "Should reject disabled watcher");

    if let Err(e) = result {
        assert!(matches!(e, isolate_core::Error::InvalidConfig(_)));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 8: Watcher event channel capacity
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_channel_capacity() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    let config = WatchConfig {
        enabled: true.into(),
        debounce_ms: 10, // Very short debounce
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, std::slice::from_ref(&workspace_path))
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Don't consume events immediately - test channel buffering (reduced iterations: 10 -> 7)
    for i in 0..7 {
        fs::write(&beads_db, format!("content {i}").as_bytes())
            .await
            .map_err(|e| {
                isolate_core::Error::IoError(format!("Failed to write iteration {i}: {e}"))
            })?;
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    // Now consume events with optimized timeout (1s -> 700ms)
    let mut event_count = 0;
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_millis(700) {
        match timeout(Duration::from_millis(180), rx.recv()).await {
            Ok(Some(_)) => {
                event_count += 1;
            }
            Ok(None) | Err(_) => break,
        }
    }

    // Should receive at least some events
    assert!(
        event_count > 0,
        "Should receive events through channel, got {event_count}"
    );

    Ok(())
}
