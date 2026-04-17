//! BDD Validation: scp-core — prove it works before ship
//!
//! Comprehensive happy-path and adversarial validation of all public types.
//! Run with: cargo test -p scp-core --test bdd_validation -- --nocapture

use scp_core::*;
use scp_core::lock::LockManager;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 1: SessionName validation
// Claims: 1-63 chars, starts with letter, alphanumeric/dash/underscore only
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_session_name_valid_inputs() {
    // Simple valid
    assert!(SessionName::parse("valid-name").is_ok());
    // Single letter
    assert!(SessionName::parse("a").is_ok());
    // With underscore
    assert!(SessionName::parse("my_session").is_ok());
    // With numbers
    assert!(SessionName::parse("session123").is_ok());
    // Mixed
    assert!(SessionName::parse("my-session_v2").is_ok());
    // Max length (63)
    assert!(SessionName::parse(&"a".repeat(63)).is_ok());
    // Whitespace trimmed
    let trimmed = SessionName::parse("  test-session  ").unwrap();
    assert_eq!(trimmed.as_str(), "test-session");
    println!("[PASS] SessionName: all valid inputs accepted");
}

#[test]
fn claim_session_name_invalid_inputs() {
    // Empty
    assert!(SessionName::parse("").is_err());
    // Whitespace only
    assert!(SessionName::parse("   ").is_err());
    // Starts with number
    assert!(SessionName::parse("123session").is_err());
    // Starts with hyphen
    assert!(SessionName::parse("-session").is_err());
    // Starts with underscore
    assert!(SessionName::parse("_session").is_err());
    // Too long (64)
    assert!(SessionName::parse(&"a".repeat(64)).is_err());
    // Special chars
    assert!(SessionName::parse("session.name").is_err());
    assert!(SessionName::parse("session@name").is_err());
    assert!(SessionName::parse("session name").is_err());
    // Unicode
    assert!(SessionName::parse("caf\u{00e9}-session").is_err());
    println!("[PASS] SessionName: all invalid inputs rejected");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 2: SessionId validation
// Claims: non-empty, alphanumeric + hyphens only
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_session_id_valid_inputs() {
    assert!(SessionId::parse("abc123").is_ok());
    assert!(SessionId::parse("session-123").is_ok());
    assert!(SessionId::parse("---").is_ok());
    assert!(SessionId::parse("a").is_ok());
    assert!(SessionId::parse("1").is_ok());
    // Very long IDs accepted
    assert!(SessionId::parse(&"a".repeat(10000)).is_ok());
    println!("[PASS] SessionId: all valid inputs accepted");
}

#[test]
fn claim_session_id_invalid_inputs() {
    assert!(SessionId::parse("").is_err());
    assert!(SessionId::parse(" ").is_err());
    assert!(SessionId::parse("session_123").is_err());
    assert!(SessionId::parse("session.123").is_err());
    assert!(SessionId::parse("session/123").is_err());
    assert!(SessionId::parse("session@host").is_err());
    assert!(SessionId::parse("session!@#$%").is_err());
    assert!(SessionId::parse("session 123").is_err());
    println!("[PASS] SessionId: all invalid inputs rejected");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 3: AbsolutePath validation
// Claims: must start with /, rejects relative paths
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_absolute_path_valid_inputs() {
    assert!(AbsolutePath::parse("/").is_ok());
    assert!(AbsolutePath::parse("/home/user").is_ok());
    assert!(AbsolutePath::parse("/tmp").is_ok());
    assert!(AbsolutePath::parse("/a/b/c/d/e/f").is_ok());
    assert!(AbsolutePath::parse("/home/user/").is_ok());
    // Path traversal WITH leading slash is technically absolute
    assert!(AbsolutePath::parse("/home/user/./docs/../workspace").is_ok());
    println!("[PASS] AbsolutePath: all valid inputs accepted");
}

#[test]
fn claim_absolute_path_invalid_inputs() {
    assert!(AbsolutePath::parse("relative/path").is_err());
    assert!(AbsolutePath::parse("").is_err());
    assert!(AbsolutePath::parse(".").is_err());
    assert!(AbsolutePath::parse("..").is_err());
    assert!(AbsolutePath::parse("~/workspace").is_err());
    println!("[PASS] AbsolutePath: all invalid inputs rejected");
}

#[test]
fn claim_absolute_path_path_traversal_attack() {
    // The critical adversarial test: path traversal via ../../etc/passwd
    assert!(AbsolutePath::parse("../../etc/passwd").is_err());
    assert!(AbsolutePath::parse("../../../etc/shadow").is_err());
    // Even with absolute prefix but traversal in segments — accepted as absolute
    // (the type only checks leading /, not canonicalization)
    let dangerous = AbsolutePath::parse("/tmp/../../etc/passwd");
    assert!(dangerous.is_ok(), "AbsolutePath only validates leading /, not canonicalization — this is expected");
    println!("[PASS] AbsolutePath: path traversal handled (relative rejected, absolute-with-traversal accepted as documented)");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 4: SessionStatus state machine
// Claims: Creating->Active|Failed, Active->Paused|Completed, Paused->Active|Completed
// Terminal: Completed, Failed
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_session_status_transitions() {
    // Valid transitions
    assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
    assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
    assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
    assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
    assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
    assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));

    // Invalid transitions
    assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
    assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Completed));
    assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));
    assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Active));
    assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Creating));
    assert!(!SessionStatus::Paused.can_transition_to(SessionStatus::Paused));

    // Terminal states reject everything
    for &next in SessionStatus::all_states() {
        assert!(!SessionStatus::Completed.can_transition_to(next));
        assert!(!SessionStatus::Failed.can_transition_to(next));
    }
    println!("[PASS] SessionStatus: all transitions correct");
}

