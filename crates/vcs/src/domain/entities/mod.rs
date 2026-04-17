//! VCS Domain Entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

impl Commit {
    pub fn new(
        id: String,
        message: String,
        author: String,
        timestamp: DateTime<Utc>,
        parents: Vec<String>,
    ) -> Self {
        Self {
            id,
            message,
            author,
            timestamp,
            parents,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}

impl Branch {
    pub fn new(name: String, is_current: bool, tracking: Option<String>) -> Self {
        Self {
            name,
            is_current,
            tracking,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}

impl Workspace {
    pub fn new(name: String, branch: String, is_current: bool) -> Self {
        Self {
            name,
            branch,
            is_current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // -- Commit tests --

    #[test]
    fn commit_new_with_all_fields() {
        let now = Utc::now();
        let commit = Commit::new(
            "abc123".to_string(),
            "Initial commit".to_string(),
            "Alice <alice@example.com>".to_string(),
            now,
            vec!["parent1".to_string()],
        );
        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.author, "Alice <alice@example.com>");
        assert_eq!(commit.timestamp, now);
        assert_eq!(commit.parents, vec!["parent1"]);
    }

    #[test]
    fn commit_new_with_empty_parents() {
        let commit = Commit::new(
            "root".to_string(),
            "root commit".to_string(),
            "Bob".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.parents.is_empty());
    }

    #[test]
    fn commit_new_with_multiple_parents() {
        let commit = Commit::new(
            "merge".to_string(),
            "merge commit".to_string(),
            "Bob".to_string(),
            Utc::now(),
            vec!["p1".to_string(), "p2".to_string()],
        );
        assert_eq!(commit.parents.len(), 2);
    }

    #[test]
    fn commit_clone() {
        let commit = Commit::new(
            "id".to_string(),
            "msg".to_string(),
            "a".to_string(),
            Utc::now(),
            vec![],
        );
        let cloned = commit.clone();
        assert_eq!(commit.id, cloned.id);
        assert_eq!(commit.message, cloned.message);
    }

    #[test]
    fn commit_serde_roundtrip() {
        let commit = Commit::new(
            "sha123".to_string(),
            "test commit".to_string(),
            "Test <test@test.com>".to_string(),
            Utc::now(),
            vec!["parent".to_string()],
        );
        let json = serde_json::to_string(&commit).expect("serialize");
        let deserialized: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(commit.id, deserialized.id);
        assert_eq!(commit.message, deserialized.message);
        assert_eq!(commit.author, deserialized.author);
        assert_eq!(commit.parents, deserialized.parents);
    }

    // ========================================================================
    // Commit exhaustive tests — ha-9en3
    // ========================================================================

    // ── SHA format acceptance (no enforcement, any string accepted) ─────────

    #[test]
    fn commit_sha_40_char_lowercase_hex() {
        let sha = "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b";
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
        let commit = Commit::new(
            sha.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, sha);
    }

    #[test]
    fn commit_sha_40_char_uppercase_hex() {
        let sha = "E7C805D7BA5E7A8B5E4C3D2F1A0B9C8D7E6F5A4B";
        assert_eq!(sha.len(), 40);
        let commit = Commit::new(
            sha.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, sha);
    }

    #[test]
    fn commit_sha_mixed_case_hex() {
        let sha = "DeAdBeEfDeAdBeEfDeAdBeEfDeAdBeEfDeAdBeEf";
        let commit = Commit::new(
            sha.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, sha);
    }

    #[test]
    fn commit_sha_short_7_char() {
        let short = "e7c805d";
        let commit = Commit::new(
            short.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, "e7c805d");
        assert_eq!(commit.id.len(), 7);
    }

    #[test]
    fn commit_sha_short_8_char() {
        let short = "e7c805d7";
        let commit = Commit::new(
            short.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id.len(), 8);
    }

    #[test]
    fn commit_sha_non_hex_tag_name() {
        let commit = Commit::new(
            "v1.0.0".to_string(),
            "tag".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, "v1.0.0");
    }

    #[test]
    fn commit_sha_empty_string_accepted() {
        // Commit::new does not validate — empty id is accepted
        let commit = Commit::new(
            String::new(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.id.is_empty());
    }

    #[test]
    fn commit_sha_with_special_chars() {
        let id = "refs/heads/feature/branch@{1}";
        let commit = Commit::new(
            id.to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, id);
    }

    #[test]
    fn commit_sha_very_long_string() {
        let long_id = "a".repeat(10_000);
        let commit = Commit::new(
            long_id.clone(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.id, long_id);
        assert_eq!(commit.id.len(), 10_000);
    }

    // ── Parent tracking — exhaustive edge cases ────────────────────────────

    #[test]
    fn commit_parents_order_preserved() {
        let parents = vec![
            "1111111111111111111111111111111111111111".to_string(),
            "2222222222222222222222222222222222222222".to_string(),
            "3333333333333333333333333333333333333333".to_string(),
        ];
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            parents.clone(),
        );
        assert_eq!(commit.parents[0], "1111111111111111111111111111111111111111");
        assert_eq!(commit.parents[1], "2222222222222222222222222222222222222222");
        assert_eq!(commit.parents[2], "3333333333333333333333333333333333333333");
    }

    #[test]
    fn commit_parents_duplicates_allowed() {
        let parents = vec![
            "same111111111111111111111111111111111111".to_string(),
            "same111111111111111111111111111111111111".to_string(),
        ];
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            parents,
        );
        assert_eq!(commit.parents[0], commit.parents[1]);
        assert_eq!(commit.parents.len(), 2);
    }

    #[test]
    fn commit_parents_many_10() {
        let parents: Vec<String> = (0..10).map(|i| format!("{i:040x}")).collect();
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            parents.clone(),
        );
        assert_eq!(commit.parents.len(), 10);
        assert_eq!(commit.parents, parents);
    }

    #[test]
    fn commit_parents_octopus_merge_3() {
        let parents = vec![
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            "cccccccccccccccccccccccccccccccccccccccc".to_string(),
        ];
        let commit = Commit::new(
            "merge".to_string(),
            "Octopus merge".to_string(),
            "test".to_string(),
            Utc::now(),
            parents.clone(),
        );
        assert_eq!(commit.parents.len(), 3);
        assert_eq!(commit.parents, parents);
    }

    #[test]
    fn commit_parents_mixed_formats() {
        // Parents can be short SHAs, full SHAs, or even non-SHA strings
        let parents = vec![
            "abc1234".to_string(),
            "ffffffffffffffffffffffffffffffffffffffff".to_string(),
            "HEAD".to_string(),
            "v2.0".to_string(),
        ];
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            parents.clone(),
        );
        assert_eq!(commit.parents, parents);
    }

    // ── Timestamp handling ─────────────────────────────────────────────────

    #[test]
    fn commit_timestamp_epoch_zero() {
        let epoch = chrono::DateTime::from_timestamp(0, 0).expect("epoch");
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            epoch,
            vec![],
        );
        assert_eq!(
            commit.timestamp,
            chrono::DateTime::from_timestamp(0, 0).unwrap()
        );
    }

    #[test]
    fn commit_timestamp_specific_date() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            ts,
            vec![],
        );
        assert_eq!(commit.timestamp.to_rfc3339(), "2024-06-15T12:30:45+00:00");
    }

