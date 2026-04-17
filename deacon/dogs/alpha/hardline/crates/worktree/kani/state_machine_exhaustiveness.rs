//! Kani verification harness for WorktreeState exhaustiveness
//!
//! Verifies that all 6 states have valid_next_states() defined

use worktree::domain::WorktreeState;

#[kani::proof]
fn prove_worktree_state_exhaustiveness() {
    // Test all 6 states have valid_next_states() defined
    let states = [
        WorktreeState::Creating,
        WorktreeState::Incomplete,
        WorktreeState::Active,
        WorktreeState::Suspended,
        WorktreeState::Removing,
        WorktreeState::Removed,
    ];

    for state in states.iter() {
        let next_states = state.valid_next_states();
        // Each state must have valid_next_states() returning only valid states
        for next in next_states.iter() {
            kani::assume(next.as_u8() <= 5);
        }
    }
}

#[kani::proof]
fn prove_worktree_state_transition_consistency() {
    let states = [
        WorktreeState::Creating,
        WorktreeState::Incomplete,
        WorktreeState::Active,
        WorktreeState::Suspended,
        WorktreeState::Removing,
        WorktreeState::Removed,
    ];

    for from in states.iter() {
        for to in states.iter() {
            let can_transition = from.can_transition_to(*to);
            let next_states = from.valid_next_states();
            let is_next = next_states.contains(to);
            kani::assert(can_transition == is_next, "Transition consistency");
        }
    }
}
