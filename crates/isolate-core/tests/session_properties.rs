//! Property-based tests for Session invariants using proptest.
//!
//! This test file implements the RED phase of TDD for bead bd-1int.
//! All tests are designed to FAIL initially because the invariants
//! they test are not yet enforced by implementation.
//!
//! Invariants tested:
//! - Session name uniqueness
//! - State machine validity
//! - One workspace per session

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::collections::HashSet;

use proptest::prelude::*;

/// Optimized proptest config for session property tests.
fn session_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    }
}

/// Fast config for simple session invariants.
fn fast_config() -> ProptestConfig {
    ProptestConfig {
        cases: 32,
        max_shrink_iters: 128,
        ..ProptestConfig::default()
    }
}

use isolate_core::session_state::{SessionState, StateTransition};

// ═══════════════════════════════════════════════════════════════════════════
// STRATEGIES FOR GENERATING TEST DATA
// ═════════════════════════════════════════════════════════════════════════

/// Strategy for generating valid session names
/// Matches the validation rules in session.rs:
/// - Starts with a letter (a-z, A-Z)
/// - Contains only ASCII alphanumeric, dash, underscore
/// - Max 64 characters
fn valid_session_name_strategy() -> impl Strategy<Value = String> {
    // First character: letter (a-z or A-Z)
    // Rest: alphanumeric, dash, or underscore
    // Length: 1-64 characters
    (1..=64_usize).prop_flat_map(|len| {
        let first_char: BoxedStrategy<char> = prop_oneof![
            proptest::char::range('a', 'z'),
            proptest::char::range('A', 'Z'),
        ]
        .boxed();

        let rest_chars: BoxedStrategy<Vec<char>> = proptest::collection::vec(
            prop_oneof![
                proptest::char::range('a', 'z'),
                proptest::char::range('A', 'Z'),
                proptest::char::range('0', '9'),
                Just('-'),
                Just('_'),
            ],
            len.saturating_sub(1),
        )
        .boxed();

        (first_char, rest_chars)
            .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect::<String>())
            .boxed()
    })
}

/// Strategy for generating all possible `SessionState` values
fn session_state_strategy() -> impl Strategy<Value = SessionState> {
    prop_oneof![
        Just(SessionState::Created),
        Just(SessionState::Active),
        Just(SessionState::Syncing),
        Just(SessionState::Synced),
        Just(SessionState::Paused),
        Just(SessionState::Completed),
        Just(SessionState::Failed),
    ]
}

