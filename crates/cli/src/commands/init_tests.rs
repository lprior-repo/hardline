//! Proptest coverage for the init command and lock invariants.
//!
//! Tests organized by the four required categories from hl-8qu:
//! 1. Lock invariant tests (no double acquire)
//! 2. Stale lock removal tests
//! 3. State machine transition tests
//! 4. Edge case path handling tests

use proptest::{prelude::*, prop_assert};
use serial_test::serial;
use tempfile::{NamedTempFile, TempDir};

use crate::commands::lock::{
    acquire_with_path, heartbeat_with_path, list_with_path, release_with_path, status_with_path,
};

fn get_temp_db() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp db")
}

// ========================================================================
// 1. Lock invariant tests (no double acquire)
// ========================================================================

proptest! {
    /// Property: Acquiring a lock then acquiring again with a *different* agent
    /// always returns an error (SessionLocked). This holds for any valid
    /// session name (non-empty, <= 255 chars) and any distinct agent pair.
    #[test]
    fn prop_no_double_acquire_distinct_agents(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        // Skip when agents are the same (idempotent re-lock is allowed)
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res1 = acquire_with_path(&session, &agent_a, None, path);
        prop_assert!(res1.is_ok(), "First acquire failed: {:?}", res1);

        let res2 = acquire_with_path(&session, &agent_b, None, path);
        prop_assert!(res2.is_err(), "Second acquire with different agent should fail");
    }
}

proptest! {
    /// Property: The same agent can re-acquire a lock idempotently.
    /// The lock remains held and the operation succeeds.
    #[test]
    fn prop_same_agent_relock_is_idempotent(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res1 = acquire_with_path(&session, &agent, None, path);
        prop_assert!(res1.is_ok(), "First acquire failed: {:?}", res1);

        let res2 = acquire_with_path(&session, &agent, None, path);
        prop_assert!(res2.is_ok(), "Re-lock by same agent should succeed (idempotent): {:?}", res2);
    }
}

proptest! {
    /// Property: After agent A acquires and releases, agent B can acquire.
    /// The lock system must not leave stale state after release.
    #[test]
    fn prop_acquire_release_allows_new_holder(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent_a, None, path).expect("first acquire");
        let release_res = release_with_path(&session, &agent_a, path);
        prop_assert!(release_res.is_ok(), "Release failed: {:?}", release_res);

        let res_b = acquire_with_path(&session, &agent_b, None, path);
        prop_assert!(res_b.is_ok(), "Agent B should acquire after A released: {:?}", res_b);
    }
}

// ========================================================================
// 2. Stale lock removal tests
// ========================================================================

proptest! {
    /// Property: A lock with TTL=1 second becomes stale after sleeping.
    /// A different agent can then acquire the lock (stale cleanup is automatic).
    #[test]
    fn prop_expired_lock_is_stale(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent_a, Some(1), path).expect("first acquire");

        // Wait for lock to expire
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let res_b = acquire_with_path(&session, &agent_b, None, path);
        prop_assert!(res_b.is_ok(), "Agent B should acquire after lock expired: {:?}", res_b);

        // The expired agent can no longer heartbeat
        let hb_res = heartbeat_with_path(&session, &agent_a, path);
        prop_assert!(hb_res.is_err(), "Heartbeat for expired lock should fail");
    }
}

proptest! {
    /// Property: A heartbeat on a valid lock extends its TTL so that it does
    /// not expire within the original TTL window. After heartbeat, the lock
    /// holder can still operate even if the original TTL would have passed.
    #[test]
    fn prop_heartbeat_extends_lock_past_original_ttl(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent, Some(2), path).expect("first acquire");

        // Wait 500ms (within original TTL)
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Heartbeat to extend TTL
        let hb_res = heartbeat_with_path(&session, &agent, path);
        prop_assert!(hb_res.is_ok(), "Heartbeat should succeed for active lock: {:?}", hb_res);

        // Wait past original TTL
        std::thread::sleep(std::time::Duration::from_millis(1600));

        // Lock should still be valid because heartbeat extended it
        let hb_res2 = heartbeat_with_path(&session, &agent, path);
        prop_assert!(hb_res2.is_ok(), "Heartbeat should still work after extension");
    }
}

