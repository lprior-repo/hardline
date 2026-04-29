use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(pub Vec<String>);

impl Labels {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(mut self, label: impl Into<String>) -> Self {
        self.0.push(label.into());
        self
    }

    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use proptest::proptest;

    use super::*;

    #[test]
    fn new_is_empty() {
        let labels = Labels::new();
        assert!(labels.as_slice().is_empty());
    }

    #[test]
    fn default_is_empty() {
        let labels = Labels::default();
        assert!(labels.as_slice().is_empty());
    }

    #[test]
    fn with_adds_label() {
        let labels = Labels::new().with("bug").with("urgent");
        assert_eq!(
            labels.as_slice(),
            &["bug".to_string(), "urgent".to_string()]
        );
    }

    #[test]
    fn contains_returns_true_for_existing() {
        let labels = Labels::new().with("rust");
        assert!(labels.contains("rust"));
    }

    #[test]
    fn contains_returns_false_for_missing() {
        let labels = Labels::new().with("rust");
        assert!(!labels.contains("go"));
    }

    #[test]
    fn contains_returns_false_for_empty() {
        let labels = Labels::new();
        assert!(!labels.contains("anything"));
    }

    #[test]
    fn equality_works() {
        let a = Labels::new().with("x").with("y");
        let b = Labels::new().with("x").with("y");
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_works() {
        let a = Labels::new().with("x");
        let b = Labels::new().with("y");
        assert_ne!(a, b);
    }

    #[test]
    fn serde_roundtrip() {
        let labels = Labels::new().with("a").with("b").with("c");
        let json = serde_json::to_string(&labels).unwrap();
        let parsed: Labels = serde_json::from_str(&json).unwrap();
        assert_eq!(labels, parsed);
    }

    #[test]
    fn serde_roundtrip_empty() {
        let labels = Labels::new();
        let json = serde_json::to_string(&labels).unwrap();
        let parsed: Labels = serde_json::from_str(&json).unwrap();
        assert_eq!(labels, parsed);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Labels::new().with("tag-1"));
        assert!(set.contains(&Labels::new().with("tag-1")));
        assert!(!set.contains(&Labels::new().with("tag-2")));
    }

    #[test]
    fn with_accepts_string_ref() {
        let labels = Labels::new().with("ref-label");
        assert!(labels.contains("ref-label"));
    }

    #[test]
    fn with_accepts_string_owned() {
        let label = String::from("owned-label");
        let labels = Labels::new().with(label);
        assert!(labels.contains("owned-label"));
    }

    #[test]
    fn as_slice_length_matches() {
        let labels = Labels::new().with("a").with("b").with("c");
        assert_eq!(labels.as_slice().len(), 3);
    }

    #[test]
    fn contains_empty_label() {
        let labels = Labels::new().with("");
        assert!(labels.contains(""));
    }

    #[test]
    fn with_duplicate_labels() {
        let labels = Labels::new().with("dup").with("dup");
        assert_eq!(labels.as_slice().len(), 2);
    }

    proptest! {
        #[test]
        fn labels_contain_added_string(ref s in ".{1,50}") {
            let labels = Labels::new().with(s.clone());
            assert!(labels.contains(s));
        }

        #[test]
        fn labels_length_matches_additions(ref parts in proptest::collection::vec(".{1,20}", 0..10)) {
            let mut labels = Labels::new();
            for p in parts {
                labels = labels.with(p);
            }
            assert_eq!(labels.as_slice().len(), parts.len());
        }

        #[test]
        fn labels_empty_when_created(ref s in ".{0}") {
            let labels = Labels::new();
            assert!(labels.as_slice().is_empty());
            assert!(!labels.contains(s));
        }
    }
}