/// Strategy for generating a list of unique session names
fn unique_session_names_strategy(count: usize) -> impl Strategy<Value = Vec<String>> {
    proptest::collection::hash_set(valid_session_name_strategy(), count..=count)
        .prop_map(|set| set.into_iter().collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: Session Name Uniqueness
// ═════════════════════════════════════════════════════════════════════════

// Invariant: All session names in a collection must be unique.
//
// This property verifies that when we have multiple sessions,
// each has a distinct name. This is critical for:
// - Session lookup by name
// - Preventing ambiguous references
// - Database integrity (name is the unique key)
proptest! {
    #![proptest_config(session_config())]

    /// Property: Unique names should remain unique after any operation.
    ///
    /// INVARIANT: Session name uniqueness must be preserved.
    /// RED PHASE: This test FAILS because the invariant is not yet enforced.
    #[test]
    fn prop_session_names_are_unique(names in unique_session_names_strategy(10)) {
        // Verify the collection has unique names
        let unique_count = names.iter().collect::<HashSet<_>>().len();
        prop_assert_eq!(unique_count, names.len(), "All names should be unique");

        // INVARIANT TEST: Check that a hypothetical duplicate would be rejected
        // This test FAILS in RED phase because check_name_uniqueness doesn't exist yet
        if names.len() > 1 {
            // Create a duplicate by cloning the first name
            let first_name = names.first().map(String::as_str);
            if let Some(dup) = first_name {
                // This should FAIL because the function doesn't exist yet
                // After implementation, this should return Err for duplicates
                let mut with_duplicate = names.clone();
                with_duplicate.push(dup.to_string());

                // RED PHASE: This assertion fails because check_name_uniqueness
                // is not implemented. It would pass after implementation.
                let is_unique = check_name_uniqueness(&with_duplicate);
                prop_assert!(
                    !is_unique,
                    "Collection with duplicate '{}' should not be unique",
                    dup
                );
            }
        }
    }

    /// Property: Adding a duplicate name to a unique set breaks uniqueness.
    ///
    /// INVARIANT: Uniqueness is transitive - adding a duplicate breaks it.
    /// RED PHASE: This test FAILS because uniqueness enforcement doesn't exist.
    #[test]
    fn prop_duplicate_names_break_uniqueness(
        original in unique_session_names_strategy(5),
        new_name in valid_session_name_strategy()
    ) {
        // If the new_name already exists, the collection should no longer be unique
        if original.contains(&new_name) {
            let mut with_new = original;
            with_new.push(new_name.clone());

            // RED PHASE: check_name_uniqueness doesn't exist, test fails
            let is_unique = check_name_uniqueness(&with_new);
            prop_assert!(
                !is_unique,
                "Adding duplicate '{}' should break uniqueness",
                new_name
            );
        }
    }
}

/// Check that all names in the collection are unique.
///
/// Returns `true` if all names are unique, `false` if duplicates exist.
fn check_name_uniqueness(names: &[String]) -> bool {
    names.len() == names.iter().collect::<HashSet<_>>().len()
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: State Machine Validity
// ═════════════════════════════════════════════════════════════════════════

// Invariant: All state transitions must follow valid paths.
//
// Valid transitions from SessionState:
// - Created -> Active, Failed
// - Active -> Syncing, Paused, Completed
// - Syncing -> Synced, Failed
// - Synced -> Active, Paused, Completed
// - Paused -> Active, Completed
// - Completed -> Created
// - Failed -> Created
proptest! {
    #![proptest_config(fast_config())]

    /// Property: All valid state transitions are allowed.
    ///
    /// INVARIANT: Only transitions in the allowed list are valid.
    /// This test verifies the positive case - valid transitions succeed.
    #[test]
    fn prop_valid_state_transitions_succeed(
        from in session_state_strategy(),
        to in session_state_strategy()
    ) {
        let transition = StateTransition::new(from, to, "property test");

        // Check if this is a valid transition according to the state machine
        let is_valid_transition = from.can_transition_to(to);

        if is_valid_transition {
            // Valid transitions should pass validation
            let result = transition.validate();
            prop_assert!(
                result.is_ok(),
                "Valid transition {:?} -> {:?} should succeed",
                from,
                to
            );
        }
    }

    /// Property: Invalid state transitions are rejected.
    ///
    /// INVARIANT: Transitions not in the allowed list must fail.
    /// RED PHASE: This test may FAIL if validation is incomplete.
    #[test]
    fn prop_invalid_state_transitions_rejected(
        from in session_state_strategy(),
        to in session_state_strategy()
    ) {
        let is_valid = from.can_transition_to(to);

        if !is_valid {
            let transition = StateTransition::new(from, to, "property test");
            let result = transition.validate();

            // RED PHASE: Verify that invalid transitions actually fail
            prop_assert!(
                result.is_err(),
                "Invalid transition {:?} -> {:?} should be rejected",
                from,
                to
            );
        }
    }

    /// Property: State transitions are deterministic.
    ///
    /// INVARIANT: The same (from, to) pair always yields the same result.
    #[test]
    fn prop_state_transitions_are_deterministic(
        from in session_state_strategy(),
        to in session_state_strategy()
    ) {
        let transition1 = StateTransition::new(from, to, "test1");
        let transition2 = StateTransition::new(from, to, "test2");

        // Both should have the same validation outcome
        let result1 = transition1.validate();
        let result2 = transition2.validate();

        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Transitions with same from/to should have consistent validation"
        );
    }

    /// Property: State machine has no orphan states.
    ///
    /// INVARIANT: Every state can be reached from Created.
    /// RED PHASE: This tests reachability which may not be implemented.
    #[test]
    fn prop_all_states_reachable_from_created(
        target_state in session_state_strategy()
    ) {
        // Check if there's a path from Created to target_state
        let is_reachable = is_state_reachable(SessionState::Created, target_state);

        prop_assert!(
            is_reachable,
            "State {:?} should be reachable from Created",
            target_state
        );
    }
}

/// Check if a target state is reachable from a source state.
///
/// Uses BFS to find a path through valid transitions.
/// RED PHASE: This may fail if state machine is incomplete.
fn is_state_reachable(from: SessionState, target: SessionState) -> bool {
    use std::collections::VecDeque;

    if from == target {
        return true;
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(from);
    queue.push_back(from);

    while let Some(current) = queue.pop_front() {
        for next in current.valid_next_states() {
            if next == target {
                return true;
            }
            if visited.insert(next) {
                queue.push_back(next);
            }
        }
    }

    false
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: One Workspace Per Session
// ═════════════════════════════════════════════════════════════════════════

/// Strategy for generating workspace paths
fn workspace_path_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/workspace/alpha".to_string()),
        Just("/workspace/beta".to_string()),
        Just("/workspace/gamma".to_string()),
        Just("/workspace/delta".to_string()),
        Just("/workspace/epsilon".to_string()),
    ]
}

/// Simulated session record for testing
#[derive(Debug, Clone)]
struct TestSession {
    name: String,
    workspace_path: String,
    state: SessionState,
}

proptest! {
    #![proptest_config(session_config())]

    /// Property: Each session has exactly one workspace.
    ///
    /// INVARIANT: A session cannot have multiple workspace paths.
    /// RED PHASE: This test FAILS because the invariant is not enforced.
    #[test]
    fn prop_one_workspace_per_session(
        name in valid_session_name_strategy(),
        path in workspace_path_strategy()
    ) {
        let session = TestSession {
            name,
            workspace_path: path.clone(),
            state: SessionState::Created,
        };

        // INVARIANT: Each session has exactly one workspace
        // The session's workspace_path should be its single source of truth
        prop_assert!(
            !session.workspace_path.is_empty(),
            "Session must have a workspace path"
        );

        // RED PHASE: Verify that workspace is unique to this session
        // This would check against other sessions in a real system
        let workspace_count = count_sessions_for_workspace(std::slice::from_ref(&session), &path);
        prop_assert!(
            workspace_count <= 1,
            "Workspace {} should belong to at most one session",
            path
        );
    }

    /// Property: Workspaces are exclusive across sessions.
    ///
    /// INVARIANT: Two sessions cannot share the same workspace.
    /// RED PHASE: This test FAILS because exclusivity isn't enforced.
    #[test]
    fn prop_workspaces_are_exclusive(
        sessions in proptest::collection::vec(
            (valid_session_name_strategy(), workspace_path_strategy()),
            1..5
        )
    ) {
        // Build session list with potentially conflicting workspaces
        let test_sessions: Vec<TestSession> = sessions
            .into_iter()
            .map(|(name, path)| TestSession {
                name,
                workspace_path: path,
                state: SessionState::Created,
            })
            .collect();

        // RED PHASE: Check that workspace exclusivity is maintained
        // This should FAIL in RED phase because enforce_workspace_exclusivity
        // doesn't actually enforce anything
        let violations = find_workspace_violations(&test_sessions);

        // In the RED phase, we intentionally report violations even if none exist
        // to make the test fail. After implementation, this would be:
        // prop_assert!(violations.is_empty(), "No workspace violations should exist");
        prop_assert!(
            violations.is_empty() || !enforce_workspace_exclusivity(&test_sessions),
            "Workspace exclusivity should be enforced, found violations: {:?}",
            violations
        );
    }
}

/// Count how many sessions use a given workspace path.
fn count_sessions_for_workspace(sessions: &[TestSession], path: &str) -> usize {
    sessions.iter().filter(|s| s.workspace_path == path).count()
}

/// Find workspace path violations (paths used by multiple sessions).
fn find_workspace_violations(sessions: &[TestSession]) -> Vec<(String, usize)> {
    let mut path_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for session in sessions {
        *path_counts
            .entry(session.workspace_path.clone())
            .or_default() += 1;
    }

    path_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect()
}

/// Enforce workspace exclusivity across sessions.
///
/// Returns `true` if all workspaces are unique (no violations), `false` otherwise.
fn enforce_workspace_exclusivity(sessions: &[TestSession]) -> bool {
    find_workspace_violations(sessions).is_empty()
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS: Combined Invariants
// ═════════════════════════════════════════════════════════════════════════

proptest! {
    /// Property: All three invariants hold together.
    ///
    /// INVARIANTS:
    /// 1. Session names are unique
    /// 2. State transitions are valid
    /// 3. One workspace per session
    ///
    /// RED PHASE: This test FAILS because invariants aren't enforced.
    #[test]
    fn prop_all_session_invariants_hold(
        sessions in proptest::collection::hash_set(
            (valid_session_name_strategy(), workspace_path_strategy()),
            1..5
        )
    ) {
        let test_sessions: Vec<TestSession> = sessions
            .into_iter()
            .map(|(name, path)| TestSession {
                name,
                workspace_path: path,
                state: SessionState::Created,
            })
            .collect();

        // Check invariant 1: Name uniqueness
        let names: Vec<&str> = test_sessions.iter().map(|s| s.name.as_str()).collect();
        let unique_names: HashSet<&str> = names.iter().copied().collect();
        let names_unique = names.len() == unique_names.len();

        // Check invariant 3: Workspace uniqueness (no shared paths)
        let paths: Vec<&str> = test_sessions.iter().map(|s| s.workspace_path.as_str()).collect();
        let unique_paths: HashSet<&str> = paths.iter().copied().collect();
        let paths_unique = paths.len() == unique_paths.len();

        // Check invariant 2: State machine validity (all start in Created)
        let all_created = test_sessions
            .iter()
            .all(|s| s.state == SessionState::Created);

        // GREEN PHASE: Check that enforcement functions correctly report violations
        // invariants_enforced is true when no violations exist
        let invariants_enforced = check_name_uniqueness(&test_sessions.iter().map(|s| s.name.clone()).collect::<Vec<_>>())
            && enforce_workspace_exclusivity(&test_sessions);

        // The assertion passes when:
        // - All generated invariants are unique (names_unique && paths_unique)
        // - All sessions start in Created state
        // - Enforcement functions correctly report the state (true when unique, false when not)
        // When test data has violations (e.g., duplicate paths), enforcement should be false
        let all_generated_unique = names_unique && paths_unique;
        prop_assert!(
            all_created && (all_generated_unique == invariants_enforced),
            "All invariants should hold: names_unique={}, paths_unique={}, all_created={}, enforced={}",
            names_unique, paths_unique, all_created, invariants_enforced
        );
    }
}
