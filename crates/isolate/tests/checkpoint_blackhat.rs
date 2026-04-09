//! Black-hat tests for checkpoint types and command classification.
//!
//! Covers:
//! - OperationRisk: Safe vs Risky, default, needs_checkpoint
//! - CheckpointState: all variants, from_db, as_db roundtrip, invalid input
//! - CheckpointRecord: construction, equality
//! - classify_command: all known risky commands, all safe commands, unknown, case sensitivity
//! - Proptests

use scp_isolate::classify_command;
use scp_isolate::{CheckpointRecord, CheckpointState, OperationRisk};

// === OperationRisk ===

#[test]
fn safe_is_default() {
    assert_eq!(OperationRisk::default(), OperationRisk::Safe);
}

#[test]
fn safe_needs_no_checkpoint() {
    assert!(!OperationRisk::Safe.needs_checkpoint());
}

#[test]
fn risky_needs_checkpoint() {
    assert!(OperationRisk::Risky.needs_checkpoint());
}

#[test]
fn risk_clone_eq() {
    assert_eq!(OperationRisk::Safe.clone(), OperationRisk::Safe);
    assert_eq!(OperationRisk::Risky.clone(), OperationRisk::Risky);
}

#[test]
fn risk_copy() {
    let safe = OperationRisk::Safe;
    let copied = safe;
    assert_eq!(safe, copied);
}

#[test]
fn risk_debug_format() {
    assert!(!format!("{:?}", OperationRisk::Safe).is_empty());
    assert!(!format!("{:?}", OperationRisk::Risky).is_empty());
}

#[test]
fn risk_equality() {
    assert_eq!(OperationRisk::Safe, OperationRisk::Safe);
    assert_eq!(OperationRisk::Risky, OperationRisk::Risky);
    assert_ne!(OperationRisk::Safe, OperationRisk::Risky);
}

#[test]
fn risk_hash_consistency() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(OperationRisk::Safe);
    assert!(set.contains(&OperationRisk::Safe));
    assert!(!set.contains(&OperationRisk::Risky));
}

#[test]
fn risk_serde_roundtrip() {
    for risk in [OperationRisk::Safe, OperationRisk::Risky] {
        let json = serde_json::to_string(&risk).unwrap();
        let parsed: OperationRisk = serde_json::from_str(&json).unwrap();
        assert_eq!(risk, parsed);
    }
}

// === CheckpointState ===

#[test]
fn default_is_pending() {
    assert_eq!(CheckpointState::default(), CheckpointState::Pending);
}

#[test]
fn from_db_valid_inputs() {
    assert_eq!(
        CheckpointState::from_db("pending").unwrap(),
        CheckpointState::Pending
    );
    assert_eq!(
        CheckpointState::from_db("committed").unwrap(),
        CheckpointState::Committed
    );
    assert_eq!(
        CheckpointState::from_db("needs_restore").unwrap(),
        CheckpointState::NeedsRestore
    );
}

#[test]
fn from_db_invalid_inputs() {
    let invalid = [
        "", "PENDING", "invalid", "created", "ready", "active", "done",
    ];
    for input in &invalid {
        assert!(
            CheckpointState::from_db(input).is_err(),
            "'{input}' should be invalid"
        );
    }
}

#[test]
fn as_db_roundtrip() {
    for state in [
        CheckpointState::Pending,
        CheckpointState::Committed,
        CheckpointState::NeedsRestore,
    ] {
        let db_str = state.as_db();
        let parsed = CheckpointState::from_db(db_str).unwrap();
        assert_eq!(state, parsed, "roundtrip failed for {state:?}");
    }
}

#[test]
fn as_db_returns_expected_strings() {
    assert_eq!(CheckpointState::Pending.as_db(), "pending");
    assert_eq!(CheckpointState::Committed.as_db(), "committed");
    assert_eq!(CheckpointState::NeedsRestore.as_db(), "needs_restore");
}

#[test]
fn checkpoint_state_equality() {
    assert_eq!(CheckpointState::Pending, CheckpointState::Pending);
    assert_ne!(CheckpointState::Pending, CheckpointState::Committed);
}

#[test]
fn checkpoint_state_clone() {
    assert_eq!(CheckpointState::Pending.clone(), CheckpointState::Pending);
}

#[test]
fn checkpoint_state_serde_roundtrip() {
    for state in [
        CheckpointState::Pending,
        CheckpointState::Committed,
        CheckpointState::NeedsRestore,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let parsed: CheckpointState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed);
    }
}

