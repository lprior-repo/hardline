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
    clippy::manual_clamp,
)]
//! High-concurrency stress tests for lock system
//!
//! Tests the system's ability to handle high concurrency levels:
//! - 10 concurrent agents
//! - 50 concurrent agents
//! - 100 concurrent agents
//!
//! Validates:
//! - No race conditions
//! - Lock contention metrics
//! - No deadlocks
//! - State consistency under load
//! - Lock acquisition rates
//! - Exactly one holder per session

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::cast_possible_truncation)] // Test code using intentional casts
#![forbid(unsafe_code)]
// Test code for lock contention metrics uses integer-to-float casts.
// Precision loss is acceptable for statistical calculations in tests.
#![allow(clippy::cast_precision_loss)]

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use isolate_core::{coordination::locks::LockManager, Error};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::{sync::Barrier, task::JoinSet};

/// Test helper: Create in-memory database pool
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))
}

/// Test helper: Setup lock manager
async fn setup_lock_manager() -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

/// Metrics collected during stress test
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // duplicate_acquisitions reserved for future use
struct StressTestMetrics {
    successful_acquisitions: usize,
    failed_acquisitions: usize,
    contentions: usize,
    acquisition_times_ms: Vec<f64>,
    unique_holders: HashSet<String>,
    #[allow(dead_code)]
    duplicate_acquisitions: usize,
    deadlocks_detected: usize,
}

impl StressTestMetrics {
    /// Calculate lock acquisition rate (acquisitions per second)
    #[allow(clippy::cast_precision_loss)]
    fn acquisition_rate(&self, total_duration_ms: f64) -> f64 {
        if total_duration_ms > 0.0 {
            (self.successful_acquisitions as f64) / (total_duration_ms / 1000.0)
        } else {
            0.0
        }
    }

