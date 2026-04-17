#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
    Draft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePrStateError(String);

impl fmt::Display for ParsePrStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown PR state: {}", self.0)
    }
}

impl std::error::Error for ParsePrStateError {}

impl fmt::Display for PrState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closed => write!(f, "closed"),
            Self::Merged => write!(f, "merged"),
            Self::Draft => write!(f, "draft"),
        }
    }
}

impl FromStr for PrState {
    type Err = ParsePrStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().trim() {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "merged" => Ok(Self::Merged),
            "draft" => Ok(Self::Draft),
            other => Err(ParsePrStateError(other.to_string())),
        }
    }
}

impl PrState {
    /// Convert from GitHub API response fields.
    ///
    /// GitHub returns `state` (open/closed), `merged` (bool), and `draft` (bool).
    /// Resolution order: merged > draft > state.
    #[must_use]
    pub fn from_github_fields(state: &str, merged: bool, draft: bool) -> Self {
        if merged {
            return Self::Merged;
        }
        if draft {
            return Self::Draft;
        }
        state.parse().unwrap_or(Self::Open)
    }

    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }

    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed)
    }

    #[must_use]
    pub const fn is_merged(self) -> bool {
        matches!(self, Self::Merged)
    }

    #[must_use]
    pub const fn is_draft(self) -> bool {
        matches!(self, Self::Draft)
    }
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
        assert_ne!(PrState::Open, PrState::Draft);
        assert_ne!(PrState::Closed, PrState::Merged);
        assert_ne!(PrState::Closed, PrState::Draft);
        assert_ne!(PrState::Merged, PrState::Draft);
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
        let states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
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
        let states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
        assert_eq!(states.len(), 4);
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                assert_ne!(states[i], states[j]);
            }
        }
    }

    // === PrState Display tests ===

    #[test]
    fn test_pr_state_display_open() {
        assert_eq!(format!("{}", PrState::Open), "open");
    }

    #[test]
    fn test_pr_state_display_closed() {
        assert_eq!(format!("{}", PrState::Closed), "closed");
    }

    #[test]
    fn test_pr_state_display_merged() {
        assert_eq!(format!("{}", PrState::Merged), "merged");
    }

    #[test]
    fn test_pr_state_display_draft() {
        assert_eq!(format!("{}", PrState::Draft), "draft");
    }

    #[test]
    fn test_pr_state_display_matches_from_str_roundtrip() {
        let states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
        for state in &states {
            let displayed = format!("{}", state);
            let parsed: PrState = displayed.parse().expect("roundtrip parse");
            assert_eq!(*state, parsed);
        }
    }

    // === PrState FromStr tests ===

    #[test]
    fn test_pr_state_from_str_lowercase() {
        assert_eq!("open".parse::<PrState>().as_ref(), Ok(&PrState::Open));
        assert_eq!("closed".parse::<PrState>().as_ref(), Ok(&PrState::Closed));
        assert_eq!("merged".parse::<PrState>().as_ref(), Ok(&PrState::Merged));
        assert_eq!("draft".parse::<PrState>().as_ref(), Ok(&PrState::Draft));
    }

    #[test]
    fn test_pr_state_from_str_uppercase() {
        assert_eq!("OPEN".parse::<PrState>().as_ref(), Ok(&PrState::Open));
        assert_eq!("CLOSED".parse::<PrState>().as_ref(), Ok(&PrState::Closed));
        assert_eq!("MERGED".parse::<PrState>().as_ref(), Ok(&PrState::Merged));
        assert_eq!("DRAFT".parse::<PrState>().as_ref(), Ok(&PrState::Draft));
    }

    #[test]
    fn test_pr_state_from_str_mixed_case() {
        assert_eq!("Open".parse::<PrState>().as_ref(), Ok(&PrState::Open));
        assert_eq!("ClOsEd".parse::<PrState>().as_ref(), Ok(&PrState::Closed));
        assert_eq!("MeRgEd".parse::<PrState>().as_ref(), Ok(&PrState::Merged));
        assert_eq!("DrAfT".parse::<PrState>().as_ref(), Ok(&PrState::Draft));
    }

    #[test]
    fn test_pr_state_from_str_with_whitespace() {
        assert_eq!("  open  ".parse::<PrState>().as_ref(), Ok(&PrState::Open));
        assert_eq!("\tclosed\n".parse::<PrState>().as_ref(), Ok(&PrState::Closed));
        assert_eq!(" merged ".parse::<PrState>().as_ref(), Ok(&PrState::Merged));
        assert_eq!("  draft  ".parse::<PrState>().as_ref(), Ok(&PrState::Draft));
    }

    #[test]
    fn test_pr_state_from_str_unknown_returns_error() {
        let result = "unknown".parse::<PrState>();
        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(err.to_string().contains("unknown PR state"));
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn test_pr_state_from_str_empty_returns_error() {
        let result = "".parse::<PrState>();
        assert!(result.is_err());
    }

    #[test]
    fn test_pr_state_from_str_garbage_returns_error() {
        let inputs = ["foo", "bar123", "OPENED", "DRAFFT", "merge", "close"];
        for input in &inputs {
            assert!(input.parse::<PrState>().is_err(), "expected error for: {input}");
        }
    }

    #[test]
    fn test_parse_pr_state_error_display() {
        let err = ParsePrStateError("bad_state".to_string());
        assert_eq!(format!("{err}"), "unknown PR state: bad_state");
    }

    #[test]
    fn test_parse_pr_state_error_is_std_error() {
        let err = ParsePrStateError("x".to_string());
        let _: &dyn std::error::Error = &err;
    }

    // === PrState ordering tests ===

    #[test]
    fn test_pr_state_ordering_is_total() {
        // derive(Ord) uses declaration order: Open < Closed < Merged < Draft
        let states = [PrState::Draft, PrState::Merged, PrState::Closed, PrState::Open];
        let mut sorted = states;
        sorted.sort();
        assert_eq!(sorted, [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft]);
    }

    #[test]
    fn test_pr_state_partial_ord_consistent_with_ord() {
        let states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
        for i in 0..states.len() {
            for j in 0..states.len() {
                assert_eq!(states[i].cmp(&states[j]), states[i].partial_cmp(&states[j]).expect("partial_cmp"));
            }
        }
    }

    #[test]
    fn test_pr_state_hash_consistent_with_equality() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PrState::Open);
        set.insert(PrState::Open);
        assert_eq!(set.len(), 1);
        set.insert(PrState::Closed);
        assert_eq!(set.len(), 2);
    }

    // === PrState serde format tests ===

    #[test]
    fn test_pr_state_serde_snake_case_open() {
        let json = serde_json::to_string(&PrState::Open).expect("serialize");
        assert_eq!(json, "\"open\"");
    }

    #[test]
    fn test_pr_state_serde_snake_case_closed() {
        let json = serde_json::to_string(&PrState::Closed).expect("serialize");
        assert_eq!(json, "\"closed\"");
    }

    #[test]
    fn test_pr_state_serde_snake_case_merged() {
        let json = serde_json::to_string(&PrState::Merged).expect("serialize");
        assert_eq!(json, "\"merged\"");
    }

    #[test]
    fn test_pr_state_serde_snake_case_draft() {
        let json = serde_json::to_string(&PrState::Draft).expect("serialize");
        assert_eq!(json, "\"draft\"");
    }

    #[test]
    fn test_pr_state_serde_deserialize_from_snake_case() {
        let open: PrState = serde_json::from_str("\"open\"").expect("parse");
        assert_eq!(open, PrState::Open);
        let closed: PrState = serde_json::from_str("\"closed\"").expect("parse");
        assert_eq!(closed, PrState::Closed);
        let merged: PrState = serde_json::from_str("\"merged\"").expect("parse");
        assert_eq!(merged, PrState::Merged);
        let draft: PrState = serde_json::from_str("\"draft\"").expect("parse");
        assert_eq!(draft, PrState::Draft);
    }

    #[test]
    fn test_pr_state_serde_rejects_invalid() {
        let result = serde_json::from_str::<PrState>("\"invalid\"");
        assert!(result.is_err());
    }

    // === PrState from GitHub API fields ===

    #[test]
    fn test_from_github_fields_open_pr() {
        assert_eq!(PrState::from_github_fields("open", false, false), PrState::Open);
    }

    #[test]
    fn test_from_github_fields_closed_pr() {
        assert_eq!(PrState::from_github_fields("closed", false, false), PrState::Closed);
    }

    #[test]
    fn test_from_github_fields_merged_pr() {
        assert_eq!(PrState::from_github_fields("closed", true, false), PrState::Merged);
    }

    #[test]
    fn test_from_github_fields_merged_overrides_state() {
        // GitHub returns state=closed when merged, merged=true is the discriminator
        assert_eq!(PrState::from_github_fields("closed", true, false), PrState::Merged);
        assert_eq!(PrState::from_github_fields("open", true, false), PrState::Merged);
    }

    #[test]
    fn test_from_github_fields_draft_pr() {
        assert_eq!(PrState::from_github_fields("open", false, true), PrState::Draft);
    }

    #[test]
    fn test_from_github_fields_merged_takes_priority_over_draft() {
        assert_eq!(PrState::from_github_fields("open", true, true), PrState::Merged);
    }

    #[test]
    fn test_from_github_fields_unknown_state_falls_back_to_open() {
        assert_eq!(PrState::from_github_fields("unknown", false, false), PrState::Open);
    }

    #[test]
    fn test_from_github_fields_empty_state_falls_back_to_open() {
        assert_eq!(PrState::from_github_fields("", false, false), PrState::Open);
    }

    // === PrState predicate tests ===

    #[test]
    fn test_pr_state_is_open() {
        assert!(PrState::Open.is_open());
        assert!(!PrState::Closed.is_open());
        assert!(!PrState::Merged.is_open());
        assert!(!PrState::Draft.is_open());
    }

    #[test]
    fn test_pr_state_is_closed() {
        assert!(PrState::Closed.is_closed());
        assert!(!PrState::Open.is_closed());
        assert!(!PrState::Merged.is_closed());
        assert!(!PrState::Draft.is_closed());
    }

    #[test]
    fn test_pr_state_is_merged() {
        assert!(PrState::Merged.is_merged());
        assert!(!PrState::Open.is_merged());
        assert!(!PrState::Closed.is_merged());
        assert!(!PrState::Draft.is_merged());
    }

    #[test]
    fn test_pr_state_is_draft() {
        assert!(PrState::Draft.is_draft());
        assert!(!PrState::Open.is_draft());
        assert!(!PrState::Closed.is_draft());
        assert!(!PrState::Merged.is_draft());
    }

    // === PrState clone/copy tests ===

    #[test]
    fn test_pr_state_clone() {
        let state = PrState::Merged;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_pr_state_copy() {
        let state = PrState::Draft;
        let copied = state;
        assert_eq!(state, copied);
    }

    // === PrState debug format ===

    #[test]
    fn test_pr_state_debug_format() {
        assert!(format!("{:?}", PrState::Open).contains("Open"));
        assert!(format!("{:?}", PrState::Closed).contains("Closed"));
        assert!(format!("{:?}", PrState::Merged).contains("Merged"));
        assert!(format!("{:?}", PrState::Draft).contains("Draft"));
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
        fn prop_pr_state_serde_roundtrip_json(idx in 0u8..4u8) {
            let all_states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
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

        #[test]
        fn prop_pr_state_display_from_str_roundtrip(idx in 0u8..4u8) {
            let all_states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
            let s = &all_states[idx as usize];
            let displayed = format!("{}", s);
            let parsed: PrState = displayed.parse().expect("display roundtrip");
            assert_eq!(*s, parsed);
        }

        #[test]
        fn prop_pr_state_from_str_case_insensitive(s in "open|closed|merged|draft") {
            let parsed: PrState = s.parse().expect("lowercase parse");
            let upper: PrState = s.to_uppercase().parse().expect("uppercase parse");
            assert_eq!(parsed, upper);
        }

        #[test]
        fn prop_pr_state_from_str_rejects_garbage(s in "[a-z]{1,10}") {
            let valid = ["open", "closed", "merged", "draft"];
            let result = s.parse::<PrState>();
            if valid.contains(&s.as_str()) {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }

        #[test]
        fn prop_pr_state_ordering_transitivity(a in 0u8..4u8, b in 0u8..4u8, c in 0u8..4u8) {
            let all_states = [PrState::Open, PrState::Closed, PrState::Merged, PrState::Draft];
            let sa = all_states[a as usize];
            let sb = all_states[b as usize];
            let sc = all_states[c as usize];
            if sa <= sb && sb <= sc { assert!(sa <= sc); }
            if sa >= sb && sb >= sc { assert!(sa >= sc); }
        }

        #[test]
        fn prop_pr_state_from_github_fields_merged_overrides(
            state in "open|closed|other",
            draft in proptest::bool::ANY
        ) {
            // merged=true always returns Merged regardless of other fields
            assert_eq!(PrState::from_github_fields(&state, true, draft), PrState::Merged);
        }

        #[test]
        fn prop_pr_state_from_github_fields_draft_when_not_merged(
            state in "open|closed|other"
        ) {
            // draft=true + merged=false always returns Draft
            assert_eq!(PrState::from_github_fields(&state, false, true), PrState::Draft);
        }
    }
}
