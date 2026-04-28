//! Proptest invariants for worktree crate

use itertools::Itertools;
use proptest::{prelude::*, prop_assert, prop_assert_eq};
use worktree::{WorktreeId, WorktreeState, WorktreeTypeEnum};

fn arb_worktree_state() -> impl Strategy<Value = WorktreeState> {
    prop_oneof![
        Just(WorktreeState::Creating),
        Just(WorktreeState::Incomplete),
        Just(WorktreeState::Active),
        Just(WorktreeState::Suspended),
        Just(WorktreeState::Removing),
        Just(WorktreeState::Removed),
    ]
}

fn arb_worktree_type() -> impl Strategy<Value = WorktreeTypeEnum> {
    prop_oneof![
        Just(WorktreeTypeEnum::Development),
        Just(WorktreeTypeEnum::Testing),
        Just(WorktreeTypeEnum::Review),
        Just(WorktreeTypeEnum::Debugging),
        Just(WorktreeTypeEnum::Research),
    ]
}

#[allow(dead_code)]
fn arb_worktree_id() -> impl Strategy<Value = WorktreeId> {
    prop::collection::vec(any::<u8>(), 16).prop_map(|bytes| {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        WorktreeId::from_bytes(arr)
    })
}

// Test without proptest macro
#[test]
fn proptest_worktree_id_new_random_generates_unique_ids() {
    let ids: Vec<WorktreeId> = (0..100).map(|_| WorktreeId::new_random()).collect();
    let unique_count = ids.iter().unique().count();
    assert_eq!(unique_count, ids.len());
}

proptest! {
    #[test]
    fn proptest_worktree_id_from_string_valid_uuid_roundtrips(
        uuid_str in r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
    ) {
        let result = WorktreeId::from_string(&uuid_str);
        prop_assert!(result.is_ok());
        if let Ok(id) = result {
            prop_assert_eq!(id.to_string(), uuid_str);
        }
    }

    #[test]
    fn proptest_worktree_id_from_bytes_roundtrips(bytes in prop::array::uniform16(any::<u8>())) {
        let id = WorktreeId::from_bytes(bytes);
        prop_assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn proptest_worktree_state_round_trip_preserves_value(state in arb_worktree_state()) {
        let round_trip = WorktreeState::from_u8(state.as_u8()).unwrap();
        prop_assert_eq!(state, round_trip);
    }

    #[test]
    fn proptest_worktree_state_all_states_valid(u8_val in 0u8..=5) {
        let result = WorktreeState::from_u8(u8_val);
        prop_assert!(result.is_some());
    }

    #[test]
    fn proptest_worktree_state_invalid_u8_returns_none(u8_val in 6u8..=255) {
        let result = WorktreeState::from_u8(u8_val);
        prop_assert!(result.is_none());
    }

    #[test]
    fn proptest_worktree_state_valid_next_states_only_valid_states(
        state in arb_worktree_state()
    ) {
        let next_states = state.valid_next_states();
        for next in next_states {
            prop_assert!(next.as_u8() <= 5);
        }
    }

    #[test]
    fn proptest_worktree_state_transition_consistency(
        from_state in arb_worktree_state(),
        to_state in arb_worktree_state()
    ) {
        let can_transition = from_state.can_transition_to(to_state);
        let next_states = from_state.valid_next_states();
        let is_next = next_states.contains(&to_state);
        prop_assert_eq!(can_transition, is_next);
    }

    #[test]
    fn proptest_worktree_state_transitions_valid(
        state in arb_worktree_state()
    ) {
        let next_states = state.valid_next_states();
        if state != WorktreeState::Removed {
            prop_assert!(!next_states.is_empty());
        } else {
            prop_assert!(next_states.is_empty());
        }
    }

    #[test]
    fn proptest_worktree_type_enum_round_trip_preserves_value(
        type_enum in arb_worktree_type()
    ) {
        let round_trip = WorktreeTypeEnum::from_u8(type_enum.as_u8()).unwrap();
        prop_assert_eq!(type_enum, round_trip);
    }

    #[test]
    fn proptest_worktree_type_all_types_valid(u8_val in 0u8..=4) {
        let result = WorktreeTypeEnum::from_u8(u8_val);
        prop_assert!(result.is_some());
    }

    #[test]
    fn proptest_worktree_type_invalid_u8_returns_none(u8_val in 5u8..=255) {
        let result = WorktreeTypeEnum::from_u8(u8_val);
        prop_assert!(result.is_none());
    }
}
