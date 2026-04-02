#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackState {
    Draft,
    Published,
    Merging,
    Merged,
    Conflict,
    Failed,
}

impl StackState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    Open,
    Draft,
    Approved,
    Merging,
    Merged,
    Closed,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_state_is_terminal() {
        assert!(StackState::Merged.is_terminal());
        assert!(!StackState::Draft.is_terminal());
        assert!(!StackState::Published.is_terminal());
        assert!(!StackState::Merging.is_terminal());
        assert!(!StackState::Conflict.is_terminal());
        assert!(!StackState::Failed.is_terminal());
    }

    #[test]
    fn test_stack_state_equality() {
        assert_eq!(StackState::Draft, StackState::Draft);
        assert_ne!(StackState::Draft, StackState::Published);
    }

    #[test]
    fn test_branch_state_variants() {
        let states = [
            BranchState::Open,
            BranchState::Draft,
            BranchState::Approved,
            BranchState::Merging,
            BranchState::Merged,
            BranchState::Closed,
            BranchState::Conflict,
        ];
        // Verify all variants are distinct
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                assert_ne!(states[i], states[j]);
            }
        }
    }

    #[test]
    fn test_pr_state_variants() {
        assert_eq!(PrState::Open, PrState::Open);
        assert_ne!(PrState::Open, PrState::Closed);
        assert_ne!(PrState::Open, PrState::Merged);
        assert_ne!(PrState::Closed, PrState::Merged);
    }

    #[test]
    fn test_stack_state_serde_roundtrip() {
        let states = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Merged,
            StackState::Conflict,
            StackState::Failed,
        ];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: StackState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*state, deserialized);
        }
    }

    #[test]
    fn test_stack_state_serde_snake_case() {
        let json = serde_json::to_string(&StackState::Merging).expect("serialize");
        assert_eq!(json, "\"merging\"");
        let deserialized: StackState = serde_json::from_str("\"merging\"").expect("deserialize");
        assert_eq!(deserialized, StackState::Merging);
    }

    #[test]
    fn test_branch_state_serde_roundtrip() {
        let states = [
            BranchState::Open,
            BranchState::Draft,
            BranchState::Approved,
            BranchState::Merging,
            BranchState::Merged,
            BranchState::Closed,
            BranchState::Conflict,
        ];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*state, deserialized);
        }
    }

    #[test]
    fn test_pr_state_serde_roundtrip() {
        let states = [PrState::Open, PrState::Closed, PrState::Merged];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: PrState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*state, deserialized);
        }
    }

    #[test]
    fn test_stack_state_debug_format() {
        let state = StackState::Merged;
        let debug = format!("{state:?}");
        assert!(debug.contains("Merged"));
    }

    #[test]
    fn test_branch_state_debug_format() {
        let state = BranchState::Open;
        let debug = format!("{state:?}");
        assert!(debug.contains("Open"));
    }

    #[test]
    fn test_stack_state_clone() {
        let state = StackState::Conflict;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_stack_state_copy() {
        let state = StackState::Draft;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn test_all_branch_states_are_distinct() {
        let states = [
            BranchState::Open,
            BranchState::Draft,
            BranchState::Approved,
            BranchState::Merging,
            BranchState::Merged,
            BranchState::Closed,
            BranchState::Conflict,
        ];
        assert_eq!(states.len(), 7);
    }

    #[test]
    fn test_all_stack_states_are_distinct() {
        let states = [
            StackState::Draft,
            StackState::Published,
            StackState::Merging,
            StackState::Merged,
            StackState::Conflict,
            StackState::Failed,
        ];
        assert_eq!(states.len(), 6);
    }

    #[test]
    fn test_all_pr_states_are_distinct() {
        let states = [PrState::Open, PrState::Closed, PrState::Merged];
        assert_eq!(states.len(), 3);
    }
}

#[cfg(test)]
mod proptests {
    use super::BranchState;
    use super::PrState;
    use super::StackState;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_stack_state_serde_roundtrip_json(idx in 0u8..6u8) {
            let all_states = [
                StackState::Draft,
                StackState::Published,
                StackState::Merging,
                StackState::Merged,
                StackState::Conflict,
                StackState::Failed,
            ];
            let s = &all_states[idx as usize];
            let json = serde_json::to_string(s).expect("serialize");
            let deserialized: StackState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, deserialized);
        }

        #[test]
        fn prop_pr_state_serde_roundtrip_json(idx in 0u8..3u8) {
            let all_states = [PrState::Open, PrState::Closed, PrState::Merged];
            let s = &all_states[idx as usize];
            let json = serde_json::to_string(s).expect("serialize");
            let deserialized: PrState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, deserialized);
        }

        #[test]
        fn prop_branch_state_serde_roundtrip_json(idx in 0u8..7u8) {
            let all_states = [
                BranchState::Open,
                BranchState::Draft,
                BranchState::Approved,
                BranchState::Merging,
                BranchState::Merged,
                BranchState::Closed,
                BranchState::Conflict,
            ];
            let s = &all_states[idx as usize];
            let json = serde_json::to_string(s).expect("serialize");
            let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, deserialized);
        }
    }
}
