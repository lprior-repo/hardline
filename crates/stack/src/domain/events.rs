#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::stack::StackId;
use super::state::StackState;
use super::value_objects::BranchName;

// ---------------------------------------------------------------------------
// Stack domain events
// ---------------------------------------------------------------------------

/// Domain events emitted by valid Stack state transitions.
///
/// Each variant corresponds to exactly one valid transition edge in the
/// Stack state machine. Invalid transitions produce `InvalidTransition`
/// instead of an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackEvent {
    /// Draft → Published
    StackPublished {
        stack_id: StackId,
        branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
    /// Published → Merging
    StackMerging {
        stack_id: StackId,
        branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
    /// Merging → Merged (terminal)
    StackMerged {
        stack_id: StackId,
        branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
    /// any non-Failed → Failed
    StackFailed {
        stack_id: StackId,
        previous_state: StackState,
        branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
    /// Merging → Conflict
    StackConflict {
        stack_id: StackId,
        conflicting_branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
    /// Conflict → Published (resolution)
    StackResolved {
        stack_id: StackId,
        resolved_branches: Vec<BranchName>,
        timestamp: DateTime<Utc>,
    },
}

// ---------------------------------------------------------------------------
// Invalid transition
// ---------------------------------------------------------------------------

/// Returned when a transition between two `StackState`s is not part of the
/// valid state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: StackState,
    pub to: StackState,
}

impl std::fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid transition: {:?} -> {:?}", self.from, self.to)
    }
}

impl std::error::Error for InvalidTransition {}

// ---------------------------------------------------------------------------
// Transition → Event mapping
// ---------------------------------------------------------------------------

/// Attempt a state transition and return the corresponding domain event.
///
/// # Errors
///
/// Returns `InvalidTransition` when `(from, to)` is not a valid state machine edge.
///
/// Returns `Ok(StackEvent)` when `(from, to)` is a valid edge.
/// Returns `Err(InvalidTransition)` for every other pair.
///
/// Valid edges:
/// ```text
/// Draft     → Published   ⇒ StackPublished
/// Published → Merging     ⇒ StackMerging
/// Merging   → Merged      ⇒ StackMerged
/// Merging   → Conflict    ⇒ StackConflict
/// Conflict  → Published   ⇒ StackResolved
/// X (!=Failed) → Failed   ⇒ StackFailed   (from captured in previous_state)
/// ```
pub fn try_transition(
    stack_id: StackId,
    from: StackState,
    to: StackState,
    branches: Vec<BranchName>,
) -> Result<StackEvent, InvalidTransition> {
    let timestamp = Utc::now();
    match (from, to) {
        (StackState::Draft, StackState::Published) => Ok(StackEvent::StackPublished {
            stack_id,
            branches,
            timestamp,
        }),
        (StackState::Published, StackState::Merging) => Ok(StackEvent::StackMerging {
            stack_id,
            branches,
            timestamp,
        }),
        (StackState::Merging, StackState::Merged) => Ok(StackEvent::StackMerged {
            stack_id,
            branches,
            timestamp,
        }),
        (StackState::Merging, StackState::Conflict) => Ok(StackEvent::StackConflict {
            stack_id,
            conflicting_branches: branches,
            timestamp,
        }),
        (StackState::Conflict, StackState::Published) => Ok(StackEvent::StackResolved {
            stack_id,
            resolved_branches: branches,
            timestamp,
        }),
        (from, StackState::Failed) if from != StackState::Failed => {
            Ok(StackEvent::StackFailed {
                stack_id,
                previous_state: from,
                branches,
                timestamp,
            })
        }
        _ => Err(InvalidTransition { from, to }),
    }
}

/// Return `true` when `(from, to)` is a valid transition edge.
#[must_use]
pub fn is_valid_transition(from: StackState, to: StackState) -> bool {
    try_transition(StackId::from_u64(0), from, to, Vec::new()).is_ok()
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

impl StackEvent {
    /// The stack id carried by every event variant.
    #[must_use]
    pub fn stack_id(&self) -> StackId {
        match self {
            Self::StackPublished { stack_id, .. }
            | Self::StackMerging { stack_id, .. }
            | Self::StackMerged { stack_id, .. }
            | Self::StackFailed { stack_id, .. }
            | Self::StackConflict { stack_id, .. }
            | Self::StackResolved { stack_id, .. } => *stack_id,
        }
    }

    /// The timestamp carried by every event variant.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::StackPublished { timestamp, .. }
            | Self::StackMerging { timestamp, .. }
            | Self::StackMerged { timestamp, .. }
            | Self::StackFailed { timestamp, .. }
            | Self::StackConflict { timestamp, .. }
            | Self::StackResolved { timestamp, .. } => *timestamp,
        }
    }

    /// Human-readable event kind name (no payload).
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::StackPublished { .. } => "StackPublished",
            Self::StackMerging { .. } => "StackMerging",
            Self::StackMerged { .. } => "StackMerged",
            Self::StackFailed { .. } => "StackFailed",
            Self::StackConflict { .. } => "StackConflict",
            Self::StackResolved { .. } => "StackResolved",
        }
    }

    /// All variant names in declaration order.
    #[must_use]
    pub fn all_kinds() -> &'static [&'static str] {
        &[
            "StackPublished",
            "StackMerging",
            "StackMerged",
            "StackFailed",
            "StackConflict",
            "StackResolved",
        ]
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::discriminant;

    // -- helpers -----------------------------------------------------------

    fn sid(n: u64) -> StackId {
        StackId::from_u64(n)
    }

    fn branches(names: &[&str]) -> Vec<BranchName> {
        names.iter().map(|n| BranchName::new(*n)).collect()
    }

    // -- valid transitions → correct event type ---------------------------

    #[test]
    fn draft_to_published_emits_stack_published() {
        let event = try_transition(sid(1), StackState::Draft, StackState::Published, branches(&["f"]))
            .expect("should succeed");
        assert!(matches!(event, StackEvent::StackPublished { .. }));
        assert_eq!(event.kind(), "StackPublished");
    }

    #[test]
    fn published_to_merging_emits_stack_merging() {
        let event = try_transition(sid(2), StackState::Published, StackState::Merging, branches(&["f"]))
            .expect("should succeed");
        assert!(matches!(event, StackEvent::StackMerging { .. }));
        assert_eq!(event.kind(), "StackMerging");
    }

    #[test]
    fn merging_to_merged_emits_stack_merged() {
        let event = try_transition(sid(3), StackState::Merging, StackState::Merged, branches(&["f"]))
            .expect("should succeed");
        assert!(matches!(event, StackEvent::StackMerged { .. }));
        assert_eq!(event.kind(), "StackMerged");
    }

    #[test]
    fn merging_to_conflict_emits_stack_conflict() {
        let event = try_transition(
            sid(4),
            StackState::Merging,
            StackState::Conflict,
            branches(&["f", "g"]),
        )
        .expect("should succeed");
        assert!(matches!(event, StackEvent::StackConflict { .. }));
        assert_eq!(event.kind(), "StackConflict");
    }

    #[test]
    fn conflict_to_published_emits_stack_resolved() {
        let event = try_transition(
            sid(5),
            StackState::Conflict,
            StackState::Published,
            branches(&["f"]),
        )
        .expect("should succeed");
        assert!(matches!(event, StackEvent::StackResolved { .. }));
        assert_eq!(event.kind(), "StackResolved");
    }

    #[test]
    fn any_non_failed_to_failed_emits_stack_failed() {
        let non_failed = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Conflict,
            StackState::Merged,
        ];
        for from in non_failed {
            let event =
                try_transition(sid(10), from, StackState::Failed, Vec::new()).expect("should succeed");
            assert!(
                matches!(event, StackEvent::StackFailed { .. }),
                "from {from:?} → Failed should emit StackFailed"
            );
            assert_eq!(event.kind(), "StackFailed");
        }
    }

    // -- event payload: stack_id ------------------------------------------

    #[test]
    fn event_carries_correct_stack_id() {
        let cases: Vec<StackEvent> = vec![
            try_transition(sid(100), StackState::Draft, StackState::Published, Vec::new()).expect("a"),
            try_transition(sid(200), StackState::Published, StackState::Merging, Vec::new()).expect("b"),
            try_transition(sid(300), StackState::Merging, StackState::Merged, Vec::new()).expect("c"),
            try_transition(sid(400), StackState::Draft, StackState::Failed, Vec::new()).expect("d"),
            try_transition(sid(500), StackState::Merging, StackState::Conflict, Vec::new()).expect("e"),
            try_transition(sid(600), StackState::Conflict, StackState::Published, Vec::new()).expect("f"),
        ];
        let expected: Vec<StackId> = [100, 200, 300, 400, 500, 600].map(sid).to_vec();
        for (event, want) in cases.iter().zip(expected.iter()) {
            assert_eq!(event.stack_id(), *want);
        }
    }

    // -- event payload: branches ------------------------------------------

    #[test]
    fn published_event_carries_branches() {
        let event = try_transition(
            sid(1),
            StackState::Draft,
            StackState::Published,
            branches(&["main", "feat-a"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackPublished { branches, .. } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(branches[0].as_str(), "main");
                assert_eq!(branches[1].as_str(), "feat-a");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn merging_event_carries_branches() {
        let event = try_transition(
            sid(1),
            StackState::Published,
            StackState::Merging,
            branches(&["feat-b"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackMerging { branches, .. } => {
                assert_eq!(branches.len(), 1);
                assert_eq!(branches[0].as_str(), "feat-b");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn merged_event_carries_branches() {
        let event = try_transition(
            sid(1),
            StackState::Merging,
            StackState::Merged,
            branches(&["x", "y", "z"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackMerged { branches, .. } => {
                assert_eq!(branches.len(), 3);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn conflict_event_carries_conflicting_branches() {
        let event = try_transition(
            sid(1),
            StackState::Merging,
            StackState::Conflict,
            branches(&["conflict-a", "conflict-b"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackConflict {
                conflicting_branches,
                ..
            } => {
                assert_eq!(conflicting_branches.len(), 2);
                assert_eq!(conflicting_branches[0].as_str(), "conflict-a");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn resolved_event_carries_resolved_branches() {
        let event = try_transition(
            sid(1),
            StackState::Conflict,
            StackState::Published,
            branches(&["resolved-x"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackResolved {
                resolved_branches, ..
            } => {
                assert_eq!(resolved_branches.len(), 1);
                assert_eq!(resolved_branches[0].as_str(), "resolved-x");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn failed_event_carries_branches_and_previous_state() {
        let event = try_transition(
            sid(99),
            StackState::Merging,
            StackState::Failed,
            branches(&["broken"]),
        )
        .expect("ok");
        match event {
            StackEvent::StackFailed {
                previous_state,
                branches,
                ..
            } => {
                assert_eq!(previous_state, StackState::Merging);
                assert_eq!(branches.len(), 1);
                assert_eq!(branches[0].as_str(), "broken");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn failed_event_previous_state_varies() {
        let froms = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Conflict,
            StackState::Merged,
        ];
        for from in froms {
            let event =
                try_transition(sid(1), from, StackState::Failed, Vec::new()).expect("ok");
            match event {
                StackEvent::StackFailed { previous_state, .. } => {
                    assert_eq!(previous_state, from);
                }
                _ => panic!("expected StackFailed"),
            }
        }
    }

    // -- event payload: timestamp -----------------------------------------

    #[test]
    fn event_timestamp_is_recent() {
        let before = Utc::now();
        let event = try_transition(sid(1), StackState::Draft, StackState::Published, Vec::new())
            .expect("ok");
        let after = Utc::now();
        let ts = event.timestamp();
        assert!(ts >= before, "timestamp should be >= before");
        assert!(ts <= after, "timestamp should be <= after");
    }

    // -- events emitted in order for multi-step workflow ------------------

    #[test]
    fn happy_path_emits_events_in_order() {
        let id = sid(1);
        let br = branches(&["feat"]);

        let e1 = try_transition(id, StackState::Draft, StackState::Published, br.clone()).expect("1");
        let e2 = try_transition(id, StackState::Published, StackState::Merging, br.clone()).expect("2");
        let e3 = try_transition(id, StackState::Merging, StackState::Merged, br).expect("3");

        assert_eq!(e1.kind(), "StackPublished");
        assert_eq!(e2.kind(), "StackMerging");
        assert_eq!(e3.kind(), "StackMerged");

        // timestamps are non-decreasing
        assert!(e1.timestamp() <= e2.timestamp());
        assert!(e2.timestamp() <= e3.timestamp());
    }

    #[test]
    fn conflict_resolution_path_emits_events_in_order() {
        let id = sid(2);
        let br = branches(&["feat"]);

        let e1 = try_transition(id, StackState::Draft, StackState::Published, br.clone()).expect("1");
        let e2 = try_transition(id, StackState::Published, StackState::Merging, br.clone()).expect("2");
        let e3 = try_transition(id, StackState::Merging, StackState::Conflict, br.clone()).expect("3");
        let e4 = try_transition(id, StackState::Conflict, StackState::Published, br).expect("4");

        assert_eq!(e1.kind(), "StackPublished");
        assert_eq!(e2.kind(), "StackMerging");
        assert_eq!(e3.kind(), "StackConflict");
        assert_eq!(e4.kind(), "StackResolved");

        assert!(e1.timestamp() <= e2.timestamp());
        assert!(e2.timestamp() <= e3.timestamp());
        assert!(e3.timestamp() <= e4.timestamp());
    }

    #[test]
    fn failure_from_any_step_path() {
        let id = sid(3);
        let br = branches(&["feat"]);

        // Fail at draft
        let e1 = try_transition(id, StackState::Draft, StackState::Failed, br.clone()).expect("1");
        assert_eq!(e1.kind(), "StackFailed");

        // Fail at published
        let e2 = try_transition(id, StackState::Published, StackState::Failed, br.clone()).expect("2");
        assert_eq!(e2.kind(), "StackFailed");

        // Fail at merging
        let e3 = try_transition(id, StackState::Merging, StackState::Failed, br).expect("3");
        assert_eq!(e3.kind(), "StackFailed");
    }

    // -- no events on invalid transitions ---------------------------------

    #[test]
    fn same_state_transition_is_invalid() {
        let states = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Merged,
            StackState::Conflict,
            StackState::Failed,
        ];
        for s in states {
            assert!(
                !is_valid_transition(s, s),
                "{s:?} → {s:?} should be invalid"
            );
            let result = try_transition(sid(1), s, s, Vec::new());
            assert!(result.is_err(), "{s:?} → {s:?} should be Err");
            let err = result.err().expect("err");
            assert_eq!(err.from, s);
            assert_eq!(err.to, s);
        }
    }

    #[test]
    fn backwards_transitions_are_invalid() {
        let invalid = [
            (StackState::Published, StackState::Draft),
            (StackState::Merging, StackState::Draft),
            (StackState::Merged, StackState::Draft),
            (StackState::Merged, StackState::Published),
            (StackState::Merged, StackState::Merging),
            (StackState::Conflict, StackState::Draft),
            (StackState::Conflict, StackState::Merging),
            (StackState::Conflict, StackState::Merged),
            (StackState::Failed, StackState::Draft),
            (StackState::Failed, StackState::Published),
            (StackState::Failed, StackState::Merging),
            (StackState::Failed, StackState::Merged),
            (StackState::Failed, StackState::Conflict),
        ];
        for (from, to) in invalid {
            assert!(
                !is_valid_transition(from, to),
                "{from:?} → {to:?} should be invalid"
            );
        }
    }

    #[test]
    fn invalid_cross_transitions() {
        let invalid = [
            (StackState::Draft, StackState::Merging),
            (StackState::Draft, StackState::Merged),
            (StackState::Draft, StackState::Conflict),
            (StackState::Published, StackState::Draft),
            (StackState::Published, StackState::Merged),
            (StackState::Published, StackState::Conflict),
            (StackState::Merged, StackState::Conflict),
        ];
        for (from, to) in invalid {
            assert!(
                !is_valid_transition(from, to),
                "{from:?} → {to:?} should be invalid"
            );
        }
    }

    #[test]
    fn failed_to_failed_is_invalid() {
        assert!(!is_valid_transition(StackState::Failed, StackState::Failed));
    }

    #[test]
    fn invalid_transition_error_display() {
        let err = InvalidTransition {
            from: StackState::Merged,
            to: StackState::Draft,
        };
        let msg = format!("{err}");
        assert!(msg.contains("invalid transition"));
        assert!(msg.contains("Merged"));
        assert!(msg.contains("Draft"));
    }

    #[test]
    fn invalid_transition_is_std_error() {
        let err = InvalidTransition {
            from: StackState::Draft,
            to: StackState::Draft,
        };
        let _: &dyn std::error::Error = &err;
    }

    // -- event discriminants are unique -----------------------------------

    #[test]
    fn all_event_variants_have_unique_discriminants() {
        let id = sid(1);
        let br = branches(&["x"]);

        let events: Vec<StackEvent> = vec![
            try_transition(id, StackState::Draft, StackState::Published, br.clone()).expect("a"),
            try_transition(id, StackState::Published, StackState::Merging, br.clone()).expect("b"),
            try_transition(id, StackState::Merging, StackState::Merged, br.clone()).expect("c"),
            try_transition(id, StackState::Draft, StackState::Failed, br.clone()).expect("d"),
            try_transition(id, StackState::Merging, StackState::Conflict, br.clone()).expect("e"),
            try_transition(id, StackState::Conflict, StackState::Published, br).expect("f"),
        ];

        let mut seen: Vec<std::mem::Discriminant<StackEvent>> = Vec::new();
        for event in &events {
            let d = discriminant(event);
            assert!(
                !seen.contains(&d),
                "duplicate discriminant for {:?}",
                event.kind()
            );
            seen.push(d);
        }
        assert_eq!(seen.len(), 6, "should have exactly 6 distinct discriminants");
    }

    // -- event kind -------------------------------------------------------

    #[test]
    fn all_kinds_returns_six_names() {
        let kinds = StackEvent::all_kinds();
        assert_eq!(kinds.len(), 6);
        assert_eq!(kinds[0], "StackPublished");
        assert_eq!(kinds[1], "StackMerging");
        assert_eq!(kinds[2], "StackMerged");
        assert_eq!(kinds[3], "StackFailed");
        assert_eq!(kinds[4], "StackConflict");
        assert_eq!(kinds[5], "StackResolved");
    }

    #[test]
    fn all_kinds_are_distinct() {
        let kinds = StackEvent::all_kinds().to_vec();
        let mut sorted = kinds.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), kinds.len(), "all kinds must be unique");
    }

    // -- is_valid_transition: exhaustive matrix ---------------------------

    #[test]
    fn valid_transition_matrix() {
        let states = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Merged,
            StackState::Conflict,
            StackState::Failed,
        ];

        // Expected valid edges
        let valid = [
            (StackState::Draft, StackState::Published),
            (StackState::Published, StackState::Merging),
            (StackState::Merging, StackState::Merged),
            (StackState::Merging, StackState::Conflict),
            (StackState::Conflict, StackState::Published),
            (StackState::Draft, StackState::Failed),
            (StackState::Published, StackState::Failed),
            (StackState::Merging, StackState::Failed),
            (StackState::Conflict, StackState::Failed),
            (StackState::Merged, StackState::Failed),
        ];

        let mut valid_count = 0;
        for from in states {
            for to in states {
                let expected = valid.contains(&(from, to));
                let actual = is_valid_transition(from, to);
                assert_eq!(
                    actual, expected,
                    "is_valid_transition({from:?}, {to:?}) should be {expected}"
                );
                if expected {
                    valid_count += 1;
                }
            }
        }
        assert_eq!(valid_count, valid.len());
    }

    // -- serialization roundtrips -----------------------------------------

    #[test]
    fn all_event_variants_serde_roundtrip() {
        let id = sid(42);
        let br = branches(&["branch-a", "branch-b"]);

        let events: Vec<StackEvent> = vec![
            try_transition(id, StackState::Draft, StackState::Published, br.clone()).expect("a"),
            try_transition(id, StackState::Published, StackState::Merging, br.clone()).expect("b"),
            try_transition(id, StackState::Merging, StackState::Merged, br.clone()).expect("c"),
            try_transition(id, StackState::Draft, StackState::Failed, br.clone()).expect("d"),
            try_transition(id, StackState::Merging, StackState::Conflict, br.clone()).expect("e"),
            try_transition(id, StackState::Conflict, StackState::Published, br).expect("f"),
        ];

        for original in &events {
            let json = serde_json::to_string(original).expect("serialize");
            let restored: StackEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(original.kind(), restored.kind());
            assert_eq!(original.stack_id(), restored.stack_id());
        }
    }

    // -- clone / debug ----------------------------------------------------

    #[test]
    fn event_is_clone() {
        let event = try_transition(sid(1), StackState::Draft, StackState::Published, Vec::new())
            .expect("ok");
        let cloned = event.clone();
        assert_eq!(event.kind(), cloned.kind());
        assert_eq!(event.stack_id(), cloned.stack_id());
    }

    #[test]
    fn event_debug_contains_variant_name() {
        let event = try_transition(sid(1), StackState::Draft, StackState::Published, Vec::new())
            .expect("ok");
        let debug = format!("{event:?}");
        assert!(debug.contains("StackPublished"));
    }

    // -- empty branches edge case -----------------------------------------

    #[test]
    fn event_with_empty_branches() {
        let event = try_transition(sid(1), StackState::Draft, StackState::Published, Vec::new())
            .expect("ok");
        match event {
            StackEvent::StackPublished { branches, .. } => {
                assert!(branches.is_empty());
            }
            _ => panic!("wrong variant"),
        }
    }

    // -- invalid transition error preserves from/to -----------------------

    #[test]
    fn invalid_transition_error_fields() {
        let result = try_transition(sid(1), StackState::Merged, StackState::Draft, Vec::new());
        let err = result.err().expect("should be error");
        assert_eq!(err.from, StackState::Merged);
        assert_eq!(err.to, StackState::Draft);
    }

    #[test]
    fn invalid_transition_error_clone_eq() {
        let err1 = InvalidTransition {
            from: StackState::Draft,
            to: StackState::Draft,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn invalid_transition_error_debug() {
        let err = InvalidTransition {
            from: StackState::Draft,
            to: StackState::Merged,
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("Draft"));
        assert!(debug.contains("Merged"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        /// Any state pair NOT in the valid set must produce Err.
        #[test]
        fn prop_invalid_transitions_produce_error(
            from_idx in 0u8..6u8,
            to_idx in 0u8..6u8,
        ) {
            let all = [
                StackState::Draft,
                StackState::Published,
                StackState::Merging,
                StackState::Merged,
                StackState::Conflict,
                StackState::Failed,
            ];
            let from = all[from_idx as usize];
            let to = all[to_idx as usize];
            let result = try_transition(StackId::from_u64(1), from, to, Vec::new());

            let valid = matches!(
                (from, to),
                (StackState::Draft, StackState::Published)
                | (StackState::Published, StackState::Merging)
                | (StackState::Merging, StackState::Merged)
                | (StackState::Merging, StackState::Conflict)
                | (StackState::Conflict, StackState::Published)
                | (_, StackState::Failed) if from != StackState::Failed
            );

            assert_eq!(result.is_ok(), valid, "transition {from:?} → {to:?}: expected valid={valid}");
        }

        /// StackId survives through the event round-trip.
        #[test]
        fn prop_event_stack_id_roundtrip(id in 1u64..100_000u64) {
            let event = try_transition(
                StackId::from_u64(id),
                StackState::Draft,
                StackState::Published,
                Vec::new(),
            )
            .expect("valid");
            assert_eq!(event.stack_id().to_u64(), id);
        }

        /// Branch names survive through the event.
        #[test]
        fn prop_event_preserves_branch_names(branches in proptest::collection::vec(".{0,20}", 0..10)) {
            let expected: Vec<BranchName> = branches.iter().map(|s| BranchName::new(s.clone())).collect();
            let event = try_transition(
                StackId::from_u64(1),
                StackState::Draft,
                StackState::Published,
                expected.clone(),
            )
            .expect("valid");
            match event {
                StackEvent::StackPublished { branches, .. } => {
                    assert_eq!(branches.len(), expected.len());
                    for (got, want) in branches.iter().zip(expected.iter()) {
                        assert_eq!(got.as_str(), want.as_str());
                    }
                }
                _ => panic!("wrong variant"),
            }
        }

        /// serde roundtrip preserves kind for all valid transitions.
        #[test]
        fn prop_serde_roundtrip_preserves_kind(
            from_idx in 0u8..5u8, // exclude Failed (index 5) as 'from' for non-failed transitions
            to_idx in 0u8..6u8,
        ) {
            let all = [
                StackState::Draft,
                StackState::Published,
                StackState::Merging,
                StackState::Merged,
                StackState::Conflict,
                StackState::Failed,
            ];
            let from = all[from_idx as usize];
            let to = all[to_idx as usize];

            if let Ok(event) = try_transition(StackId::from_u64(99), from, to, vec![BranchName::new("b")]) {
                let json = serde_json::to_string(&event).expect("ser");
                let restored: StackEvent = serde_json::from_str(&json).expect("de");
                assert_eq!(event.kind(), restored.kind());
                assert_eq!(event.stack_id(), restored.stack_id());
            }
        }
    }
}