// ========================================================================
// 3. State machine transition tests
// ========================================================================

proptest! {
    /// Property: unlocked -> locked transition always succeeds for any
    /// valid session/agent pair.
    #[test]
    fn prop_lock_state_machine_unlocked_to_locked(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // unlocked -> locked: should succeed
        let res = acquire_with_path(&session, &agent, None, path);
        prop_assert!(res.is_ok(), "unlocked->locked should succeed: {:?}", res);
    }
}

proptest! {
    /// Property: Non-holder cannot unlock a lock (locked -> unlocked
    /// transition requires the holder agent).
    #[test]
    fn prop_lock_state_machine_locked_unlocked_requires_holder(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent_a, None, path).expect("setup");

        // Non-holder unlock must fail
        let unlock_res = release_with_path(&session, &agent_b, path);
        prop_assert!(unlock_res.is_err(), "Non-holder unlock should fail");

        // Original holder should still be able to heartbeat (lock still held)
        let hb_res = heartbeat_with_path(&session, &agent_a, path);
        prop_assert!(hb_res.is_ok(), "Holder should still hold lock");
    }
}

proptest! {
    /// Property: Double-unlock is idempotent (unlocked -> unlocked is valid).
    /// The system must not error on unlocking an already-unlocked session.
    #[test]
    fn prop_lock_state_machine_double_unlock_is_idempotent(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // Acquire then release
        acquire_with_path(&session, &agent, None, path).expect("first acquire");
        release_with_path(&session, &agent, path).expect("first release");

        // Second release (double-unlock) must succeed
        let double_release = release_with_path(&session, &agent, path);
        prop_assert!(double_release.is_ok(), "Double-unlock should be idempotent: {:?}", double_release);
    }
}

proptest! {
    /// Property: After a full lock lifecycle (acquire -> heartbeat -> release),
    /// the session returns to unlocked state and a different agent can acquire.
    #[test]
    fn prop_lock_full_lifecycle_returns_to_unlocked(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // Full lifecycle: acquire -> heartbeat -> release
        acquire_with_path(&session, &agent_a, Some(60), path).expect("acquire");
        let hb = heartbeat_with_path(&session, &agent_a, path);
        prop_assert!(hb.is_ok(), "Heartbeat in lifecycle should succeed: {:?}", hb);
        let rel = release_with_path(&session, &agent_a, path);
        prop_assert!(rel.is_ok(), "Release in lifecycle should succeed: {:?}", rel);

        // Verify unlocked state
        let status_res = status_with_path(&session, path);
        prop_assert!(status_res.is_ok(), "Status query should succeed");

        // New agent can acquire
        let new_acquire = acquire_with_path(&session, &agent_b, None, path);
        prop_assert!(new_acquire.is_ok(), "New agent should acquire after lifecycle: {:?}", new_acquire);
    }
}

// ========================================================================
// 4. Edge case path handling tests
// ========================================================================

proptest! {
    /// Property: Empty VCS type always returns config_invalid error.
    #[test]
    #[serial]
    fn prop_empty_vcs_type_returns_config_error(vcs_type in "") {
        // Use a temp dir so current_dir() succeeds even when other tests
        // are racing on set_current_dir.
        let original_dir = std::env::current_dir().ok();
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir");

        let result = crate::commands::init::run(&vcs_type);
        prop_assert!(result.is_err(), "Empty VCS type should return error");
        let err = result.unwrap_err().to_string();
        prop_assert!(err.contains("Unknown VCS type"), "Error should mention unknown VCS: {err}");

        // Restore cwd before TempDir drops
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
        if std::env::current_dir().is_err() {
            if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
                std::env::set_current_dir(manifest).ok();
            }
        }
    }
}

proptest! {
    /// Property: Any VCS type other than "git" returns config_invalid.
    #[test]
    #[serial]
    fn prop_unknown_vcs_type_returns_config_error(
        vcs_type in "[A-Za-z0-9_-]{1,50}"
    ) {
        // Skip the two valid types
        prop_assume!(vcs_type != "git");

        // Use a temp dir so current_dir() succeeds even when other tests
        // are racing on set_current_dir.
        let original_dir = std::env::current_dir().ok();
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir");

        let result = crate::commands::init::run(&vcs_type);
        prop_assert!(result.is_err(), "Unknown VCS type '{vcs_type}' should return error");
        let err = result.unwrap_err().to_string();
        prop_assert!(err.contains("Unknown VCS type"), "Error should mention unknown VCS type: {err}");
        prop_assert!(err.contains(&vcs_type), "Error should include the invalid type: {err}");

        // Restore cwd before TempDir drops
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
        if std::env::current_dir().is_err() {
            if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
                std::env::set_current_dir(manifest).ok();
            }
        }
    }
}