#[test]
fn checkpoint_state_debug() {
    for state in [
        CheckpointState::Pending,
        CheckpointState::Committed,
        CheckpointState::NeedsRestore,
    ] {
        let debug = format!("{state:?}");
        assert!(!debug.is_empty());
    }
}

#[test]
fn checkpoint_state_hash_consistency() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(CheckpointState::Pending);
    assert!(set.contains(&CheckpointState::Pending));
    assert!(!set.contains(&CheckpointState::Committed));
}

// === CheckpointRecord ===

#[test]
fn record_new_sets_fields() {
    let record = CheckpointRecord::new("auto-123".to_string(), false);
    assert_eq!(record.id, "auto-123");
    assert!(!record.committed);
}

#[test]
fn record_equality() {
    let a = CheckpointRecord::new("auto-1".to_string(), true);
    let b = CheckpointRecord::new("auto-1".to_string(), true);
    assert_eq!(a, b);
}

#[test]
fn record_inequality_different_id() {
    let a = CheckpointRecord::new("auto-1".to_string(), true);
    let b = CheckpointRecord::new("auto-2".to_string(), true);
    assert_ne!(a, b);
}

#[test]
fn record_inequality_different_committed() {
    let a = CheckpointRecord::new("auto-1".to_string(), true);
    let b = CheckpointRecord::new("auto-1".to_string(), false);
    assert_ne!(a, b);
}

#[test]
fn record_clone() {
    let a = CheckpointRecord::new("auto-clone".to_string(), false);
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn record_debug() {
    let record = CheckpointRecord::new("auto-dbg".to_string(), true);
    let debug = format!("{record:?}");
    assert!(debug.contains("auto-dbg"));
}

// === classify_command ===

#[test]
fn risky_commands() {
    let risky = ["batch", "spawn", "remove", "cleanup", "rebase", "squash"];
    for cmd in &risky {
        assert_eq!(
            classify_command(cmd),
            OperationRisk::Risky,
            "'{cmd}' should be Risky"
        );
    }
}

#[test]
fn safe_commands() {
    let safe = [
        "list", "status", "context", "focus", "help", "version", "show", "init", "switch", "done",
    ];
    for cmd in &safe {
        assert_eq!(
            classify_command(cmd),
            OperationRisk::Safe,
            "'{cmd}' should be Safe"
        );
    }
}

#[test]
fn empty_string_is_safe() {
    assert_eq!(classify_command(""), OperationRisk::Safe);
}

#[test]
fn unknown_command_is_safe() {
    assert_eq!(classify_command("xyzzy"), OperationRisk::Safe);
}

#[test]
fn classification_is_case_sensitive() {
    assert_eq!(classify_command("Batch"), OperationRisk::Safe);
    assert_eq!(classify_command("BATCH"), OperationRisk::Safe);
    assert_eq!(classify_command("batch"), OperationRisk::Risky);
    assert_eq!(classify_command("REBASE"), OperationRisk::Safe);
    assert_eq!(classify_command("rebase"), OperationRisk::Risky);
}

#[test]
fn classification_is_exact_match() {
    // Partial matches should be safe
    assert_eq!(classify_command("batch-"), OperationRisk::Safe);
    assert_eq!(classify_command("cleanup-"), OperationRisk::Safe);
    assert_eq!(classify_command("-spawn"), OperationRisk::Safe);
}

// === Proptests ===

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn classify_unknown_is_safe(cmd in "[a-zA-Z0-9_]{1,20}") {
            let risky_set = ["batch", "spawn", "remove", "cleanup", "rebase", "squash"];
            if !risky_set.contains(&cmd.as_str()) {
                prop_assert_eq!(classify_command(&cmd), OperationRisk::Safe);
            }
        }

        #[test]
        fn checkpoint_state_roundtrip(s in "(pending|committed|needs_restore)") {
            let state = CheckpointState::from_db(&s).unwrap();
            prop_assert_eq!(state.as_db(), s);
        }

        #[test]
        fn checkpoint_record_equality_same(id in "[a-zA-Z0-9_-]{1,20}", committed in proptest::bool::ANY) {
            let a = CheckpointRecord::new(id.clone(), committed);
            let b = CheckpointRecord::new(id, committed);
            prop_assert_eq!(a, b);
        }

        #[test]
        fn operation_risk_serde_roundtrip(is_risky in proptest::bool::ANY) {
            let risk = if is_risky { OperationRisk::Risky } else { OperationRisk::Safe };
            let json = serde_json::to_string(&risk).unwrap();
            let parsed: OperationRisk = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(risk, parsed);
        }
    }
}