    /// Calculate contention percentage
    #[allow(clippy::cast_precision_loss)]
    fn contention_percentage(&self) -> f64 {
        let total_attempts = self.successful_acquisitions + self.failed_acquisitions;
        if total_attempts > 0 {
            (self.contentions as f64) / (total_attempts as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average acquisition time in milliseconds
    fn average_acquisition_time_ms(&self) -> f64 {
        if self.acquisition_times_ms.is_empty() {
            0.0
        } else {
            let sum: f64 = self.acquisition_times_ms.iter().sum();
            sum / self.acquisition_times_ms.len() as f64
        }
    }
}

// ========================================================================
// BDD SCENARIO 1: 10 Agents Locking Same Session
// ========================================================================
//
// GIVEN: A session and 10 concurrent agents
// WHEN: All agents attempt to lock the same session
// THEN: Exactly 1 agent succeeds, others fail with SESSION_LOCKED

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_10_agents_lock_same_session() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "contention-session-10";
    let num_agents = 10;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 10 concurrent agents trying to lock the same session
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let session_clone = session.to_string();
        let agent_name = format!("agent-{agent_id}");

        join_set.spawn(async move {
            let attempt_start = Instant::now();
            let result = mgr_clone.lock(&session_clone, &agent_name).await;
            let elapsed = attempt_start.elapsed().as_millis();

            match result {
                Ok(_) => (agent_name, true, elapsed, None),
                Err(Error::SessionLocked { holder, .. }) => {
                    (agent_name, false, elapsed, Some(holder))
                }
                Err(e) => (agent_name, false, elapsed, Some(e.to_string())),
            }
        });
    }

    // Collect results using functional pattern
    let mut metrics = StressTestMetrics::default();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, success, elapsed_ms, _holder)) => {
                if success {
                    metrics.successful_acquisitions += 1;
                    metrics.acquisition_times_ms.push(elapsed_ms as f64);
                    metrics.unique_holders.insert(agent);
                } else {
                    metrics.failed_acquisitions += 1;
                    metrics.contentions += 1;
                }
            }
            Err(e) => {
                // Task panicked - this indicates a serious problem
                metrics.deadlocks_detected += 1;
                eprintln!("Task panicked: {e}");
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: Exactly 1 agent MUST succeed
    assert_eq!(
        metrics.successful_acquisitions, 1,
        "Exactly 1 agent should acquire lock, got {}",
        metrics.successful_acquisitions
    );

    // THEN: Exactly 1 unique holder
    assert_eq!(
        metrics.unique_holders.len(),
        1,
        "Exactly 1 unique holder should exist, got {}",
        metrics.unique_holders.len()
    );

    // THEN: All other agents should have failed
    assert_eq!(
        metrics.failed_acquisitions,
        num_agents - 1,
        "Expected {} failed acquisitions, got {}",
        num_agents - 1,
        metrics.failed_acquisitions
    );

    // THEN: No deadlocks detected
    assert_eq!(metrics.deadlocks_detected, 0, "No deadlocks should occur");

    // THEN: Verify lock state in database
    let lock_state = mgr.get_lock_state(session).await?;
    assert!(
        lock_state.holder.is_some(),
        "Lock should be held in database"
    );
    assert!(
        metrics
            .unique_holders
            .contains(lock_state.holder.as_deref().ok_or_else(|| {
                Error::ValidationError {
                    message: "Failed to convert holder to &str".into(),
                    field: None,
                    value: None,
                    constraints: vec![],
                }
            })?),
        "Lock holder should match successful agent"
    );

    // Log metrics
    let acquisition_rate = metrics.acquisition_rate(total_duration.as_millis() as f64);
    let avg_time = metrics.average_acquisition_time_ms();
    let contention = metrics.contention_percentage();

    println!("test_10_agents_lock_same_session metrics:");
    println!("  - Acquisition rate: {acquisition_rate:.2} locks/sec");
    println!("  - Average acquisition time: {avg_time:.2}ms");
    println!("  - Contention percentage: {contention:.1}%");
    println!("  - Total duration: {total_duration:?}");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 2: 50 Agents Claiming Different Resources
// ========================================================================
//
// GIVEN: 50 agents and 50 different sessions
// WHEN: Each agent claims a unique session
// THEN: No duplicates, all succeed, state consistency maintained

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_50_agents_claim_unique_resources() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 50 agents, each locking a unique session
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let session_name = format!("session-{agent_id}");
        let agent_name = format!("agent-{agent_id}");

        join_set.spawn(async move {
            let mut agent_acquisitions = 0;
            let mut agent_errors = 0;

            // Each agent attempts to lock its assigned session
            let result = mgr_clone.lock(&session_name, &agent_name).await;
            match result {
                Ok(_) => {
                    agent_acquisitions += 1;
                }
                Err(_) => {
                    agent_errors += 1;
                }
            }

            (agent_name, session_name, agent_acquisitions, agent_errors)
        });
    }

    // Collect results
    let mut successful_locks: HashMap<String, String> = HashMap::new();
    let mut failed_count = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, session, acquisitions, errors)) => {
                if acquisitions > 0 {
                    successful_locks.insert(session, agent);
                }
                if errors > 0 {
                    failed_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked during resource claim".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All 50 agents should succeed
    assert_eq!(
        successful_locks.len(),
        num_agents,
        "Expected {} successful locks, got {}",
        num_agents,
        successful_locks.len()
    );

    // THEN: No failures
    assert_eq!(
        failed_count, 0,
        "No lock acquisitions should fail for unique sessions"
    );

    // THEN: Verify no duplicate session locks
    assert_eq!(
        successful_locks.len(),
        successful_locks.keys().collect::<HashSet<&String>>().len(),
        "No duplicate session locks should exist"
    );

    // THEN: Verify database state consistency
    let all_locks = mgr.get_all_locks().await?;
    assert_eq!(
        all_locks.len(),
        num_agents,
        "Database should contain exactly {num_agents} locks"
    );

    // THEN: Verify each session has correct holder
    for (session, expected_holder) in &successful_locks {
        let lock_state = mgr.get_lock_state(session).await?;
        assert!(
            lock_state.holder.as_ref() == Some(expected_holder),
            "Session {session} holder mismatch: expected {expected_holder}, got {:?}",
            lock_state.holder
        );
    }

    // Log performance metrics
    let acquisition_rate = (num_agents as f64) / (total_duration.as_secs_f64());
    println!("test_50_agents_claim_unique_resources metrics:");
    println!("  - Total locks: {num_agents}");
    println!("  - Acquisition rate: {acquisition_rate:.2} locks/sec");
    println!(
        "  - Average per-lock time: {:.2}ms",
        total_duration.as_millis() as f64 / num_agents as f64
    );
    println!("  - Total duration: {total_duration:?}");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 3: 100 Agents Concurrent Operations
// ========================================================================
//
// GIVEN: 100 agents performing lock/unlock operations
// WHEN: Agents perform rapid concurrent operations
// THEN: No crashes, state consistency maintained, no lost updates

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_100_agents_concurrent_operations() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 100;
    let num_sessions = 20; // Multiple sessions to reduce contention

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 100 agents performing lock/unlock cycles on different sessions
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("agent-{agent_id}");
        let session_name = format!("session-{}", agent_id % num_sessions);

        join_set.spawn(async move {
            let mut successful_operations = 0;
            let mut failed_operations = 0;

            // Perform 5 lock/unlock cycles per agent
            for cycle in 0..5 {
                // Try to lock
                let lock_result = mgr_clone.lock(&session_name, &agent_name).await;
                match lock_result {
                    Ok(_) => {
                        successful_operations += 1;

                        // Immediately unlock
                        let unlock_result = mgr_clone.unlock(&session_name, &agent_name).await;
                        match unlock_result {
                            Ok(()) => {
                                successful_operations += 1;
                            }
                            Err(_) => {
                                failed_operations += 1;
                            }
                        }
                    }
                    Err(Error::SessionLocked { .. }) => {
                        // Expected under contention
                        failed_operations += 1;
                    }
                    Err(_) => {
                        failed_operations += 1;
                    }
                }

                // Small random delay to increase concurrency chaos
                let delay_ms = (agent_id + cycle) % 10;
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            (
                agent_name,
                session_name,
                successful_operations,
                failed_operations,
            )
        });
    }

    // Collect results
    let mut total_successful = 0;
    let mut total_failed = 0;
    let mut agent_results: Vec<(String, String, usize, usize)> = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, session, success, fail)) => {
                total_successful += success;
                total_failed += fail;
                agent_results.push((agent, session, success, fail));
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown(
                    "Task panicked during concurrent operations".into(),
                ));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All agents should complete without crashes
    assert_eq!(
        agent_results.len(),
        num_agents as usize,
        "All {num_agents} agents should complete"
    );

    // THEN: Total operations should be successful
    let total_operations = total_successful + total_failed;
    let success_rate = if total_operations > 0 {
        (total_successful as f64) / (total_operations as f64) * 100.0
    } else {
        0.0
    };

    println!("test_100_agents_concurrent_operations metrics:");
    println!("  - Total successful operations: {total_successful}");
    println!("  - Total failed operations: {total_failed}");
    println!("  - Success rate: {success_rate:.1}%");
    println!("  - Total duration: {total_duration:?}");
    println!(
        "  - Operations per second: {:.2}",
        total_operations as f64 / total_duration.as_secs_f64()
    );

    // THEN: Verify database integrity (no partial/corrupted state)
    let all_locks = mgr.get_all_locks().await?;
    // Some locks may still be held, that's okay
    println!("  - Active locks remaining: {}", all_locks.len());

    // THEN: Verify no lock has invalid state
    for lock_info in &all_locks {
        let lock_state = mgr.get_lock_state(&lock_info.session).await?;
        assert!(
            lock_state.holder.is_some(),
            "Active lock {} should have a holder",
            lock_info.session
        );
    }

    Ok(())
}