proptest! {
    /// Property: Empty session name always fails lock acquisition.
    #[test]
    fn prop_empty_session_name_rejected(
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path("", &agent, None, path);
        prop_assert!(res.is_err(), "Empty session name should be rejected");
    }
}

proptest! {
    /// Property: Empty agent ID always fails lock acquisition.
    #[test]
    fn prop_empty_agent_id_rejected(
        session in "[a-z0-9_-]{1,255}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path(&session, "", None, path);
        prop_assert!(res.is_err(), "Empty agent ID should be rejected");
    }
}

proptest! {
    /// Property: Session names at exactly 255 characters are accepted.
    #[test]
    fn prop_session_name_boundary_255(
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // At boundary: 255 chars should succeed
        let session_max = "s".repeat(255);
        let res_max = acquire_with_path(&session_max, &agent, None, path);
        prop_assert!(res_max.is_ok(), "Session name of 255 chars should succeed");
    }
}

proptest! {
    /// Property: Session names longer than 255 characters are always rejected.
    #[test]
    fn prop_session_name_over_255_rejected(
        agent in "[a-z0-9_-]{1,100}",
        over_len in 256u32..300u32
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let session_long = "s".repeat(over_len as usize);
        let res = acquire_with_path(&session_long, &agent, None, path);
        prop_assert!(res.is_err(), "Session name of {over_len} chars should be rejected");
    }
}

proptest! {
    /// Property: TTL at max boundary (86400) succeeds.
    #[test]
    fn prop_ttl_max_boundary(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // Max TTL should succeed
        let res = acquire_with_path(&session, &agent, Some(86400), path);
        prop_assert!(res.is_ok(), "TTL=86400 should succeed");
    }
}

proptest! {
    /// Property: TTL values exceeding max (86400) are always rejected.
    #[test]
    fn prop_ttl_over_max_rejected(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}",
        ttl in 86401u64..200000u64
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path(&session, &agent, Some(ttl), path);
        prop_assert!(res.is_err(), "TTL={ttl} should be rejected (exceeds 86400)");
    }
}

// ========================================================================
// 5. Additional lock invariant tests
// ========================================================================

proptest! {
    /// Property: Non-holder can never heartbeat a lock held by another agent.
    /// This is a critical safety invariant preventing unauthorized lock extension.
    #[test]
    fn prop_non_holder_heartbeat_always_fails(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent_a, Some(60), path).expect("first acquire");

        let hb_res = heartbeat_with_path(&session, &agent_b, path);
        prop_assert!(hb_res.is_err(), "Non-holder heartbeat should fail");
    }
}

proptest! {
    /// Property: Different sessions can be locked independently by the same
    /// agent. Locking one session must not affect another.
    #[test]
    fn prop_independent_session_locks_same_agent(
        session_a in "[a-z0-9_-]{1,128}",
        session_b in "[a-z0-9_-]{1,128}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(session_a != session_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res_a = acquire_with_path(&session_a, &agent, None, path);
        prop_assert!(res_a.is_ok(), "Acquire session_a should succeed: {:?}", res_a);

        let res_b = acquire_with_path(&session_b, &agent, None, path);
        prop_assert!(res_b.is_ok(), "Acquire session_b should succeed: {:?}", res_b);

        // Both heartbeats should succeed
        let hb_a = heartbeat_with_path(&session_a, &agent, path);
        prop_assert!(hb_a.is_ok(), "Heartbeat session_a should succeed: {:?}", hb_a);

        let hb_b = heartbeat_with_path(&session_b, &agent, path);
        prop_assert!(hb_b.is_ok(), "Heartbeat session_b should succeed: {:?}", hb_b);

        // Releasing one does not affect the other
        let rel_a = release_with_path(&session_a, &agent, path);
        prop_assert!(rel_a.is_ok(), "Release session_a should succeed: {:?}", rel_a);

        // session_b should still be held by agent
        let hb_b2 = heartbeat_with_path(&session_b, &agent, path);
        prop_assert!(hb_b2.is_ok(), "session_b should still be locked after releasing session_a");
    }
}

