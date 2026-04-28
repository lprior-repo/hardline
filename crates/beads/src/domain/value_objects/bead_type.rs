use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_exist() {
        let _ = BeadType::Bug;
        let _ = BeadType::Feature;
        let _ = BeadType::Task;
        let _ = BeadType::Epic;
        let _ = BeadType::Chore;
    }

    #[test]
    fn display_bug() {
        assert_eq!(format!("{}", BeadType::Bug), "bug");
    }

    #[test]
    fn display_feature() {
        assert_eq!(format!("{}", BeadType::Feature), "feature");
    }

    #[test]
    fn display_task() {
        assert_eq!(format!("{}", BeadType::Task), "task");
    }

    #[test]
    fn display_epic() {
        assert_eq!(format!("{}", BeadType::Epic), "epic");
    }

    #[test]
    fn display_chore() {
        assert_eq!(format!("{}", BeadType::Chore), "chore");
    }

    #[test]
    fn from_str_parses_all_variants() {
        assert_eq!("bug".parse::<BeadType>().unwrap(), BeadType::Bug);
        assert_eq!("feature".parse::<BeadType>().unwrap(), BeadType::Feature);
        assert_eq!("task".parse::<BeadType>().unwrap(), BeadType::Task);
        assert_eq!("epic".parse::<BeadType>().unwrap(), BeadType::Epic);
        assert_eq!("chore".parse::<BeadType>().unwrap(), BeadType::Chore);
    }

    #[test]
    fn from_str_rejects_invalid() {
        let result: std::result::Result<BeadType, _> = "nonexistent".parse();
        assert!(result.is_err());
    }

    #[test]
    fn serde_roundtrip() {
        for bt in [
            BeadType::Bug,
            BeadType::Feature,
            BeadType::Task,
            BeadType::Epic,
            BeadType::Chore,
        ] {
            let json = serde_json::to_string(&bt).unwrap();
            let parsed: BeadType = serde_json::from_str(&json).unwrap();
            assert_eq!(bt, parsed);
        }
    }

    #[test]
    fn serde_serializes_lowercase() {
        let json = serde_json::to_string(&BeadType::Feature).unwrap();
        assert_eq!(json, "\"feature\"");
    }

    #[test]
    fn equality_works() {
        assert_eq!(BeadType::Bug, BeadType::Bug);
        assert_ne!(BeadType::Bug, BeadType::Feature);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BeadType::Bug);
        assert!(set.contains(&BeadType::Bug));
        assert!(!set.contains(&BeadType::Feature));
    }

    #[test]
    fn debug_format() {
        let debug = format!("{:?}", BeadType::Feature);
        assert!(debug.contains("Feature"));
    }

    #[test]
    fn all_five_variants_are_distinct() {
        let variants = [
            BeadType::Bug,
            BeadType::Feature,
            BeadType::Task,
            BeadType::Epic,
            BeadType::Chore,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    mod proptest_bead_type {
        use proptest::proptest;

        use super::*;

        proptest! {
            #[test]
            fn serde_roundtrip_any_variant(seed in 0u8..=4) {
                let bt = match seed {
                    0 => BeadType::Bug,
                    1 => BeadType::Feature,
                    2 => BeadType::Task,
                    3 => BeadType::Epic,
                    _ => BeadType::Chore,
                };
                let json = serde_json::to_string(&bt).unwrap();
                let parsed: BeadType = serde_json::from_str(&json).unwrap();
                assert_eq!(bt, parsed);
            }
        }
    }
}