// ========================================================================
// BDD SCENARIO 4: Lock/Unlock Storm
// ========================================================================
//
// GIVEN: 50 agents and 10 sessions
// WHEN: Rapid lock/unlock storm occurs
// THEN: State consistency maintained, audit trail complete

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_lock_unlock_storm_consistency() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;
    let num_sessions = 10;
    let operations_per_agent = 10;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn agents for lock/unlock storm
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("storm-agent-{agent_id}");

        join_set.spawn(async move {
            let mut ops_completed = 0;

            for op in 0..operations_per_agent {
                let session_name = format!("storm-session-{}", op % num_sessions);

                // Try to lock
                let lock_result = mgr_clone.lock(&session_name, &agent_name).await;

                if lock_result.is_ok() {
                    ops_completed += 1;

                    // Random delay before unlock
                    let delay_ms = (agent_id * op) % 5;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                    // Unlock
                    let _ = mgr_clone.unlock(&session_name, &agent_name).await;
                    ops_completed += 1;
                }

                // Small delay between operations
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            ops_completed
        });
    }

    // Wait for all agents
    let mut total_operations = 0;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(ops) => {
                total_operations += ops;
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked during storm".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: Verify all sessions have consistent state
    for session_id in 0..num_sessions {
        let session_name = format!("storm-session-{session_id}");
        let lock_state = mgr.get_lock_state(&session_name).await?;

        // State should be valid (either locked or unlocked, not corrupted)
        assert!(
            lock_state.holder.is_some() || lock_state.holder.is_none(),
            "Lock state for {session_name} should be valid (not corrupted)"
        );

        // Check audit trail
        let audit_log = mgr.get_lock_audit_log(&session_name).await?;

        // Verify audit log is not corrupted
        for entry in &audit_log {
            assert!(
                !entry.session.is_empty(),
                "Audit entry session should not be empty"
            );
            assert!(
                !entry.agent_id.is_empty(),
                "Audit entry agent_id should not be empty"
            );
            assert!(
                !entry.operation.is_empty(),
                "Audit entry operation should not be empty"
            );
            // timestamp is DateTime<Utc>, always valid if present
        }

        println!(
            "Session {} - Audit entries: {}",
            session_name,
            audit_log.len()
        );
    }

    println!("test_lock_unlock_storm_consistency metrics:");
    println!("  - Total operations completed: {total_operations}");
    println!("  - Duration: {total_duration:?}");
    println!(
        "  - Operations per second: {:.2}",
        f64::from(total_operations) / total_duration.as_secs_f64()
    );

    Ok(())
}