proptest! {
    /// Property: Different sessions can be locked by different agents concurrently.
    /// No cross-session lock interference.
    #[test]
    fn prop_independent_session_locks_different_agents(
        session_a in "[a-z0-9_-]{1,128}",
        session_b in "[a-z0-9_-]{1,128}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(session_a != session_b);
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res_a = acquire_with_path(&session_a, &agent_a, None, path);
        prop_assert!(res_a.is_ok(), "Agent A acquire session_a should succeed: {:?}", res_a);

        let res_b = acquire_with_path(&session_b, &agent_b, None, path);
        prop_assert!(res_b.is_ok(), "Agent B acquire session_b should succeed: {:?}", res_b);

        // Each agent can heartbeat their own session
        let hb_a = heartbeat_with_path(&session_a, &agent_a, path);
        prop_assert!(hb_a.is_ok(), "Agent A heartbeat session_a should succeed: {:?}", hb_a);

        let hb_b = heartbeat_with_path(&session_b, &agent_b, path);
        prop_assert!(hb_b.is_ok(), "Agent B heartbeat session_b should succeed: {:?}", hb_b);
    }
}

// ========================================================================
// 6. Additional stale lock removal tests
// ========================================================================

proptest! {
    /// Property: After an expired lock is taken over by agent B, agent B
    /// can heartbeat the new lock. Agent A's heartbeat on the old lock fails.
    #[test]
    fn prop_expired_lock_new_holder_can_heartbeat(
        session in "[a-z0-9_-]{1,255}",
        agent_a in "[a-z0-9_-]{1,100}",
        agent_b in "[a-z0-9_-]{1,100}"
    ) {
        prop_assume!(agent_a != agent_b);

        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path(&session, &agent_a, Some(1), path).expect("first acquire");

        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Agent B takes over
        acquire_with_path(&session, &agent_b, Some(60), path).expect("agent B acquire");

        // Agent B can heartbeat
        let hb_b = heartbeat_with_path(&session, &agent_b, path);
        prop_assert!(hb_b.is_ok(), "New holder (B) should be able to heartbeat: {:?}", hb_b);

        // Agent A's heartbeat should fail (no longer holds lock)
        let hb_a = heartbeat_with_path(&session, &agent_a, path);
        prop_assert!(hb_a.is_err(), "Expired holder (A) should not be able to heartbeat");
    }
}

// ========================================================================
// 7. Additional state machine transition tests
// ========================================================================

proptest! {
    /// Property: Releasing a lock that was never acquired is idempotent (no error).
    /// This prevents errors from race conditions or double-init scenarios.
    #[test]
    fn prop_release_nonexistent_lock_is_idempotent(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = release_with_path(&session, &agent, path);
        prop_assert!(res.is_ok(), "Releasing a nonexistent lock should be idempotent: {:?}", res);
    }
}

proptest! {
    /// Property: Status query always succeeds regardless of lock state.
    /// Both for locked and unlocked sessions, status should return Ok.
    #[test]
    fn prop_status_always_succeeds(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // Status on unlocked session
        let res_unlocked = status_with_path(&session, path);
        prop_assert!(res_unlocked.is_ok(), "Status on unlocked session should succeed: {:?}", res_unlocked);

        // Acquire lock
        acquire_with_path(&session, &agent, None, path).expect("acquire");

        // Status on locked session
        let res_locked = status_with_path(&session, path);
        prop_assert!(res_locked.is_ok(), "Status on locked session should succeed: {:?}", res_locked);
    }
}

// ========================================================================
// 8. Additional edge case tests
// ========================================================================

proptest! {
    /// Property: TTL=0 is accepted (uses default TTL internally).
    /// The lock should be acquired successfully with default TTL.
    #[test]
    fn prop_ttl_zero_uses_default(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path(&session, &agent, Some(0), path);
        prop_assert!(res.is_ok(), "TTL=0 should succeed (uses default): {:?}", res);

        // Lock should be active after acquire (heartbeat works)
        let hb = heartbeat_with_path(&session, &agent, path);
        prop_assert!(hb.is_ok(), "Heartbeat should succeed on TTL=0 lock: {:?}", hb);
    }
}