    #[test]
    fn commit_timestamp_far_future() {
        let ts = chrono::DateTime::parse_from_rfc3339("2099-12-31T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            ts,
            vec![],
        );
        assert_eq!(commit.timestamp, ts);
    }

    #[test]
    fn commit_timestamp_millisecond_precision() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00.123Z")
            .unwrap()
            .with_timezone(&Utc);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            ts,
            vec![],
        );
        assert_eq!(commit.timestamp.timestamp_millis(), 1704067200123);
    }

    #[test]
    fn commit_timestamp_microsecond_precision() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00.123456Z")
            .unwrap()
            .with_timezone(&Utc);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            ts,
            vec![],
        );
        assert_eq!(commit.timestamp.timestamp_micros(), 1704067200123456);
    }

    // ── Author field edge cases ────────────────────────────────────────────

    #[test]
    fn commit_author_with_email() {
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "Alice <alice@example.com>".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.author.contains('<'));
        assert!(commit.author.contains('>'));
        assert!(commit.author.contains('@'));
    }

    #[test]
    fn commit_author_name_only() {
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "Alice".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.author, "Alice");
    }

    #[test]
    fn commit_author_unicode() {
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "José García <jose@例え.jp>".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.author.contains('é'));
        assert!(commit.author.contains('í'));
        assert!(commit.author.contains("例え"));
    }

    #[test]
    fn commit_author_empty_string() {
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            String::new(),
            Utc::now(),
            vec![],
        );
        assert!(commit.author.is_empty());
    }

    #[test]
    fn commit_author_very_long() {
        let long_author = "A".repeat(10_000);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            long_author.clone(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.author, long_author);
    }

    // ── Message field edge cases ───────────────────────────────────────────

    #[test]
    fn commit_message_single_line() {
        let commit = Commit::new(
            "abc".to_string(),
            "Fix bug in parser".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.message, "Fix bug in parser");
        assert!(!commit.message.contains('\n'));
    }

    #[test]
    fn commit_message_multiline() {
        let msg = "Fix bug in parser\n\nDetailed explanation.\n\nSigned-off-by: Alice";
        let commit = Commit::new(
            "abc".to_string(),
            msg.to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.message, msg);
        assert!(commit.message.contains('\n'));
    }

    #[test]
    fn commit_message_empty() {
        let commit = Commit::new(
            "abc".to_string(),
            String::new(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.message.is_empty());
    }

    #[test]
    fn commit_message_unicode() {
        let msg = "修正解析器错误 🐛";
        let commit = Commit::new(
            "abc".to_string(),
            msg.to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.message.contains("修正"));
        assert!(commit.message.contains("🐛"));
    }

    #[test]
    fn commit_message_very_long() {
        let long_msg = "x".repeat(100_000);
        let commit = Commit::new(
            "abc".to_string(),
            long_msg.clone(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        assert_eq!(commit.message, long_msg);
        assert_eq!(commit.message.len(), 100_000);
    }

    // ── Clone independence ─────────────────────────────────────────────────

    #[test]
    fn commit_clone_independence() {
        let mut commit = Commit::new(
            "abc".to_string(),
            "original".to_string(),
            "Alice".to_string(),
            Utc::now(),
            vec!["p1".to_string()],
        );
        let cloned = commit.clone();
        commit.id = "changed".to_string();
        commit.message = "changed".to_string();
        commit.parents.push("p2".to_string());
        assert_eq!(cloned.id, "abc");
        assert_eq!(cloned.message, "original");
        assert_eq!(cloned.parents.len(), 1);
    }

    #[test]
    fn commit_clone_preserves_timestamp() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "test".to_string(),
            ts,
            vec!["p1".to_string(), "p2".to_string()],
        );
        let cloned = commit.clone();
        assert_eq!(cloned.id, commit.id);
        assert_eq!(cloned.message, commit.message);
        assert_eq!(cloned.author, commit.author);
        assert_eq!(cloned.timestamp, commit.timestamp);
        assert_eq!(cloned.parents, commit.parents);
    }

    // ── Debug format ───────────────────────────────────────────────────────

    #[test]
    fn commit_debug_contains_type_name() {
        let commit = Commit::new(
            "abc123".to_string(),
            "test msg".to_string(),
            "Test Author".to_string(),
            Utc::now(),
            vec!["parent1".to_string()],
        );
        let debug = format!("{commit:?}");
        assert!(debug.contains("Commit"), "Debug should contain type name");
    }

    #[test]
    fn commit_debug_contains_id() {
        let commit = Commit::new(
            "deadbeef12345678".to_string(),
            "test".to_string(),
            "test".to_string(),
            Utc::now(),
            vec![],
        );
        let debug = format!("{commit:?}");
        assert!(
            debug.contains("deadbeef12345678"),
            "Debug should contain id value"
        );
    }

    // ── Comparison by field ────────────────────────────────────────────────

    #[test]
    fn commit_same_id_different_everything_else() {
        let sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let ts = Utc::now();
        let a = Commit::new(
            sha.to_string(),
            "msg a".to_string(),
            "author a".to_string(),
            ts,
            vec!["111".to_string()],
        );
        let b = Commit::new(
            sha.to_string(),
            "msg b".to_string(),
            "author b".to_string(),
            chrono::DateTime::from_timestamp(99999, 0).unwrap(),
            vec!["222".to_string()],
        );
        assert_eq!(a.id, b.id);
        assert_ne!(a.message, b.message);
        assert_ne!(a.author, b.author);
        assert_ne!(a.timestamp, b.timestamp);
        assert_ne!(a.parents, b.parents);
    }

    #[test]
    fn commit_different_id_same_everything_else() {
        let ts = Utc::now();
        let a = Commit::new(
            "aaa".to_string(),
            "msg".to_string(),
            "author".to_string(),
            ts,
            vec!["p1".to_string()],
        );
        let b = Commit::new(
            "bbb".to_string(),
            "msg".to_string(),
            "author".to_string(),
            ts,
            vec!["p1".to_string()],
        );
        assert_ne!(a.id, b.id);
        assert_eq!(a.message, b.message);
        assert_eq!(a.author, b.author);
        assert_eq!(a.timestamp, b.timestamp);
        assert_eq!(a.parents, b.parents);
    }

    // ── Serde exhaustive ───────────────────────────────────────────────────

    #[test]
    fn commit_serde_roundtrip_root_commit() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let original = Commit::new(
            "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b".to_string(),
            "Initial commit".to_string(),
            "Alice <alice@example.com>".to_string(),
            ts,
            vec![],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.message, original.message);
        assert_eq!(decoded.author, original.author);
        assert_eq!(decoded.timestamp, original.timestamp);
        assert_eq!(decoded.parents, original.parents);
    }

    #[test]
    fn commit_serde_roundtrip_merge_commit() {
        let ts = Utc::now();
        let original = Commit::new(
            "deadbeef".to_string(),
            "Merge".to_string(),
            "Bob".to_string(),
            ts,
            vec!["aaa".to_string(), "bbb".to_string()],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.parents.len(), 2);
        assert_eq!(decoded.parents[0], "aaa");
        assert_eq!(decoded.parents[1], "bbb");
    }

    #[test]
    fn commit_serde_preserves_empty_fields() {
        let original = Commit::new(
            "abc".to_string(),
            String::new(),
            String::new(),
            Utc::now(),
            vec![],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert!(decoded.message.is_empty());
        assert!(decoded.author.is_empty());
        assert!(decoded.parents.is_empty());
    }

    #[test]
    fn commit_deserialize_from_known_json() {
        let json = r#"{"id":"abc123","message":"test","author":"Alice","timestamp":"2024-01-01T00:00:00Z","parents":["p1","p2"]}"#;
        let c: Commit = serde_json::from_str(json).expect("deserialize");
        assert_eq!(c.id, "abc123");
        assert_eq!(c.message, "test");
        assert_eq!(c.author, "Alice");
        assert_eq!(c.parents.len(), 2);
        assert_eq!(c.parents[0], "p1");
        assert_eq!(c.parents[1], "p2");
    }

    #[test]
    fn commit_serde_unicode_roundtrip() {
        let original = Commit::new(
            "abc".to_string(),
            "修正 🐛 日本語テスト".to_string(),
            "José García <jose@例え.jp>".to_string(),
            Utc::now(),
            vec![],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.message, original.message);
        assert_eq!(decoded.author, original.author);
    }

    #[test]
    fn commit_serde_json_format() {
        let commit = Commit::new(
            "abc".to_string(),
            "test".to_string(),
            "Alice".to_string(),
            chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            vec!["p1".to_string()],
        );
        let json = serde_json::to_string(&commit).expect("serialize");
        assert!(json.contains("\"id\":\"abc\""));
        assert!(json.contains("\"message\":\"test\""));
        assert!(json.contains("\"author\":\"Alice\""));
        assert!(json.contains("\"parents\":[\"p1\"]"));
    }

    // ========================================================================
    // Commit proptest — ha-9en3
    // ========================================================================

    proptest::proptest! {
        /// Commit::new() stores all fields exactly as provided.
        #[test]
        fn proptest_commit_new_stores_exact(
            id in "[a-f0-9]{40}",
            message in ".*",
            author in ".*",
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..5),
        ) {
            let ts = Utc::now();
            let commit = Commit::new(
                id.clone(),
                message.clone(),
                author.clone(),
                ts,
                parents.clone(),
            );
            assert_eq!(commit.id, id);
            assert_eq!(commit.message, message);
            assert_eq!(commit.author, author);
            assert_eq!(commit.parents, parents);
        }

        /// Commit serde round-trip preserves all fields.
        #[test]
        fn proptest_commit_serde_roundtrip(
            id in ".*",
            message in ".*",
            author in ".*",
            parents in proptest::collection::vec(".*", 0..5),
        ) {
            let ts = Utc::now();
            let original = Commit::new(
                id.clone(),
                message.clone(),
                author.clone(),
                ts,
                parents.clone(),
            );
            let json = serde_json::to_string(&original).expect("serialize");
            let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded.id, id);
            assert_eq!(decoded.message, message);
            assert_eq!(decoded.author, author);
            assert_eq!(decoded.parents, parents);
        }

        /// Commit clone is always identical (field-by-field).
        #[test]
        fn proptest_commit_clone_identical(
            id in "[a-f0-9]{40}",
            message in ".*",
            author in ".*",
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..5),
        ) {
            let ts = Utc::now();
            let commit = Commit::new(
                id.clone(),
                message.clone(),
                author.clone(),
                ts,
                parents.clone(),
            );
            let cloned = commit.clone();
            assert_eq!(cloned.id, id);
            assert_eq!(cloned.message, message);
            assert_eq!(cloned.author, author);
            assert_eq!(cloned.parents, parents);
        }

        /// Commit parents count always matches input.
        #[test]
        fn proptest_commit_parents_count(
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..=10),
        ) {
            let count = parents.len();
            let commit = Commit::new(
                "abc".to_string(),
                "test".to_string(),
                "test".to_string(),
                Utc::now(),
                parents.clone(),
            );
            assert_eq!(commit.parents.len(), count);
            for (i, p) in parents.iter().enumerate() {
                assert_eq!(&commit.parents[i], p);
            }
        }

        /// Commit id length is preserved exactly.
        #[test]
        fn proptest_commit_id_length_preserved(
            id in proptest::string::string_regex("[a-f0-9]{1,100}").unwrap(),
        ) {
            let expected_len = id.len();
            let commit = Commit::new(
                id.clone(),
                "test".to_string(),
                "test".to_string(),
                Utc::now(),
                vec![],
            );
            assert_eq!(commit.id.len(), expected_len);
            assert_eq!(commit.id, id);
        }
    }

    // -- Branch tests --

    #[test]
    fn branch_new_with_tracking() {
        let branch = Branch::new("main".to_string(), true, Some("origin/main".to_string()));
        assert_eq!(branch.name, "main");
        assert!(branch.is_current);
        assert_eq!(branch.tracking, Some("origin/main".to_string()));
    }

    #[test]
    fn branch_new_without_tracking() {
        let branch = Branch::new("develop".to_string(), false, None);
        assert_eq!(branch.name, "develop");
        assert!(!branch.is_current);
        assert!(branch.tracking.is_none());
    }

    #[test]
    fn branch_clone() {
        let branch = Branch::new(
            "feature".to_string(),
            false,
            Some("origin/feature".to_string()),
        );
        let cloned = branch.clone();
        assert_eq!(branch.name, cloned.name);
        assert_eq!(branch.is_current, cloned.is_current);
    }

    #[test]
    fn branch_serde_roundtrip() {
        let branch = Branch::new(
            "release".to_string(),
            true,
            Some("origin/release".to_string()),
        );
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(branch.name, deserialized.name);
        assert_eq!(branch.is_current, deserialized.is_current);
        assert_eq!(branch.tracking, deserialized.tracking);
    }

    #[test]
    fn branch_serde_roundtrip_no_tracking() {
        let branch = Branch::new("hotfix".to_string(), false, None);
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.tracking.is_none());
    }

    // -- Workspace tests --

    #[test]
    fn workspace_new_current() {
        let ws = Workspace::new("default".to_string(), "main".to_string(), true);
        assert_eq!(ws.name, "default");
        assert_eq!(ws.branch, "main");
        assert!(ws.is_current);
    }

    #[test]
    fn workspace_new_not_current() {
        let ws = Workspace::new("feature-ws".to_string(), "feature/x".to_string(), false);
        assert_eq!(ws.name, "feature-ws");
        assert!(!ws.is_current);
    }

    #[test]
    fn workspace_clone() {
        let ws = Workspace::new("ws1".to_string(), "main".to_string(), true);
        let cloned = ws.clone();
        assert_eq!(ws.name, cloned.name);
        assert_eq!(ws.branch, cloned.branch);
        assert_eq!(ws.is_current, cloned.is_current);
    }

    #[test]
    fn workspace_serde_roundtrip() {
        let ws = Workspace::new("ws2".to_string(), "develop".to_string(), false);
        let json = serde_json::to_string(&ws).expect("serialize");
        let deserialized: Workspace = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ws.name, deserialized.name);
        assert_eq!(ws.branch, deserialized.branch);
        assert_eq!(ws.is_current, deserialized.is_current);
    }
}