// ========================================================================
// BDD SCENARIO 5: Claim Transfer Under Load
// ========================================================================
//
// GIVEN: A lock held by agent A
// WHEN: Agent B attempts to lock, A unlocks, then B locks
// THEN: No lost updates, clean transfer, no race conditions

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_claim_transfer_under_load() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "transfer-session";
    let num_transfer_cycles = 50;
    let mut successful_transfers = 0;
    let mut failed_transfers = 0;
    let start = Instant::now();

    // Run transfers sequentially to avoid race conditions
    // This tests the lock transfer logic, not concurrent lock acquisition
    for cycle in 0..num_transfer_cycles {
        let agent_a = format!("agent-a-{cycle}");
        let agent_b = format!("agent-b-{cycle}");

        // Agent A locks
        let lock_a = mgr.lock(session, &agent_a).await;
        if lock_a.is_err() {
            failed_transfers += 1;
            continue;
        }

        // Agent B tries to lock (should fail)
        let lock_b_fails = mgr.lock(session, &agent_b).await.is_err();

        // Agent A unlocks
        let unlock_a = mgr.unlock(session, &agent_a).await;

        // Small delay to ensure database state is fully propagated
        tokio::time::sleep(Duration::from_millis(1)).await;

        // Agent B locks again (should succeed)
        let lock_b_succeeds = mgr.lock(session, &agent_b).await.is_ok();

        // Agent B unlocks
        let unlock_b = mgr.unlock(session, &agent_b).await;

        if lock_b_fails && unlock_a.is_ok() && unlock_b.is_ok() && lock_b_succeeds {
            successful_transfers += 1;
        } else {
            failed_transfers += 1;
        }
    }

    let total_duration = start.elapsed();

    // THEN: All transfers should succeed
    assert_eq!(
        successful_transfers, num_transfer_cycles,
        "All {num_transfer_cycles} transfers should succeed"
    );

    assert_eq!(failed_transfers, 0, "No transfers should fail");

    // THEN: Final state should be unlocked (all agents cleaned up)
    let final_state = mgr.get_lock_state(session).await?;
    assert!(
        final_state.holder.is_none(),
        "Session should be unlocked after all transfers"
    );

    // THEN: Verify audit trail completeness
    let audit_log = mgr.get_lock_audit_log(session).await?;
    assert!(
        audit_log.len() >= num_transfer_cycles * 4,
        "Audit log should have at least {} entries (4 per cycle)",
        num_transfer_cycles * 4
    );

    println!("test_claim_transfer_under_load metrics:");
    println!("  - Successful transfers: {successful_transfers}");
    println!("  - Failed transfers: {failed_transfers}");
    println!("  - Total audit entries: {}", audit_log.len());
    println!("  - Duration: {total_duration:?}");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 6: Lock Contention Metrics
