use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub const fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    #[must_use]
    pub const fn from_value(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            _ => Self::P4,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p0_value_is_zero() {
        assert_eq!(Priority::P0.value(), 0);
    }

    #[test]
    fn p1_value_is_one() {
        assert_eq!(Priority::P1.value(), 1);
    }

    #[test]
    fn p2_value_is_two() {
        assert_eq!(Priority::P2.value(), 2);
    }

    #[test]
    fn p3_value_is_three() {
        assert_eq!(Priority::P3.value(), 3);
    }

    #[test]
    fn p4_value_is_four() {
        assert_eq!(Priority::P4.value(), 4);
    }

    #[test]
    fn from_value_maps_correctly() {
        assert_eq!(Priority::from_value(0), Priority::P0);
        assert_eq!(Priority::from_value(1), Priority::P1);
        assert_eq!(Priority::from_value(2), Priority::P2);
        assert_eq!(Priority::from_value(3), Priority::P3);
    }

    #[test]
    fn from_value_defaults_to_p4_for_unknown() {
        assert_eq!(Priority::from_value(99), Priority::P4);
        assert_eq!(Priority::from_value(255), Priority::P4);
    }

    #[test]
    fn ordering_is_correct() {
        assert!(Priority::P0 < Priority::P1);
        assert!(Priority::P1 < Priority::P2);
        assert!(Priority::P2 < Priority::P3);
        assert!(Priority::P3 < Priority::P4);
    }

    #[test]
    fn display_p0() {
        assert_eq!(format!("{}", Priority::P0), "P0");
    }

    #[test]
    fn display_p4() {
        assert_eq!(format!("{}", Priority::P4), "P4");
    }

    #[test]
    fn equality_works() {
        assert_eq!(Priority::P0, Priority::P0);
        assert_ne!(Priority::P0, Priority::P1);
    }

    #[test]
    fn serde_roundtrip() {
        for p in [
            Priority::P0,
            Priority::P1,
            Priority::P2,
            Priority::P3,
            Priority::P4,
        ] {
            let json = serde_json::to_string(&p).unwrap();
            let parsed: Priority = serde_json::from_str(&json).unwrap();
            assert_eq!(p, parsed);
        }
    }

    #[test]
    fn serde_serializes_lowercase() {
        let json = serde_json::to_string(&Priority::P0).unwrap();
        assert_eq!(json, "\"p0\"");
    }

    #[test]
    fn serde_deserializes_lowercase() {
        let parsed: Priority = serde_json::from_str("\"p3\"").unwrap();
        assert_eq!(parsed, Priority::P3);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Priority::P0);
        assert!(set.contains(&Priority::P0));
        assert!(!set.contains(&Priority::P1));
    }

    #[test]
    fn from_value_four_maps_to_p4() {
        assert_eq!(Priority::from_value(4), Priority::P4);
    }

    #[test]
    fn display_p1_through_p3() {
        assert_eq!(format!("{}", Priority::P1), "P1");
        assert_eq!(format!("{}", Priority::P2), "P2");
        assert_eq!(format!("{}", Priority::P3), "P3");
    }

    #[test]
    fn total_ordering() {
        assert!(Priority::P0 < Priority::P1);
        assert!(Priority::P1 < Priority::P2);
        assert!(Priority::P2 < Priority::P3);
        assert!(Priority::P3 < Priority::P4);
        assert!(Priority::P0 <= Priority::P0);
        assert!(Priority::P4 >= Priority::P3);
    }

    mod proptest_priority {
        use proptest::proptest;

        use super::*;

        proptest! {
            #[test]
            fn from_value_roundtrips(val in 0u8..=4) {
                let p = Priority::from_value(val);
                assert_eq!(p.value(), val);
            }

            #[test]
            fn values_greater_than_four_default_to_p4(val in 5u8..=255) {
                let p = Priority::from_value(val);
                assert_eq!(p, Priority::P4);
            }

            #[test]
            fn priority_ordering_is_total(a in 0u8..=4, b in 0u8..=4) {
                let pa = Priority::from_value(a);
                let pb = Priority::from_value(b);
                assert_eq!(pa < pb || pa == pb || pa > pb, true);
            }

            #[test]
            fn priority_ordering_consistent_with_value(a in 0u8..=4, b in 0u8..=4) {
                let pa = Priority::from_value(a);
                let pb = Priority::from_value(b);
                assert_eq!(pa.cmp(&pb), a.cmp(&b));
            }

            #[test]
            fn serde_roundtrip_any_priority(val in 0u8..=4) {
                let p = Priority::from_value(val);
                let json = serde_json::to_string(&p).unwrap();
                let parsed: Priority = serde_json::from_str(&json).unwrap();
                assert_eq!(p, parsed);
            }
        }
    }
}
