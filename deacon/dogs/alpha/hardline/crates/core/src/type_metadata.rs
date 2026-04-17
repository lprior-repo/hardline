//! Validated metadata storage

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidatedMetadata {
    data: std::collections::HashMap<String, String>,
}

impl ValidatedMetadata {
    pub fn empty() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn test_empty() {
        let meta = ValidatedMetadata::empty();
        assert_eq!(meta.get("anything"), None);
    }

    #[test]
    fn test_default_is_empty() {
        let meta = ValidatedMetadata::default();
        assert_eq!(meta.get("key"), None);
    }

    // ── Insert and Get ───────────────────────────────────────────────────────

    #[test]
    fn test_insert_and_get() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key", "value");
        assert_eq!(meta.get("key"), Some("value"));
    }

    #[test]
    fn test_insert_overwrites() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key", "first");
        meta.insert("key", "second");
        assert_eq!(meta.get("key"), Some("second"));
    }

    #[test]
    fn test_multiple_keys() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("a", "1");
        meta.insert("b", "2");
        meta.insert("c", "3");
        assert_eq!(meta.get("a"), Some("1"));
        assert_eq!(meta.get("b"), Some("2"));
        assert_eq!(meta.get("c"), Some("3"));
    }

    #[test]
    fn test_get_missing_key() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("exists", "yes");
        assert_eq!(meta.get("missing"), None);
    }

    #[test]
    fn test_insert_with_string_types() {
        let mut meta = ValidatedMetadata::empty();

        // Insert with &str
        meta.insert("str_key", "str_value");
        assert_eq!(meta.get("str_key"), Some("str_value"));

        // Insert with String
        meta.insert(String::from("owned_key"), String::from("owned_value"));
        assert_eq!(meta.get("owned_key"), Some("owned_value"));
    }

    #[test]
    fn test_get_empty_string_value() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key", "");
        assert_eq!(meta.get("key"), Some(""));
    }

    #[test]
    fn test_get_key_with_special_chars() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key-with-dashes", "value");
        meta.insert("key_with_underscores", "value2");
        meta.insert("key.with.dots", "value3");
        assert_eq!(meta.get("key-with-dashes"), Some("value"));
        assert_eq!(meta.get("key_with_underscores"), Some("value2"));
        assert_eq!(meta.get("key.with.dots"), Some("value3"));
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_serde_roundtrip_empty() {
        let meta = ValidatedMetadata::empty();
        let json = serde_json::to_string(&meta).expect("serialize ok");
        let deserialized: ValidatedMetadata = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.get("anything"), None);
    }

    #[test]
    fn test_serde_roundtrip_with_data() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("author", "test-agent");
        meta.insert("version", "1.0.0");

        let json = serde_json::to_string(&meta).expect("serialize ok");
        let deserialized: ValidatedMetadata = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.get("author"), Some("test-agent"));
        assert_eq!(deserialized.get("version"), Some("1.0.0"));
    }

    // ── Clone ────────────────────────────────────────────────────────────────

    #[test]
    fn test_clone_independent() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key", "original");

        let cloned = meta.clone();
        // Mutate original
        meta.insert("key", "changed");

        // Clone should still have original
        assert_eq!(cloned.get("key"), Some("original"));
        assert_eq!(meta.get("key"), Some("changed"));
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn test_debug_empty() {
        let meta = ValidatedMetadata::empty();
        let debug = format!("{meta:?}");
        assert!(debug.contains("ValidatedMetadata"));
    }

    #[test]
    fn test_debug_with_data() {
        let mut meta = ValidatedMetadata::empty();
        meta.insert("key", "value");
        let debug = format!("{meta:?}");
        assert!(debug.contains("key"));
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_insert_and_get_roundtrip(key in "[a-zA-Z0-9_-]{1,50}", val in "[a-zA-Z0-9_-]{0,100}") {
            let mut meta = ValidatedMetadata::empty();
            meta.insert(&key, &val);
            assert_eq!(meta.get(&key), Some(val.as_str()));
        }

        #[test]
        fn prop_missing_key_always_none(key in "[a-zA-Z0-9_-]{1,50}") {
            let meta = ValidatedMetadata::empty();
            assert_eq!(meta.get(&key), None);
        }

        #[test]
        fn prop_overwrite_always_wins(
            key in "[a-zA-Z0-9_-]{1,20}",
            v1 in "[a-zA-Z0-9]{1,20}",
            v2 in "[a-zA-Z0-9]{1,20}"
        ) {
            let mut meta = ValidatedMetadata::empty();
            meta.insert(&key, &v1);
            meta.insert(&key, &v2);
            assert_eq!(meta.get(&key), Some(v2.as_str()));
        }
    }
}