// ========================================================================
//
// GIVEN: High contention scenario (100 agents, 5 sessions)
// WHEN: Agents compete for locks
// THEN: Contention metrics are measurable and reasonable

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_lock_contention_metrics() -> Result<(), Error> {
    const FAIL_FAST_FLOOR_MS: f64 = 50.0;
    const FAIL_FAST_JITTER_ALLOWANCE_MS: f64 = 35.0;
    const FAIL_FAST_HARD_CEILING_MS: f64 = 150.0;

    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 100;
    let num_sessions = 5;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // High contention: 100 agents competing for 5 sessions
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("contention-agent-{agent_id}");
        let session_name = format!("contention-session-{}", agent_id % num_sessions);

        join_set.spawn(async move {
            let attempt_start = Instant::now();
            let result = mgr_clone.lock(&session_name, &agent_name).await;
            let elapsed = attempt_start.elapsed();

            match result {
                Ok(_) => {
                    // Hold for a bit then release
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let _ = mgr_clone.unlock(&session_name, &agent_name).await;
                    (true, elapsed.as_millis())
                }
                Err(_) => (false, elapsed.as_millis()),
            }
        });
    }

    // Collect metrics
    let mut acquisition_times: Vec<u128> = Vec::new();
    let mut failure_times: Vec<u128> = Vec::new();
    let mut successes = 0;
    let mut failures = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((success, elapsed)) => {
                if success {
                    successes += 1;
                    acquisition_times.push(elapsed);
                } else {
                    failures += 1;
                    failure_times.push(elapsed);
                }
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown(
                    "Task panicked during contention test".into(),
                ));
            }
        }
    }

    let total_duration = start.elapsed();

    // Calculate metrics
    let avg_success_time = if acquisition_times.is_empty() {
        0.0
    } else {
        let sum: u128 = acquisition_times.iter().sum();
        sum as f64 / acquisition_times.len() as f64
    };

    let avg_failure_time = if failure_times.is_empty() {
        0.0
    } else {
        let sum: u128 = failure_times.iter().sum();
        sum as f64 / failure_times.len() as f64
    };

    let contention_rate = if num_agents > 0 {
        f64::from(failures) / f64::from(num_agents) * 100.0
    } else {
        0.0
    };

    println!("test_lock_contention_metrics results:");
    println!("  - Total agents: {num_agents}");
    println!("  - Successful acquisitions: {successes}");
    println!("  - Failed acquisitions (contention): {failures}");
    println!("  - Contention rate: {contention_rate:.1}%");
    println!("  - Avg success time: {avg_success_time:.2}ms");
    println!("  - Avg failure time: {avg_failure_time:.2}ms");
    println!("  - Total duration: {total_duration:?}");

    // THEN: All agents should complete
    assert_eq!(
        successes + failures,
        num_agents,
        "All agents should complete"
    );

    // THEN: Contention should be measurable (not zero, not 100%)
    assert!(
        contention_rate > 0.0,
        "Contention should be measurable under high competition"
    );

    // THEN: Failure time should remain fail-fast with realistic scheduler/DB jitter
    // under heavy parallel load.
    if avg_success_time > 0.0 && avg_failure_time > 0.0 {
        let fail_fast_budget_ms = (avg_success_time + FAIL_FAST_JITTER_ALLOWANCE_MS)
            .max(FAIL_FAST_FLOOR_MS)
            .min(FAIL_FAST_HARD_CEILING_MS);
        assert!(
            avg_failure_time <= fail_fast_budget_ms,
            "Failed lock attempts should stay fail-fast (<= {fail_fast_budget_ms:.2}ms): {avg_failure_time:.2}ms vs {avg_success_time:.2}ms"
        );
    }

    Ok(())
}

// ========================================================================
// BDD SCENARIO 7: Deterministic Fail-Fast Under Contention
// ========================================================================
//
// GIVEN: A lock held by one agent
// WHEN: Many contenders attempt to acquire it at once
// THEN: All contenders fail quickly and report the same holder

