//! Bead type enumeration and Priority level.
//!
//! These define the classification and urgency of beads.

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

/// Bead type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum BeadType {
    Bug,
    Feature,
    #[default]
    Task,
    Epic,
    Chore,
}

impl BeadType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Task => "task",
            Self::Epic => "epic",
            Self::Chore => "chore",
        }
    }
}


/// Priority level (0-4)
///
/// - 0: Critical (security, data loss, broken builds)
/// - 1: High (major features, important bugs)
/// - 2: Medium (default, nice-to-have)
/// - 3: Low (polish, optimization)
/// - 4: Backlog (future ideas)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(priority: u8) -> Result<Self, SessionError> {
        if priority > 4 {
            return Err(SessionError::InvalidPriority(format!(
                "Priority must be 0-4, got {}",
                priority
            )));
        }
        Ok(Self(priority))
    }

    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(2) // Medium priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_type_default_is_task() {
        assert_eq!(BeadType::default(), BeadType::Task);
    }

    #[test]
    fn bead_type_as_str() {
        assert_eq!(BeadType::Bug.as_str(), "bug");
        assert_eq!(BeadType::Feature.as_str(), "feature");
        assert_eq!(BeadType::Task.as_str(), "task");
        assert_eq!(BeadType::Epic.as_str(), "epic");
        assert_eq!(BeadType::Chore.as_str(), "chore");
    }

    #[test]
    fn priority_valid() {
        let p = Priority::new(2).unwrap();
        assert_eq!(p.as_u8(), 2);
    }

    #[test]
    fn priority_default_is_medium() {
        assert_eq!(Priority::default().as_u8(), 2);
    }

    #[test]
    fn priority_out_of_range_fails() {
        assert!(Priority::new(5).is_err());
        assert!(Priority::new(255).is_err());
    }

    // =========================================================================
    // BeadType Serialization Tests
    // =========================================================================

    #[test]
    fn bead_type_serde_roundtrip_all_variants() {
        let types = [BeadType::Bug, BeadType::Feature, BeadType::Task, BeadType::Epic, BeadType::Chore];
        for bt in types {
            let json = serde_json::to_string(&bt).expect("serialize");
            let parsed: BeadType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(bt, parsed, "Roundtrip failed for {:?}", bt);
        }
    }

    #[test]
    fn bead_type_serde_json_output() {
        assert_eq!(
            serde_json::to_string(&BeadType::Bug).expect("serialize"),
            "\"Bug\""
        );
        assert_eq!(
            serde_json::to_string(&BeadType::Task).expect("serialize"),
            "\"Task\""
        );
    }

    // =========================================================================
    // Priority Extended Tests
    // =========================================================================

    #[test]
    fn priority_boundary_zero() {
        let p = Priority::new(0).expect("zero is valid");
        assert_eq!(p.as_u8(), 0);
    }

    #[test]
    fn priority_boundary_four() {
        let p = Priority::new(4).expect("four is valid");
        assert_eq!(p.as_u8(), 4);
    }

    #[test]
    fn priority_serde_roundtrip() {
        let p = Priority::new(3).expect("valid");
        let json = serde_json::to_string(&p).expect("serialize");
        let parsed: Priority = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, parsed);
    }

    #[test]
    fn priority_equality() {
        let p1 = Priority::new(1).expect("valid");
        let p2 = Priority::new(1).expect("valid");
        let p3 = Priority::new(3).expect("valid");
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    // =========================================================================
    // BeadType and Priority Proptests
    // =========================================================================

    mod bead_types_proptests {
        use super::*;
        use proptest::proptest;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            /// Priority valid range is 0..=4
            #[test]
            fn prop_priority_valid_range(p in 0u8..=4u8) {
                let result = Priority::new(p);
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap().as_u8(), p);
            }

            /// Priority out of range rejects
            #[test]
            fn prop_priority_invalid_range(p in 5u8..=255u8) {
                let result = Priority::new(p);
                prop_assert!(result.is_err());
            }

            /// BeadType as_str returns lowercase ascii
            #[test]
            fn prop_bead_type_as_str_lowercase(bug_type_idx in 0u8..5u8) {
                let types = [BeadType::Bug, BeadType::Feature, BeadType::Task, BeadType::Epic, BeadType::Chore];
                let bt = types[bug_type_idx as usize];
                let s = bt.as_str();
                prop_assert!(!s.is_empty());
                prop_assert!(s.chars().all(|c| c.is_ascii_lowercase()));
            }

            /// BeadType serde roundtrip
            #[test]
            fn prop_bead_type_serde_roundtrip(type_idx in 0u8..5u8) {
                let types = [BeadType::Bug, BeadType::Feature, BeadType::Task, BeadType::Epic, BeadType::Chore];
                let bt = types[type_idx as usize];
                let json = serde_json::to_string(&bt).unwrap();
                let parsed: BeadType = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(bt, parsed);
            }

            /// Priority serde roundtrip
            #[test]
            fn prop_priority_serde_roundtrip(p in 0u8..=4u8) {
                let priority = Priority::new(p).unwrap();
                let json = serde_json::to_string(&priority).unwrap();
                let parsed: Priority = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(priority, parsed);
            }
        }
    }
}