#[test]
fn claim_session_status_operations() {
    // Creating: no operations
    assert!(SessionStatus::Creating.allowed_operations().is_empty());
    // Active: all operations
    assert!(SessionStatus::Active.allows_operation(Operation::Status));
    assert!(SessionStatus::Active.allows_operation(Operation::Diff));
    assert!(SessionStatus::Active.allows_operation(Operation::Focus));
    assert!(SessionStatus::Active.allows_operation(Operation::Remove));
    // Paused: no Diff
    assert!(SessionStatus::Paused.allows_operation(Operation::Status));
    assert!(!SessionStatus::Paused.allows_operation(Operation::Diff));
    // Terminal: only Remove
    assert!(SessionStatus::Completed.allows_operation(Operation::Remove));
    assert!(!SessionStatus::Completed.allows_operation(Operation::Status));
    println!("[PASS] SessionStatus: operations gated correctly");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 5: WorkspaceState state machine
// Claims: Created->Working, Working->Ready|Conflict|Abandoned, Ready->Working|Merged|Conflict|Abandoned
// Terminal: Merged, Abandoned
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_workspace_state_transitions() {
    // Valid transitions
    assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));
    assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
    assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
    assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Abandoned));
    assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));
    assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
    assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Conflict));
    assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Abandoned));
    assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
    assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Abandoned));

    // Terminal states
    assert!(WorkspaceState::Merged.is_terminal());
    assert!(WorkspaceState::Abandoned.is_terminal());
    assert!(!WorkspaceState::Created.is_terminal());
    assert!(!WorkspaceState::Working.is_terminal());
    assert!(!WorkspaceState::Ready.is_terminal());
    assert!(!WorkspaceState::Conflict.is_terminal());

    // Active states
    assert!(WorkspaceState::Working.is_active());
    assert!(WorkspaceState::Conflict.is_active());
    assert!(!WorkspaceState::Created.is_active());
    assert!(!WorkspaceState::Ready.is_active());

    // Complete states
    assert!(WorkspaceState::Ready.is_complete());
    assert!(WorkspaceState::Merged.is_complete());
    assert!(!WorkspaceState::Working.is_complete());

    // Terminal rejects all
    for &next in WorkspaceState::all() {
        assert!(!WorkspaceState::Merged.can_transition_to(next));
        assert!(!WorkspaceState::Abandoned.can_transition_to(next));
    }
    println!("[PASS] WorkspaceState: all transitions correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 6: ValidatedMetadata
// Claims: key-value store, insert/get, overwrite, empty, serde roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_metadata_operations() {
    let mut meta = ValidatedMetadata::empty();
    assert_eq!(meta.get("anything"), None);

    meta.insert("key", "value");
    assert_eq!(meta.get("key"), Some("value"));

    // Overwrite
    meta.insert("key", "new-value");
    assert_eq!(meta.get("key"), Some("new-value"));

    // Multiple keys
    meta.insert("a", "1");
    meta.insert("b", "2");
    assert_eq!(meta.get("a"), Some("1"));
    assert_eq!(meta.get("b"), Some("2"));
    assert_eq!(meta.get("missing"), None);

    // Empty value
    meta.insert("empty", "");
    assert_eq!(meta.get("empty"), Some(""));

    // Default is empty
    let default_meta = ValidatedMetadata::default();
    assert_eq!(default_meta.get("x"), None);

    println!("[PASS] ValidatedMetadata: all operations correct");
}