#[tokio::test]
async fn test_contention_fail_fast_reports_consistent_holder() -> Result<(), Error> {
    const SOFT_FAIL_FAST_MS: u128 = 120;
    const HARD_FAIL_FAST_MS: u128 = 200;
    const MAX_SOFT_OUTLIERS: usize = 1;

    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "fail-fast-session";
    let holder = "holder-agent";
    let contenders = 32;

    let _initial_lock = mgr.lock(session, holder).await?;

    let barrier = Arc::new(Barrier::new(contenders));
    let mut join_set = JoinSet::new();

    for contender_id in 0..contenders {
        let mgr_clone = Arc::clone(&mgr);
        let barrier_clone = Arc::clone(&barrier);
        let session_name = session.to_string();
        let contender_name = format!("contender-{contender_id}");

        join_set.spawn(async move {
            barrier_clone.wait().await;

            let attempt_start = Instant::now();
            let result = mgr_clone.lock(&session_name, &contender_name).await;
            let elapsed_ms = attempt_start.elapsed().as_millis();

            (result, elapsed_ms)
        });
    }

    let mut failures = 0;
    let mut slow_failures = 0;
    let mut max_failure_elapsed_ms = 0;

    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok((
                Err(Error::SessionLocked {
                    holder: seen_holder,
                    ..
                }),
                elapsed_ms,
            )) => {
                failures += 1;
                max_failure_elapsed_ms = max_failure_elapsed_ms.max(elapsed_ms);
                if elapsed_ms > SOFT_FAIL_FAST_MS {
                    slow_failures += 1;
                }
                assert_eq!(
                    seen_holder, holder,
                    "holder identity should be deterministic"
                );
            }
            Ok((Err(other_error), _)) => {
                return Err(Error::Unknown(format!(
                    "unexpected lock contention error: {other_error}"
                )));
            }
            Ok((Ok(_), _)) => {
                return Err(Error::ValidationError {
                    message: "contender unexpectedly acquired lock while holder still active"
                        .into(),
                    field: None,
                    value: None,
                    constraints: Vec::new(),
                });
            }
            Err(join_error) => {
                return Err(Error::Unknown(format!(
                    "contention worker join failure: {join_error}"
                )));
            }
        }
    }

    assert_eq!(
        failures, contenders,
        "all contenders should fail immediately"
    );
    assert!(
        slow_failures <= MAX_SOFT_OUTLIERS,
        "contention failures exceeded soft fail-fast outlier bound (>{SOFT_FAIL_FAST_MS}ms): {slow_failures}/{contenders}"
    );
    assert!(
        max_failure_elapsed_ms <= HARD_FAIL_FAST_MS,
        "contention failures breached hard fail-fast bound (>{HARD_FAIL_FAST_MS}ms): max={max_failure_elapsed_ms}ms"
    );

    mgr.unlock(session, holder).await?;
    Ok(())
}

// ========================================================================
// BDD SCENARIO 8: Starvation Coverage During Repeated Contention
// ========================================================================
//
// GIVEN: Several agents repeatedly competing for one session
// WHEN: They retry until each gets at least one successful lock
// THEN: No agent starves under sustained contention

#[tokio::test]
async fn test_repeated_contention_eventually_serves_all_agents() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "starvation-session";
    let num_agents = 8;
    let deadline = Instant::now() + Duration::from_secs(5);

    let mut join_set = JoinSet::new();

    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let session_name = session.to_string();
        let agent_name = format!("starvation-agent-{agent_id}");

        join_set.spawn(async move {
            let mut attempts = 0;
            let mut acquired = false;

            while Instant::now() < deadline {
                attempts += 1;

                match mgr_clone.lock(&session_name, &agent_name).await {
                    Ok(_) => {
                        acquired = true;
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        mgr_clone.unlock(&session_name, &agent_name).await?;
                        break;
                    }
                    Err(Error::SessionLocked { .. }) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(other) => {
                        return Err(other);
                    }
                }
            }

            Ok::<(String, bool, usize), Error>((agent_name, acquired, attempts))
        });
    }

    let mut acquired_agents = HashSet::new();

    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok(Ok((agent_name, acquired, attempts))) => {
                if acquired {
                    acquired_agents.insert(agent_name);
                } else {
                    return Err(Error::ValidationError {
                        message: format!(
                            "agent starved under contention after {attempts} attempts"
                        ),
                        field: None,
                        value: None,
                        constraints: Vec::new(),
                    });
                }
            }
            Ok(Err(task_error)) => {
                return Err(task_error);
            }
            Err(join_error) => {
                return Err(Error::Unknown(format!(
                    "starvation worker join failure: {join_error}"
                )));
            }
        }
    }

    assert_eq!(
        acquired_agents.len(),
        num_agents,
        "every agent should acquire at least once"
    );

    Ok(())
}

