pub mod ci_history;

pub use ci_history::{CiCheckHistory, CiRunRecord, CiStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BranchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BranchName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BranchName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_name_new() {
        let name = BranchName::new("main");
        assert_eq!(name.as_str(), "main");
    }

    #[test]
    fn test_branch_name_display() {
        let name = BranchName::new("feature/test-branch");
        assert_eq!(format!("{name}"), "feature/test-branch");
    }

    #[test]
    fn test_branch_name_from_string() {
        let name = BranchName::from("develop".to_string());
        assert_eq!(name.as_str(), "develop");
    }

    #[test]
    fn test_branch_name_from_str() {
        let name = BranchName::from("release/1.0");
        assert_eq!(name.as_str(), "release/1.0");
    }

    #[test]
    fn test_branch_name_equality() {
        let a = BranchName::new("main");
        let b = BranchName::new("main");
        let c = BranchName::new("develop");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_branch_name_hashing() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BranchName::new("main"));
        assert!(set.contains(&BranchName::new("main")));
        assert!(!set.contains(&BranchName::new("develop")));
    }

    #[test]
    fn test_branch_name_ordering() {
        let a = BranchName::new("alpha");
        let b = BranchName::new("beta");
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn test_branch_name_empty_string() {
        let name = BranchName::new("");
        assert_eq!(name.as_str(), "");
        assert_eq!(format!("{name}"), "");
    }

    #[test]
    fn test_branch_name_unicode() {
        let name = BranchName::new("feature/日本語-branch");
        assert_eq!(name.as_str(), "feature/日本語-branch");
        assert_eq!(format!("{name}"), "feature/日本語-branch");
    }

    #[test]
    fn test_branch_name_long_string() {
        let long_name = "a".repeat(10_000);
        let name = BranchName::new(&long_name);
        assert_eq!(name.as_str(), long_name);
    }

    #[test]
    fn test_branch_name_serde_roundtrip_json() {
        let name = BranchName::new("feature/test-branch");
        let json = serde_json::to_string(&name).expect("serialize");
        let deserialized: BranchName = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(name, deserialized);
    }

    #[test]
    fn test_branch_name_serde_roundtrip_empty() {
        let name = BranchName::new("");
        let json = serde_json::to_string(&name).expect("serialize");
        let deserialized: BranchName = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(name, deserialized);
    }

    #[test]
    fn test_branch_name_partial_ord() {
        let names = vec![
            BranchName::new("zeta"),
            BranchName::new("alpha"),
            BranchName::new("beta"),
        ];
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(sorted[0], BranchName::new("alpha"));
        assert_eq!(sorted[1], BranchName::new("beta"));
        assert_eq!(sorted[2], BranchName::new("zeta"));
    }

    #[test]
    fn test_branch_name_ord_total() {
        let a = BranchName::new("same");
        let b = BranchName::new("same");
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
        assert!(a <= b);
        assert!(a >= b);
    }

    #[test]
    fn test_branch_name_clone() {
        let name = BranchName::new("main");
        let cloned = name.clone();
        assert_eq!(name, cloned);
        assert_eq!(name.as_str(), cloned.as_str());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_branch_name_roundtrip(s in ".{0,256}") {
            let name = BranchName::new(s.clone());
            let json = serde_json::to_string(&name).expect("serialize");
            let deserialized: BranchName = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(name, deserialized);
            assert_eq!(name.as_str(), s);
        }

        #[test]
        fn prop_branch_name_display_matches_as_str(s in ".{0,256}") {
            let name = BranchName::new(s.clone());
            assert_eq!(format!("{name}"), s);
            assert_eq!(name.as_str(), s);
        }

        #[test]
        fn prop_branch_name_equality_reflexive(s in ".{0,100}") {
            let a = BranchName::new(s);
            assert_eq!(a, a);
        }
    }
}
