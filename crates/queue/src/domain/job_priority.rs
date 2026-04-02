#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(value: u8) -> Result<Self, super::job_id::JobCreationError> {
        Ok(Self(value))
    }

    #[must_use]
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_valid() {
        let p = Priority::new(100);
        assert!(p.is_ok());
        assert_eq!(p.unwrap().value(), 100);
    }

    #[test]
    fn priority_max_value() {
        let p = Priority::new(255);
        assert!(p.is_ok());
    }

    #[test]
    fn priority_zero_value() {
        let p = Priority::new(0);
        assert!(p.is_ok());
        assert_eq!(p.unwrap().value(), 0);
    }

    #[test]
    fn priority_display() {
        let p = Priority::new(42).unwrap();
        assert_eq!(format!("{p}"), "42");
    }

    #[test]
    fn priority_clone_and_eq() {
        let a = Priority::new(100).unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn priority_copy() {
        let a = Priority::new(50).unwrap();
        let b = a;
        assert_eq!(a.value(), b.value());
    }

    #[test]
    fn priority_serde_roundtrip() {
        let p = Priority::new(200).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 200);
    }

    #[test]
    fn priority_serde_roundtrip_json_value() {
        let p = Priority::new(150).unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json, 150);
    }

    // --- Boundary values ---

    #[test]
    fn priority_boundary_values_all_u8() {
        for val in [0_u8, 1, 127, 128, 254, 255] {
            let p = Priority::new(val);
            assert!(p.is_ok(), "Priority::new({val}) should succeed");
            assert_eq!(p.unwrap().value(), val);
        }
    }

    // --- Display edge cases ---

    #[test]
    fn priority_display_zero() {
        let p = Priority::new(0).unwrap();
        assert_eq!(format!("{p}"), "0");
    }

    #[test]
    fn priority_display_max() {
        let p = Priority::new(255).unwrap();
        assert_eq!(format!("{p}"), "255");
    }

    // --- Debug ---

    #[test]
    fn priority_debug() {
        let p = Priority::new(42).unwrap();
        let debug = format!("{p:?}");
        assert!(debug.contains("Priority"));
    }

    // --- Hash (skip - Priority in job_priority does not implement Hash) ---

    #[test]
    fn priority_hash_not_required() {
        // Priority in this module is used for comparison, not hashing.
        // The value_objects::Priority implements Hash instead.
        let a = Priority::new(10).unwrap();
        let b = Priority::new(10).unwrap();
        assert_eq!(a, b);
    }

    // --- Eq properties ---

    #[test]
    fn priority_eq_reflexive() {
        let p = Priority::new(42).unwrap();
        assert_eq!(p, p);
    }

    #[test]
    fn priority_eq_symmetric() {
        let a = Priority::new(42).unwrap();
        let b = Priority::new(42).unwrap();
        assert_eq!(a, b);
        assert_eq!(b, a);
    }

    #[test]
    fn priority_ne_different_values() {
        let a = Priority::new(10).unwrap();
        let b = Priority::new(20).unwrap();
        assert_ne!(a, b);
    }

    // --- Serde roundtrip for boundary values ---

    #[test]
    fn priority_serde_roundtrip_zero() {
        let p = Priority::new(0).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 0);
    }

    #[test]
    fn priority_serde_roundtrip_max() {
        let p = Priority::new(255).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 255);
    }

    // --- Serde roundtrip via json Value ---

    #[test]
    fn priority_serde_roundtrip_zero_json_value() {
        let p = Priority::new(0).unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json, 0);
    }

    #[test]
    fn priority_serde_roundtrip_max_json_value() {
        let p = Priority::new(255).unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json, 255);
    }

    // --- Copy semantics verification ---

    #[test]
    fn priority_copy_still_usable_after_move() {
        let a = Priority::new(99).unwrap();
        let _b = a;
        assert_eq!(a.value(), 99, "a should still be usable after copy");
    }

    // --- new always returns Ok ---

    #[test]
    fn priority_new_always_ok_for_all_u8() {
        for val in 0_u8..=255 {
            assert!(Priority::new(val).is_ok(), "Priority::new({val}) should be Ok");
        }
    }

    // --- Proptests ---

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn proptest_priority_new_always_succeeds(val in 0u8..=255) {
            let p = Priority::new(val);
            prop_assert!(p.is_ok());
            prop_assert_eq!(p.unwrap().value(), val);
        }

        #[test]
        fn proptest_priority_serde_roundtrip(val in 0u8..=255) {
            let p = Priority::new(val).unwrap();
            let json = serde_json::to_string(&p).unwrap();
            let back: Priority = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back.value(), val);
        }

        #[test]
        fn proptest_priority_eq_same_value(val in 0u8..=255) {
            let a = Priority::new(val).unwrap();
            let b = Priority::new(val).unwrap();
            prop_assert_eq!(a, b);
        }

        #[test]
        fn proptest_priority_display_matches_value(val in 0u8..=255) {
            let p = Priority::new(val).unwrap();
            prop_assert_eq!(format!("{p}"), format!("{val}"));
        }
    }
}