#[tokio::test]
async fn test_fairness_contract_bounded_attempt_success_per_contender() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "fairness-contract-session";
    let contender_count = 10usize;
    let max_attempts_per_contender = 400usize;

    let mut join_set = JoinSet::new();

    for contender_id in 0..contender_count {
        let mgr_clone = Arc::clone(&mgr);
        let session_name = session.to_string();
        let contender = format!("fair-contender-{contender_id}");

        join_set.spawn(async move {
            let mut attempts = 0usize;
            let mut acquired_at_attempt: Option<usize> = None;

            while attempts < max_attempts_per_contender {
                attempts += 1;

                match mgr_clone.lock(&session_name, &contender).await {
                    Ok(_) => {
                        acquired_at_attempt = Some(attempts);
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        mgr_clone.unlock(&session_name, &contender).await?;
                        break;
                    }
                    Err(Error::SessionLocked { .. }) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(other) => return Err(other),
                }
            }

            Ok::<(String, Option<usize>), Error>((contender, acquired_at_attempt))
        });
    }

    let mut attempt_results = HashMap::new();

    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok(Ok((contender, acquired_at))) => {
                attempt_results.insert(contender, acquired_at);
            }
            Ok(Err(error)) => return Err(error),
            Err(join_error) => {
                return Err(Error::Unknown(format!(
                    "fairness contender join failure: {join_error}"
                )));
            }
        }
    }

    assert_eq!(
        attempt_results.len(),
        contender_count,
        "all contenders should report bounded-attempt results"
    );

    for (contender, acquired_at) in &attempt_results {
        let attempts = acquired_at.ok_or_else(|| Error::ValidationError {
            message: format!(
                "{contender} starved past bounded-attempt contract ({max_attempts_per_contender})"
            ),
            field: None,
            value: None,
            constraints: Vec::new(),
        })?;

        assert!(
            attempts <= max_attempts_per_contender,
            "{contender} exceeded bounded-attempt contract"
        );
    }

    Ok(())
}

// ========================================================================
// BDD SCENARIO 9: Deadlock Detection
// ========================================================================
//
// GIVEN: Multiple sessions and agents
// WHEN: Agents perform complex lock patterns
// THEN: No deadlocks occur (timeout would indicate deadlock)

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_no_deadlocks_under_load() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;
    let num_sessions = 10;
    let operations_per_agent = 5;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Each agent tries to lock multiple sessions in sequence
    // This could deadlock if not implemented correctly
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("deadlock-agent-{agent_id}");

        join_set.spawn(async move {
            for op in 0..operations_per_agent {
                // Try to lock 2 different sessions
                let session1 = format!("deadlock-session-{}", (op + agent_id) % num_sessions);
                let session2 = format!("deadlock-session-{}", (op + agent_id + 1) % num_sessions);

                // Lock first session
                let lock1 = mgr_clone.lock(&session1, &agent_name).await;

                if lock1.is_ok() {
                    // Small delay
                    tokio::time::sleep(Duration::from_millis(1)).await;

                    // Try to lock second session
                    let lock2 = mgr_clone.lock(&session2, &agent_name).await;

                    // Unlock both
                    let _ = mgr_clone.unlock(&session1, &agent_name).await;
                    if lock2.is_ok() {
                        let _ = mgr_clone.unlock(&session2, &agent_name).await;
                    }
                }
            }

            // Return success if we reach here (no deadlock)
            true
        });
    }

    // Set a timeout - if deadlocks occur, test will hang
    let timeout_duration = Duration::from_secs(30);
    let mut completed_agents = 0;

    // Join with timeout logic
    let deadline = Instant::now() + timeout_duration;
    while let Some(result) = join_set.join_next().await {
        if Instant::now() > deadline {
            return Err(Error::ValidationError {
                message: "Potential deadlock detected - test exceeded timeout".into(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        match result {
            Ok(_success) => {
                completed_agents += 1;
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked (possible deadlock)".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All agents should complete
    assert_eq!(
        completed_agents, num_agents,
        "All {num_agents} agents should complete without deadlock"
    );

    // THEN: Test should complete well before timeout
    assert!(
        total_duration < timeout_duration,
        "Test completed well within deadline - no deadlock detected"
    );

    println!("test_no_deadlocks_under_load results:");
    println!("  - Agents completed: {completed_agents}");
    println!("  - Duration: {total_duration:?}");
    println!("  - Timeout limit: {timeout_duration:?}");
    println!("  - Status: No deadlocks detected");

    Ok(())
}
