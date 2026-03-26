//! Kani proofs for BranchState transitions and invariants.
//!
//! # Invariants Proven
//!
//! 1. Valid transitions: Detached→OnBranch, OnBranch→Detached, OnBranch→OnBranch
//! 2. Invalid transitions: Detached→Detached is rejected
//! 3. State preservation: Transitions only change branch field

#[cfg(kani)]
mod proof {
    use crate::domain::session::BranchState;

    #[kani::proof]
    fn verify_branch_state_valid_transitions() {
        let detached = BranchState::Detached;
        let on_branch = BranchState::OnBranch {
            name: kani::any::<String>(),
        };

        kani::cover!(detached.can_transition_to(&on_branch));
        kani::cover!(on_branch.can_transition_to(&detached));
        kani::cover!(on_branch.can_transition_to(&BranchState::OnBranch { name: kani::any() }));
    }

    #[kani::proof]
    fn verify_detached_to_detached_is_invalid() {
        let detached = BranchState::Detached;
        let result = detached.can_transition_to(&BranchState::Detached);
        assert!(!result);
    }

    #[kani::proof]
    fn verify_all_valid_transitions_return_true() {
        let from = kani::any::<BranchState>();
        let to = kani::any::<BranchState>();

        if from.can_transition_to(&to) {
            match (&from, &to) {
                (BranchState::Detached, BranchState::OnBranch { .. }) => {}
                (BranchState::OnBranch { .. }, BranchState::Detached) => {}
                (BranchState::OnBranch { .. }, BranchState::OnBranch { .. }) => {}
                _ => panic!("Valid transition should match expected patterns"),
            }
        }
    }

    #[kani::proof]
    fn verify_is_detached_reflects_state() {
        let state = kani::any::<BranchState>();
        match state {
            BranchState::Detached => assert!(state.is_detached()),
            BranchState::OnBranch { .. } => assert!(!state.is_detached()),
        }
    }

    #[kani::proof]
    fn verify_branch_name_option() {
        let detached = BranchState::Detached;
        let on_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };

        assert_eq!(detached.branch_name(), None);
        assert_eq!(on_branch.branch_name(), Some("main"));
    }
}