#[test]
fn claim_metadata_serde_roundtrip() {
    let mut meta = ValidatedMetadata::empty();
    meta.insert("author", "test-agent");
    meta.insert("version", "1.0.0");

    let json = serde_json::to_string(&meta).unwrap();
    let deserialized: ValidatedMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.get("author"), Some("test-agent"));
    assert_eq!(deserialized.get("version"), Some("1.0.0"));
    println!("[PASS] ValidatedMetadata: serde roundtrip correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 7: Priority ordering
// Claims: Critical < High < Normal < Low, default is Normal
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_priority_ordering() {
    assert!(Priority::Critical < Priority::High);
    assert!(Priority::High < Priority::Normal);
    assert!(Priority::Normal < Priority::Low);
    assert_eq!(Priority::default(), Priority::Normal);

    let mut sorted = vec![Priority::Low, Priority::Critical, Priority::High, Priority::Normal];
    sorted.sort();
    assert_eq!(sorted, vec![Priority::Critical, Priority::High, Priority::Normal, Priority::Low]);
    println!("[PASS] Priority: ordering correct, default is Normal");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 8: Queue operations
// Claims: enqueue, dequeue (priority-ordered), get, remove, update, list, clear_completed
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_queue_full_lifecycle() -> Result<()> {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let queue = MemQueue::new(lock);

    // Empty queue
    assert!(queue.is_empty()?);
    assert_eq!(queue.len()?, 0);
    assert!(queue.dequeue()?.is_none());

    // Enqueue
    queue.enqueue(QueueItem::direct("branch-1"))?;
    queue.enqueue(QueueItem::direct("branch-2"))?;
    assert_eq!(queue.len()?, 2);
    assert!(!queue.is_empty()?);

    // Dequeue returns first
    let item = queue.dequeue()?.unwrap();
    assert_eq!(item.branch, "branch-1");
    assert_eq!(item.status, QueueStatus::Processing);
    assert_eq!(item.attempt_count, 1);

    // Get by ID
    let item2 = QueueItem::direct("find-me");
    let id = item2.id.clone();
    queue.enqueue(item2)?;
    let found = queue.get(&id)?.unwrap();
    assert_eq!(found.branch, "find-me");

    // Remove by ID
    let removed = queue.remove(&id)?;
    assert_eq!(removed.branch, "find-me");

    // Update
    let item3 = QueueItem::direct("update-me");
    let id3 = item3.id.clone();
    queue.enqueue(item3)?;
    let mut fetched = queue.get(&id3)?.unwrap();
    fetched.status = QueueStatus::Completed;
    queue.update(fetched)?;
    assert_eq!(queue.get(&id3)?.unwrap().status, QueueStatus::Completed);

    // List pending
    let pending = queue.list_pending()?;
    assert!(pending.iter().all(|i| i.status == QueueStatus::Pending));

    // Clear completed
    let mut completed = QueueItem::direct("done");
    completed.status = QueueStatus::Completed;
    queue.enqueue(completed)?;
    let removed_count = queue.clear_completed()?;
    assert!(removed_count > 0);

    println!("[PASS] Queue: full lifecycle correct");
    Ok(())
}

#[test]
fn claim_queue_priority_ordering() -> Result<()> {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let queue = MemQueue::new(lock);

    // Enqueue in reverse priority order
    let mut low = QueueItem::direct("low");
    low.priority = Priority::Low;
    let mut normal = QueueItem::direct("normal");
    normal.priority = Priority::Normal;
    let mut high = QueueItem::direct("high");
    high.priority = Priority::High;
    let mut critical = QueueItem::direct("critical");
    critical.priority = Priority::Critical;

    queue.enqueue(low)?;
    queue.enqueue(normal)?;
    queue.enqueue(high)?;
    queue.enqueue(critical)?;

    // Dequeue should yield Critical, High, Normal, Low
    assert_eq!(queue.dequeue()?.unwrap().branch, "critical");
    assert_eq!(queue.dequeue()?.unwrap().branch, "high");
    assert_eq!(queue.dequeue()?.unwrap().branch, "normal");
    assert_eq!(queue.dequeue()?.unwrap().branch, "low");

    println!("[PASS] Queue: priority ordering correct");
    Ok(())
}

#[test]
fn claim_queue_item_state_transitions() {
    let mut item = QueueItem::direct("test");
    assert_eq!(item.status, QueueStatus::Pending);
    assert_eq!(item.attempt_count, 0);
    assert!(item.last_error.is_none());

    item.start_processing();
    assert_eq!(item.status, QueueStatus::Processing);
    assert_eq!(item.attempt_count, 1);

    item.start_processing(); // double-call
    assert_eq!(item.attempt_count, 2);

    item.complete();
    assert_eq!(item.status, QueueStatus::Completed);
    assert!(item.last_error.is_none());

    // Fail path
    let mut item2 = QueueItem::direct("fail-test");
    item2.fail("something broke");
    assert_eq!(item2.status, QueueStatus::Failed);
    assert_eq!(item2.last_error, Some("something broke".to_string()));

    // Cancel path
    let mut item3 = QueueItem::direct("cancel-test");
    item3.cancel();
    assert_eq!(item3.status, QueueStatus::Cancelled);

    println!("[PASS] QueueItem: state transitions correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 9: Agent management
// Claims: register, heartbeat, activity tracking, status
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_agent_lifecycle() -> Result<()> {
    let registry = get_agent_registry();
    let id = AgentId::new("test-agent-bdd");

    // Register with Agent struct
    let agent = Agent::new(id.clone());
    registry.register(agent)?;

    // Get
    let found = registry.get(&id)?.expect("agent should exist");
    assert_eq!(found.id.as_str(), "test-agent-bdd");
    assert!(found.is_active());
    assert_eq!(found.status(), AgentStatus::Active);

    // Heartbeat via trait method
    registry.heartbeat(&id)?;
    assert!(registry.get(&id)?.unwrap().is_active());

    // Activity tracking: get, mutate locally, verify (no update method on trait)
    let found = registry.get(&id)?.unwrap();
    assert!(!found.activity.is_working()); // still idle
    assert_eq!(found.actions_count, 0);

    // List agents
    let all = registry.list()?;
    assert!(all.iter().any(|a| a.id.as_str() == "test-agent-bdd"));

    // List active
    let active = registry.list_active()?;
    assert!(active.iter().any(|a| a.id.as_str() == "test-agent-bdd"));

    // Unregister
    let _removed = registry.unregister(&id)?;
    assert!(registry.get(&id)?.is_none());

    println!("[PASS] Agent: lifecycle correct");
    Ok(())
}

#[test]
fn claim_agent_id_validation() {
    assert!(AgentId::new_checked("valid-id").is_ok());
    assert!(AgentId::new_checked("").is_err());
    // new() bypasses validation
    let _ = AgentId::new("anything");
    println!("[PASS] AgentId: validation correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 10: Session aggregate
// Claims: validate timestamps, serde roundtrip, metadata
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_session_validation() {
    let now = chrono::Utc::now();
    let earlier = now - chrono::Duration::seconds(60);

    let valid_session = Session {
        id: SessionId::parse("test-id").unwrap(),
        name: SessionName::parse("test-session").unwrap(),
        status: SessionStatus::Active,
        state: WorkspaceState::Working,
        workspace_path: AbsolutePath::parse("/tmp/test").unwrap(),
        branch: BranchState::OnBranch("main".to_string()),
        created_at: now,
        updated_at: now,
        last_synced: None,
        metadata: ValidatedMetadata::default(),
    };
    assert!(valid_session.validate().is_ok());

    let invalid_session = Session {
        updated_at: earlier,
        ..valid_session.clone()
    };
    assert!(invalid_session.validate().is_err());
    println!("[PASS] Session: validation correct");
}

#[test]
fn claim_session_serde_roundtrip() {
    let mut session = Session {
        id: SessionId::parse("serde-id").unwrap(),
        name: SessionName::parse("serde-session").unwrap(),
        status: SessionStatus::Active,
        state: WorkspaceState::Working,
        workspace_path: AbsolutePath::parse("/tmp/serde-test").unwrap(),
        branch: BranchState::OnBranch("feature".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: Some(chrono::Utc::now()),
        metadata: ValidatedMetadata::default(),
    };
    session.metadata.insert("author", "bdd-test");

    let json = serde_json::to_string(&session).unwrap();
    let deserialized: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(session.id.as_str(), deserialized.id.as_str());
    assert_eq!(session.name.as_str(), deserialized.name.as_str());
    assert_eq!(deserialized.metadata.get("author"), Some("bdd-test"));

    // Deserializing invalid path fails
    let bad_json = r#"{"id":"x","name":"y","status":"active","state":"working","workspace_path":"relative","branch":{"OnBranch":"main"},"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
    assert!(serde_json::from_str::<Session>(bad_json).is_err());

    println!("[PASS] Session: serde roundtrip correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 11: Error system
// Claims: unified error type, code(), exit_code(), suggestion(), context_map()
// Zero panic: no unwrap/expect in library code
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_error_system() {
    // Error creation
    let err = Error::invalid_state("test error");
    let _msg = format!("{err}");

    // Backwards-compatible constructors
    let _ = Error::database("db error");
    let _ = Error::invalid_identifier("bad id");
    let _ = Error::io_error("io error");
    let _ = Error::not_found("not found");
    let _ = Error::validation_error("validation failed");
    let _ = Error::session_locked("sess1", "agent1");

    // From<std::io::Error>
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let scp_err: Error = io_err.into();
    let _msg = format!("{scp_err}");

    // Result type works
    let result: Result<i32> = Ok(42);
    assert_eq!(result.unwrap(), 42);
    let result: Result<i32> = Err(Error::invalid_state("fail"));
    assert!(result.is_err());

    println!("[PASS] Error: system works correctly");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 12: Lock manager
// Claims: acquire, release, try_acquire, is_locked, list_locks, RAII drop
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_lock_manager() -> Result<()> {
    let mgr = MemLockManager::new();
    let lock_type = LockType::Workspace("test-ws".to_string());

    // Initially unlocked
    assert!(!mgr.is_locked(&lock_type)?);

    // Acquire
    let _guard = mgr.acquire(lock_type.clone(), "agent-1")?;
    assert!(mgr.is_locked(&lock_type)?);

    // Double-acquire fails
    let result = mgr.acquire(lock_type.clone(), "agent-2");
    assert!(result.is_err());

    // Release via drop
    drop(_guard);
    assert!(!mgr.is_locked(&lock_type)?);

    // Try-acquire
    let guard = mgr.try_acquire(lock_type.clone(), "agent-3")?;
    assert!(guard.is_some());
    drop(guard);

    // List locks
    let mgr2 = MemLockManager::new();
    let _g1 = mgr2.acquire(LockType::Session("s1".to_string()), "a1")?;
    let _g2 = mgr2.acquire(LockType::Queue("q1".to_string()), "a2")?;
    let locks = mgr2.list_locks()?;
    assert_eq!(locks.len(), 2);

    println!("[PASS] LockManager: all operations correct");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 13: File change tracking
// Claims: FileStatus enum, FileChange validation (Renamed needs old_path)
// ChangesSummary::total(), has_changes(), has_tracked_changes()
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_file_change_tracking() {
    // FileChange validation: Renamed without old_path fails
    let renamed_bad = FileChange {
        path: std::path::PathBuf::from("new/path.txt"),
        status: FileStatus::Renamed,
        old_path: None,
    };
    assert!(renamed_bad.validate().is_err());

    let renamed_good = FileChange {
        path: std::path::PathBuf::from("new/path.txt"),
        status: FileStatus::Renamed,
        old_path: Some(std::path::PathBuf::from("old/path.txt")),
    };
    assert!(renamed_good.validate().is_ok());

    // ChangesSummary: total() excludes untracked (only tracked: modified+added+deleted+renamed)
    let summary = ChangesSummary {
        modified: 5, added: 3, deleted: 2, renamed: 1, untracked: 4,
    };
    assert_eq!(summary.total(), 11); // 5+3+2+1 (untracked excluded)
    assert!(summary.has_changes());
    assert!(summary.has_tracked_changes());

    // Only untracked changes = no tracked changes
    let untracked_only = ChangesSummary {
        modified: 0, added: 0, deleted: 0, renamed: 0, untracked: 5,
    };
    assert_eq!(untracked_only.total(), 0);
    assert!(!untracked_only.has_changes());
    assert!(!untracked_only.has_tracked_changes());

    let empty = ChangesSummary::default();
    assert_eq!(empty.total(), 0);
    assert!(!empty.has_changes());
    assert!(!empty.has_tracked_changes());

    println!("[PASS] FileChange: tracking correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 14: BeadsIssue and BeadsSummary
// Claims: issue types, summary counting
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_beads_types() {
    let summary = BeadsSummary {
        open: 3, in_progress: 2, blocked: 1, closed: 5,
    };
    assert_eq!(summary.total(), 11);
    assert_eq!(summary.active(), 5); // open + in_progress (3 + 2)
    assert!(summary.has_blockers());

    let no_blockers = BeadsSummary {
        open: 1, in_progress: 0, blocked: 0, closed: 3,
    };
    assert!(!no_blockers.has_blockers());

    println!("[PASS] BeadsIssue: types correct");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 15: Serde roundtrip for all core types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_serde_roundtrip_all_types() {
    // SessionName
    let sn = SessionName::parse("serde-test").unwrap();
    let json = serde_json::to_string(&sn).unwrap();
    assert_eq!(sn, serde_json::from_str::<SessionName>(&json).unwrap());

    // SessionId
    let si = SessionId::parse("serde-id-123").unwrap();
    let json = serde_json::to_string(&si).unwrap();
    assert_eq!(si, serde_json::from_str::<SessionId>(&json).unwrap());

    // AbsolutePath
    let ap = AbsolutePath::parse("/tmp/serde").unwrap();
    let json = serde_json::to_string(&ap).unwrap();
    assert_eq!(ap, serde_json::from_str::<AbsolutePath>(&json).unwrap());

    // Priority
    for p in [Priority::Critical, Priority::High, Priority::Normal, Priority::Low] {
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(p, serde_json::from_str::<Priority>(&json).unwrap());
    }

    // QueueStatus
    for s in [QueueStatus::Pending, QueueStatus::Processing, QueueStatus::Completed, QueueStatus::Failed, QueueStatus::Cancelled] {
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(s, serde_json::from_str::<QueueStatus>(&json).unwrap());
    }

    // WorkspaceState
    for ws in WorkspaceState::all() {
        let json = serde_json::to_string(ws).unwrap();
        assert_eq!(*ws, serde_json::from_str::<WorkspaceState>(&json).unwrap());
    }

    // SessionStatus
    for ss in SessionStatus::all_states() {
        let json = serde_json::to_string(ss).unwrap();
        assert_eq!(*ss, serde_json::from_str::<SessionStatus>(&json).unwrap());
    }

    // QueueItem
    let qi = QueueItem::new("main", QueueSource::Direct);
    let json = serde_json::to_string(&qi).unwrap();
    let d: QueueItem = serde_json::from_str(&json).unwrap();
    assert_eq!(qi.branch, d.branch);

    println!("[PASS] Serde: all types roundtrip correctly");
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL: Stress tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn adversarial_queue_concurrent_enqueue_dequeue() -> Result<()> {
    use std::sync::Arc;
    use std::thread;

    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let queue = Arc::new(MemQueue::new(lock));
    let mut handles = vec![];

    // Spawn 10 threads enqueuing
    for i in 0..10 {
        let q = queue.clone();
        handles.push(thread::spawn(move || {
            for j in 0..50 {
                let item = QueueItem::direct(format!("thread-{}-item-{}", i, j));
                q.enqueue(item).unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(queue.len()?, 500);

    // Dequeue all
    let mut count = 0;
    while queue.dequeue()?.is_some() {
        count += 1;
    }
    assert_eq!(count, 500);

    println!("[PASS] Adversarial: concurrent enqueue/dequeue (500 items across 10 threads)");
    Ok(())
}

#[test]
fn adversarial_lock_concurrent_access() -> Result<()> {
    use std::sync::Arc;
    use std::thread;

    let mgr = Arc::new(MemLockManager::new());
    let mut handles = vec![];

    for i in 0..10 {
        let m = mgr.clone();
        handles.push(thread::spawn(move || {
            let lock_type = LockType::Agent(format!("agent-{}", i));
            // Acquire and return the guard (moved out of thread)
            m.try_acquire(lock_type, "holder").unwrap()
        }));
    }

    // Collect all guards to keep them alive
    let mut guards = vec![];
    for h in handles {
        let guard = h.join().unwrap();
        assert!(guard.is_some());
        guards.push(guard);
    }

    let locks = mgr.list_locks()?;
    assert_eq!(locks.len(), 10);

    // Drop guards and verify locks released
    guards.clear();
    let locks_after = mgr.list_locks()?;
    assert_eq!(locks_after.len(), 0);

    println!("[PASS] Adversarial: concurrent lock acquisition (10 agents)");
    Ok(())
}

#[test]
fn adversarial_large_queue_items() -> Result<()> {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let queue = MemQueue::new(lock);

    // Large branch names
    for i in 0..100 {
        let branch = format!("feature/very-long-branch-name-{}-{}", i, "x".repeat(200));
        queue.enqueue(QueueItem::direct(branch))?;
    }
    assert_eq!(queue.len()?, 100);

    // Large error messages
    let mut item = QueueItem::direct("big-error");
    item.fail("E".repeat(10000));
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.len() > 10000);
    let deserialized: QueueItem = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.last_error.unwrap().len(), 10000);

    println!("[PASS] Adversarial: large queue items (100 items with 200-char names, 10KB error messages)");
    Ok(())
}

#[test]
fn adversarial_boundary_session_name() {
    // Exactly at boundary
    assert!(SessionName::parse(&"a".repeat(63)).is_ok());
    assert!(SessionName::parse(&"a".repeat(64)).is_err());

    // Empty after trim
    assert!(SessionName::parse("   ").is_err());

    // Single valid char
    assert!(SessionName::parse("Z").is_ok());

    println!("[PASS] Adversarial: boundary values for SessionName");
}

#[test]
fn adversarial_boundary_session_id() {
    // Very long ID
    assert!(SessionId::parse(&"a".repeat(100000)).is_ok());

    // Single char
    assert!(SessionId::parse("a").is_ok());

    println!("[PASS] Adversarial: boundary values for SessionId");
}

#[test]
fn adversarial_queue_error_cases() {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let queue = MemQueue::new(lock);

    // Remove non-existent
    assert!(queue.remove("nonexistent").is_err());

    // Update non-existent
    let mut ghost = QueueItem::direct("ghost");
    ghost.id = "nonexistent-id".to_string();
    assert!(queue.update(ghost).is_err());

    // Get non-existent
    assert!(queue.get("nope").unwrap().is_none());

    println!("[PASS] Adversarial: queue error cases handled gracefully");
}

#[test]
fn adversarial_workspace_state_from_str() {
    use std::str::FromStr;
    // Valid
    assert!(WorkspaceState::from_str("created").is_ok());
    assert!(WorkspaceState::from_str("Working").is_ok()); // case insensitive
    assert!(WorkspaceState::from_str("MERGED").is_ok());
    // Invalid
    assert!(WorkspaceState::from_str("bogus").is_err());
    assert!(WorkspaceState::from_str("").is_err());

    println!("[PASS] Adversarial: WorkspaceState from_str edge cases");
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM 16: VERSION constant
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn claim_version_constant() {
    assert!(!VERSION.is_empty());
    assert!(VERSION.starts_with('0') || VERSION.starts_with('1'));
    println!("[PASS] VERSION: non-empty, starts with digit");
}