proptest! {
    /// Property: Session names containing control characters are always rejected.
    /// This fuzzes the boundary between valid and invalid characters.
    #[test]
    fn prop_control_chars_in_session_rejected(
        agent in "[a-z0-9_-]{1,100}",
        byte_val in 0u8..31u8
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // Build a session name with the control byte injected
        let control_char = byte_val as char;
        let session = format!("session{control_char}name");

        let res = acquire_with_path(&session, &agent, None, path);
        prop_assert!(res.is_err(), "Session with control char (0x{:02x}) should be rejected", byte_val);
    }
}

proptest! {
    /// Property: list_with_path always succeeds and returns Ok, even when
    /// no locks exist or multiple locks are active.
    #[test]
    fn prop_list_always_succeeds(
        session in "[a-z0-9_-]{1,128}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        // List with no locks
        let res_empty = list_with_path(path);
        prop_assert!(res_empty.is_ok(), "List with no locks should succeed: {:?}", res_empty);

        // Acquire a lock
        acquire_with_path(&session, &agent, None, path).expect("acquire");

        // List with one lock
        let res_one = list_with_path(path);
        prop_assert!(res_one.is_ok(), "List with one lock should succeed: {:?}", res_one);
    }
}

proptest! {
    /// Property: u64::MAX TTL is always rejected (overflow protection).
    #[test]
    fn prop_ttl_u64_max_rejected(
        session in "[a-z0-9_-]{1,255}",
        agent in "[a-z0-9_-]{1,100}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path(&session, &agent, Some(u64::MAX), path);
        prop_assert!(res.is_err(), "TTL=u64::MAX should be rejected (overflow protection)");
    }
}

proptest! {
    /// Property: Agent IDs with whitespace characters are accepted (validation
    /// only rejects empty strings, not whitespace content).
    #[test]
    fn prop_agent_with_whitespace_accepted(
        session in "[a-z0-9_-]{1,128}",
        base_agent in "[a-z0-9_-]{1,50}",
        whitespace in " {1,10}"
    ) {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let agent = format!("{base_agent}{whitespace}");
        let res = acquire_with_path(&session, &agent, None, path);
        prop_assert!(res.is_ok(), "Agent ID with whitespace should be accepted: {:?}", res);
    }
}

// ========================================================================
// 9. Symlink security tests for init lock
// ========================================================================

#[test]
fn test_acquire_init_lock_refuses_symlink() {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let cwd = tmp_dir.path();

    // Create a target file that we don't want to be corrupted
    let target_file = cwd.join("important-file.txt");
    std::fs::write(&target_file, "critical data").expect("Failed to write target file");

    // Create a symlink at the lock path pointing to the target file
    let lock_path = cwd.join(".scp-init.lock");
    std::os::unix::fs::symlink(&target_file, &lock_path).expect("Failed to create symlink");

    // Verify the symlink exists and points to the right place
    assert!(lock_path.is_symlink());

    // Attempt to acquire the lock should fail with a symlink error
    let result = crate::commands::init::acquire_init_lock(cwd);
    assert!(
        result.is_err(),
        "acquire_init_lock should refuse to follow a symlink"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("symlink"),
        "Error message should mention 'symlink': {err_msg}"
    );
    assert!(
        err_msg.contains("refusing to follow"),
        "Error message should explain security reason: {err_msg}"
    );

    // The target file must remain intact
    let contents = std::fs::read_to_string(&target_file).expect("Failed to read target file");
    assert_eq!(
        contents, "critical data",
        "Target file must not be corrupted"
    );
}

#[test]
fn test_acquire_init_lock_succeeds_when_no_symlink() {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let cwd = tmp_dir.path();

    // No symlink, no existing file — lock should succeed
    let result = crate::commands::init::acquire_init_lock(cwd);
    assert!(
        result.is_ok(),
        "acquire_init_lock should succeed when no symlink exists"
    );

    // Clean up: release the lock
    if let Ok(file) = &result {
        let _ = file.unlock();
    }
}
