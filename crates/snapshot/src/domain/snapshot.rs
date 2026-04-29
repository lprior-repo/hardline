use std::convert::TryFrom;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, SnapshotError};

/// The type of snapshot, determining its provenance and lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    Checkpoint,
    #[default]
    Manual,
    PreOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub branch_name: String,
    pub commit_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub snapshot_type: SnapshotType,
    pub description: Option<String>,
}

/// Validate a branch name, rejecting empty strings and strings containing
/// path traversal or separator characters that could cause security issues.
///
/// Rejected: empty strings, strings containing `..`, `/`, `\`, or `\0`.
pub fn validate_branch_name(name: &str) -> std::result::Result<(), SnapshotError> {
    if name.is_empty() {
        return Err(SnapshotError::ValidationError(
            "Branch name must not be empty".to_string(),
        ));
    }
    if name.contains('\0') {
        return Err(SnapshotError::ValidationError(
            "Branch name must not contain null bytes".to_string(),
        ));
    }
    if name.contains("..") {
        return Err(SnapshotError::ValidationError(
            "Branch name must not contain '..' (path traversal)".to_string(),
        ));
    }
    if name.contains('/') {
        return Err(SnapshotError::ValidationError(
            "Branch name must not contain '/'".to_string(),
        ));
    }
    if name.contains('\\') {
        return Err(SnapshotError::ValidationError(
            "Branch name must not contain backslash".to_string(),
        ));
    }
    Ok(())
}

impl Snapshot {
    /// Default TTL for snapshots: 24 hours.
    const DEFAULT_TTL_HOURS: i64 = 24;

    pub fn create(
        branch_name: String,
        commit_hash: String,
        description: Option<String>,
    ) -> Result<Self> {
        Self::create_with_type(branch_name, commit_hash, description, SnapshotType::default())
    }

    pub fn create_with_type(
        branch_name: String,
        commit_hash: String,
        description: Option<String>,
        snapshot_type: SnapshotType,
    ) -> Result<Self> {
        validate_branch_name(&branch_name)?;
        Ok(Self {
            id: SnapshotId::generate(),
            branch_name,
            commit_hash,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(Self::DEFAULT_TTL_HOURS)),
            snapshot_type,
            description,
        })
    }

    /// Returns true if this snapshot has an expiration and it has passed.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => expires < Utc::now(),
            None => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnapshotId(String);

impl SnapshotId {
    /// Generate a new unique snapshot ID with the `snap-` prefix.
    pub fn generate() -> Self {
        Self(format!("snap-{}", uuid::Uuid::new_v4()))
    }

    /// Parse a string into a SnapshotId, validating the `snap-` prefix and
    /// rejecting strings containing null bytes.
    pub fn parse(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        if s.contains('\0') {
            return Err(SnapshotError::InvalidSnapshot(
                "SnapshotId must not contain null bytes".to_string(),
            ));
        }
        if s.starts_with("snap-") && s.len() > 5 {
            Ok(Self(s))
        } else {
            Err(SnapshotError::InvalidSnapshot(format!(
                "SnapshotId must start with 'snap-' and be non-empty, got: {s}"
            )))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SnapshotId {
    type Error = SnapshotError;

    fn try_from(s: String) -> Result<Self> {
        Self::parse(s)
    }
}

impl Serialize for SnapshotId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SnapshotId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SnapshotId::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

    use super::*;

    // --- SnapshotId tests ---

    #[test]
    fn snapshot_id_generate_has_snap_prefix() {
        let id = SnapshotId::generate();
        assert!(
            id.as_str().starts_with("snap-"),
            "id should start with 'snap-', got: {}",
            id.as_str()
        );
    }

    #[test]
    fn snapshot_id_generate_is_unique() {
        let id1 = SnapshotId::generate();
        let id2 = SnapshotId::generate();
        assert_ne!(id1, id2, "two generated IDs should be unique");
    }

    #[test]
    fn snapshot_id_as_str_matches_inner() {
        let id = SnapshotId::generate();
        let s = id.as_str();
        assert!(!s.is_empty());
        assert_eq!(s.len(), format!("{id}").len());
    }

    #[test]
    fn snapshot_id_display_matches_as_str() {
        let id = SnapshotId::generate();
        assert_eq!(format!("{id}"), id.as_str());
    }

    #[test]
    fn snapshot_id_clone() {
        let id = SnapshotId::generate();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn snapshot_id_equality() {
        let id1 = SnapshotId::parse("snap-test-123").expect("valid");
        let id2 = SnapshotId::parse("snap-test-123").expect("valid");
        assert_eq!(id1, id2);
    }

    #[test]
    fn snapshot_id_hash() {
        use std::collections::HashSet;
        let id = SnapshotId::generate();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn snapshot_id_inequality() {
        let id1 = SnapshotId::parse("snap-aaa").expect("valid");
        let id2 = SnapshotId::parse("snap-bbb").expect("valid");
        assert_ne!(id1, id2);
    }

    #[test]
    fn snapshot_id_different_hash_values_for_different_ids() {
        use std::collections::HashSet;
        let id1 = SnapshotId::parse("snap-111").expect("valid");
        let id2 = SnapshotId::parse("snap-222").expect("valid");
        let set: HashSet<_> = [id1, id2].into_iter().collect();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn snapshot_id_as_str_does_not_expose_mutable_reference() {
        let id = SnapshotId::parse("snap-const-test").expect("valid");
        let s = id.as_str();
        assert_eq!(s, "snap-const-test");
    }

    #[test]
    fn snapshot_id_debug_contains_inner_value() {
        let id = SnapshotId::parse("snap-debug-test").expect("valid");
        let debug_str = format!("{id:?}");
        assert!(debug_str.contains("snap-debug-test"));
    }

    #[test]
    fn snapshot_id_from_known_string() {
        let id = SnapshotId::parse("snap-known").expect("valid");
        assert_eq!(id.as_str(), "snap-known");
    }

    #[test]
    fn snapshot_id_parse_rejects_empty_string() {
        let result = SnapshotId::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_id_parse_rejects_missing_prefix() {
        let result = SnapshotId::parse("not-a-snap-id");
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_id_parse_rejects_prefix_only() {
        let result = SnapshotId::parse("snap-");
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_id_with_special_characters() {
        let id = SnapshotId::parse("snap-テスト-path").expect("valid");
        assert_eq!(id.as_str(), "snap-テスト-path");
    }

    #[test]
    fn snapshot_id_parse_rejects_null_bytes() {
        let result = SnapshotId::parse("snap-test\0evil");
        assert!(result.is_err(), "null bytes should be rejected");
    }

    // --- validate_branch_name tests ---

    #[test]
    fn validate_branch_name_accepts_normal() {
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature-branch").is_ok());
        assert!(validate_branch_name("dev_v2").is_ok());
    }

    #[test]
    fn validate_branch_name_rejects_empty() {
        assert!(validate_branch_name("").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_null() {
        assert!(validate_branch_name("main\0evil").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_dotdot() {
        assert!(validate_branch_name("..").is_err());
        assert!(validate_branch_name("foo..bar").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_slash() {
        assert!(validate_branch_name("feature/foo").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_backslash() {
        assert!(validate_branch_name("feature\\foo").is_err());
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_create_sets_fields() {
        let snapshot = Snapshot::create(
            "feature-branch".to_string(),
            "abc123def".to_string(),
            Some("a test snapshot".to_string()),
        ).expect("valid snapshot");
        assert_eq!(snapshot.branch_name, "feature-branch");
        assert_eq!(snapshot.commit_hash, "abc123def");
        assert_eq!(snapshot.description, Some("a test snapshot".to_string()));
        assert!(snapshot.id.as_str().starts_with("snap-"));
    }

    #[test]
    fn snapshot_create_without_description() {
        let snapshot = Snapshot::create("main".to_string(), "deadbeef".to_string(), None).expect("valid snapshot");
        assert!(snapshot.description.is_none());
    }

    #[test]
    fn snapshot_create_generates_unique_ids() {
        let s1 = Snapshot::create("a".to_string(), "h1".to_string(), None).expect("valid");
        let s2 = Snapshot::create("b".to_string(), "h2".to_string(), None).expect("valid");
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn snapshot_clone() {
        let snapshot = Snapshot::create(
            "dev".to_string(),
            "123".to_string(),
            Some("desc".to_string()),
        ).expect("valid snapshot");
        let cloned = snapshot.clone();
        assert_eq!(snapshot.id, cloned.id);
        assert_eq!(snapshot.branch_name, cloned.branch_name);
        assert_eq!(snapshot.commit_hash, cloned.commit_hash);
    }

    #[test]
    fn snapshot_clone_preserves_description() {
        let snapshot = Snapshot::create(
            "x".to_string(),
            "y".to_string(),
            Some("preserved".to_string()),
        ).expect("valid snapshot");
        let cloned = snapshot.clone();
        assert_eq!(snapshot.description, cloned.description);
    }

    #[test]
    fn snapshot_clone_preserves_none_description() {
        let snapshot = Snapshot::create("x".to_string(), "y".to_string(), None).expect("valid snapshot");
        let cloned = snapshot.clone();
        assert!(cloned.description.is_none());
    }

    #[test]
    fn snapshot_create_with_empty_branch_name_rejected() {
        let result = Snapshot::create(String::new(), "abc".to_string(), None);
        assert!(result.is_err(), "empty branch name should be rejected");
        assert!(matches!(result.unwrap_err(), SnapshotError::ValidationError(_)));
    }

    #[test]
    fn snapshot_create_with_empty_commit_hash() {
        let snapshot = Snapshot::create("main".to_string(), String::new(), None).expect("valid snapshot");
        assert_eq!(snapshot.commit_hash, "");
    }

    #[test]
    fn snapshot_create_with_empty_description() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), Some(String::new())).expect("valid snapshot");
        assert_eq!(snapshot.description, Some(String::new()));
    }

    #[test]
    fn snapshot_created_at_is_recent() {
        let before = chrono::Utc::now();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let after = chrono::Utc::now();
        // created_at should be between before and after
        assert!(snapshot.created_at >= before);
        assert!(snapshot.created_at <= after);
    }

    #[test]
    fn snapshot_debug_contains_branch() {
        let snapshot = Snapshot::create("debug-branch".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let debug_str = format!("{snapshot:?}");
        assert!(debug_str.contains("debug-branch"));
    }

    #[test]
    fn snapshot_with_slash_in_branch_name_rejected() {
        let result = Snapshot::create(
            "feature/JS-123".to_string(),
            "abc".to_string(),
            None,
        );
        assert!(result.is_err(), "branch name with '/' should be rejected");
        assert!(matches!(result.unwrap_err(), SnapshotError::ValidationError(_)));
    }

    #[test]
    fn snapshot_with_long_description() {
        let long_desc = "a".repeat(10_000);
        let snapshot = Snapshot::create(
            "main".to_string(),
            "abc".to_string(),
            Some(long_desc.clone()),
        ).expect("valid snapshot");
        assert_eq!(snapshot.description, Some(long_desc));
        assert_eq!(snapshot.description.as_ref().map(|d| d.len()), Some(10_000));
    }

    #[test]
    fn snapshot_with_newlines_in_description() {
        let desc = "line1\nline2\nline3".to_string();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), Some(desc.clone())).expect("valid snapshot");
        assert_eq!(snapshot.description, Some(desc));
    }

    #[test]
    fn snapshot_with_unicode_description() {
        let desc = "日本語テスト Ñoño café".to_string();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), Some(desc.clone())).expect("valid snapshot");
        assert_eq!(snapshot.description, Some(desc));
    }

    #[test]
    fn snapshot_with_whitespace_branch_name() {
        let snapshot = Snapshot::create("   ".to_string(), "abc".to_string(), None).expect("valid snapshot");
        assert_eq!(snapshot.branch_name, "   ");
    }

    #[test]
    fn snapshot_two_snapshots_same_commit_different_branches() {
        let s1 = Snapshot::create("main".to_string(), "same-hash".to_string(), None).expect("valid");
        let s2 = Snapshot::create("dev".to_string(), "same-hash".to_string(), None).expect("valid");
        assert_eq!(s1.commit_hash, s2.commit_hash);
        assert_ne!(s1.id, s2.id);
        assert_ne!(s1.branch_name, s2.branch_name);
    }

    // --- New field tests: expires_at and snapshot_type ---

    #[test]
    fn snapshot_create_sets_expires_at_to_24_hours() {
        let before = chrono::Utc::now();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let after = chrono::Utc::now();
        let expires = snapshot.expires_at.expect("expires_at should be set");
        let expected_min = before + chrono::Duration::hours(24);
        let expected_max = after + chrono::Duration::hours(24);
        assert!(expires >= expected_min, "expires_at too early");
        assert!(expires <= expected_max, "expires_at too late");
    }

    #[test]
    fn snapshot_create_default_type_is_manual() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        assert_eq!(snapshot.snapshot_type, SnapshotType::Manual);
    }

    #[test]
    fn snapshot_create_with_type_checkpoint() {
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc".to_string(),
            None,
            SnapshotType::Checkpoint,
        ).expect("valid snapshot");
        assert_eq!(snapshot.snapshot_type, SnapshotType::Checkpoint);
        assert!(snapshot.expires_at.is_some());
    }

    #[test]
    fn snapshot_create_with_type_pre_operation() {
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc".to_string(),
            None,
            SnapshotType::PreOperation,
        ).expect("valid snapshot");
        assert_eq!(snapshot.snapshot_type, SnapshotType::PreOperation);
    }

    #[test]
    fn snapshot_is_expired_false_when_in_future() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        assert!(!snapshot.is_expired(), "fresh snapshot should not be expired");
    }

    #[test]
    fn snapshot_is_expired_false_when_no_expires_at() {
        let mut snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        snapshot.expires_at = None;
        assert!(!snapshot.is_expired(), "snapshot without expires_at never expires");
    }

    #[test]
    fn snapshot_type_default() {
        assert_eq!(SnapshotType::default(), SnapshotType::Manual);
    }

    #[test]
    fn snapshot_type_equality() {
        assert_eq!(SnapshotType::Checkpoint, SnapshotType::Checkpoint);
        assert_ne!(SnapshotType::Manual, SnapshotType::PreOperation);
    }

    #[test]
    fn snapshot_type_clone_copy() {
        let t = SnapshotType::PreOperation;
        let copied = t;
        assert_eq!(t, copied);
    }

    #[test]
    fn snapshot_serialize_deserialize_preserves_expires_at() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let deserialized: Snapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.expires_at, snapshot.expires_at);
    }

    #[test]
    fn snapshot_serialize_deserialize_preserves_snapshot_type() {
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc".to_string(),
            None,
            SnapshotType::PreOperation,
        ).expect("valid snapshot");
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let deserialized: Snapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.snapshot_type, SnapshotType::PreOperation);
    }

    #[test]
    fn snapshot_clone_preserves_new_fields() {
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc".to_string(),
            Some("desc".to_string()),
            SnapshotType::Checkpoint,
        ).expect("valid snapshot");
        let cloned = snapshot.clone();
        assert_eq!(snapshot.expires_at, cloned.expires_at);
        assert_eq!(snapshot.snapshot_type, cloned.snapshot_type);
    }

    // --- Serialization roundtrip ---

    #[test]
    fn snapshot_serialize_deserialize_roundtrip() {
        let snapshot = Snapshot::create(
            "release".to_string(),
            "abcdef".to_string(),
            Some("release snapshot".to_string()),
        ).expect("valid snapshot");
        let json = serde_json::to_string(&snapshot).expect("serialize should succeed");
        let deserialized: Snapshot =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.id, snapshot.id);
        assert_eq!(deserialized.branch_name, snapshot.branch_name);
        assert_eq!(deserialized.commit_hash, snapshot.commit_hash);
        assert_eq!(deserialized.description, snapshot.description);
    }

    #[test]
    fn snapshot_id_serialize_deserialize_roundtrip() {
        let id = SnapshotId::generate();
        let json = serde_json::to_string(&id).expect("serialize should succeed");
        let deserialized: SnapshotId =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized, id);
    }

    #[test]
    fn snapshot_serialize_json_contains_expected_keys() {
        let snapshot = Snapshot::create(
            "test-branch".to_string(),
            "aaaa".to_string(),
            Some("test desc".to_string()),
        ).expect("valid snapshot");
        let json = serde_json::to_value(&snapshot).expect("serialize should succeed");
        let obj = json.as_object().expect("should be an object");
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("branch_name"));
        assert!(obj.contains_key("commit_hash"));
        assert!(obj.contains_key("created_at"));
        assert!(obj.contains_key("expires_at"));
        assert!(obj.contains_key("snapshot_type"));
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn snapshot_serialize_without_description() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let json = serde_json::to_value(&snapshot).expect("serialize should succeed");
        let obj = json.as_object().expect("should be an object");
        assert_eq!(
            obj.get("description").expect("key should exist"),
            &serde_json::Value::Null
        );
    }

    #[test]
    fn snapshot_deserialize_from_valid_json() {
        let json = r#"{
            "id": "snap-test-123",
            "branch_name": "feature",
            "commit_hash": "abc123",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-01-02T00:00:00Z",
            "snapshot_type": "manual",
            "description": "test"
        }"#;
        let snapshot: Snapshot = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(snapshot.id.as_str(), "snap-test-123");
        assert_eq!(snapshot.branch_name, "feature");
        assert_eq!(snapshot.commit_hash, "abc123");
        assert_eq!(snapshot.description, Some("test".to_string()));
    }

    #[test]
    fn snapshot_deserialize_missing_optional_fields() {
        let json = r#"{
            "id": "snap-test-456",
            "branch_name": "main",
            "commit_hash": "def456",
            "created_at": "2024-06-15T12:30:00Z",
            "expires_at": null,
            "snapshot_type": "manual",
            "description": null
        }"#;
        let snapshot: Snapshot = serde_json::from_str(json).expect("deserialize should succeed");
        assert!(snapshot.description.is_none());
        assert!(snapshot.expires_at.is_none());
    }

    #[test]
    fn snapshot_deserialize_fails_with_missing_required_field() {
        let json = r#"{
            "id": "snap-test-789",
            "branch_name": "main",
            "commit_hash": "ghi789"
        }"#;
        let result: std::result::Result<Snapshot, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing created_at should fail");
    }

    #[test]
    fn snapshot_deserialize_fails_with_empty_json() {
        let result: std::result::Result<Snapshot, _> = serde_json::from_str("{}");
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_deserialize_fails_with_invalid_json() {
        let result: std::result::Result<Snapshot, _> = serde_json::from_str("not json");
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_deserialize_fails_with_wrong_types() {
        let json = r#"{
            "id": 12345,
            "branch_name": "main",
            "commit_hash": "abc",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": null,
            "snapshot_type": "manual",
            "description": null
        }"#;
        let result: std::result::Result<Snapshot, _> = serde_json::from_str(json);
        assert!(result.is_err(), "id should be a string, not a number");
    }

    #[test]
    fn snapshot_serialize_produces_valid_json() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let json = serde_json::to_string(&snapshot).expect("serialize should succeed");
        // Verify it parses back as valid JSON
        let _value: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
    }

    #[test]
    fn snapshot_serialize_deserialize_preserves_created_at() {
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let deserialized: Snapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(snapshot.created_at, deserialized.created_at);
    }

    #[test]
    fn snapshot_id_serialize_from_known_string() {
        let id = SnapshotId::parse("snap-manual-123").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize should succeed");
        assert_eq!(json, "\"snap-manual-123\"");
    }

    #[test]
    fn snapshot_id_deserialize_from_string() {
        let json = "\"snap-manual-456\"";
        let id: SnapshotId = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(id.as_str(), "snap-manual-456");
    }

    #[test]
    fn snapshot_id_deserialize_fails_from_non_string() {
        let json = "12345";
        let result: std::result::Result<SnapshotId, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_id_deserialize_fails_from_invalid_prefix() {
        let json = "\"not-a-snapshot\"";
        let result: std::result::Result<SnapshotId, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_id_deserialize_fails_from_empty_string() {
        let json = "\"\"";
        let result: std::result::Result<SnapshotId, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_json_bytes_roundtrip() {
        let snapshot = Snapshot::create(
            "main".to_string(),
            "abc".to_string(),
            Some("bytes".to_string()),
        ).expect("valid snapshot");
        let bytes = serde_json::to_vec(&snapshot).expect("to_vec should succeed");
        let deserialized: Snapshot =
            serde_json::from_slice(&bytes).expect("from_slice should succeed");
        assert_eq!(deserialized.id, snapshot.id);
        assert_eq!(deserialized.branch_name, snapshot.branch_name);
    }

    #[test]
    fn snapshot_pretty_print_roundtrip() {
        let snapshot = Snapshot::create(
            "main".to_string(),
            "abc".to_string(),
            Some("pretty".to_string()),
        ).expect("valid snapshot");
        let pretty = serde_json::to_string_pretty(&snapshot).expect("pretty print should succeed");
        let deserialized: Snapshot =
            serde_json::from_str(&pretty).expect("roundtrip should succeed");
        assert_eq!(deserialized.id, snapshot.id);
    }

    // --- Proptests ---

    proptest! {
        #[test]
        fn snapshot_create_always_has_snap_prefix(branch in "[a-zA-Z0-9_-]{1,50}", commit in "[a-f0-9]{1,40}") {
            let snapshot = Snapshot::create(branch, commit, None).expect("valid snapshot");
            prop_assert!(snapshot.id.as_str().starts_with("snap-"));
        }

        #[test]
        fn snapshot_create_preserves_branch_and_commit(branch in "[a-zA-Z0-9_.-]{1,100}", commit in "[a-f0-9]{1,40}") {
            let snapshot = Snapshot::create(branch.clone(), commit.clone(), None).expect("valid snapshot");
            prop_assert_eq!(snapshot.branch_name, branch);
            prop_assert_eq!(snapshot.commit_hash, commit);
        }

        #[test]
        fn snapshot_create_preserves_description(branch in "[a-z]{1,10}", commit in "[a-f0-9]{7}", desc in "[a-zA-Z ]{0,200}") {
            let snapshot = Snapshot::create(branch.clone(), commit.clone(), Some(desc.clone())).expect("valid snapshot");
            prop_assert_eq!(snapshot.description, Some(desc));
        }

        #[test]
        fn snapshot_id_roundtrip_via_json(inner in "[a-zA-Z0-9_-]{1,100}") {
            let full = format!("snap-{inner}");
            let id = SnapshotId::parse(&full).expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let deserialized: SnapshotId = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(deserialized, id);
        }

        #[test]
        fn snapshot_roundtrip_via_json(branch in "[a-zA-Z0-9_-]{1,50}", commit in "[a-f0-9]{7,40}", desc in ".{0,500}") {
            let snapshot = Snapshot::create(branch.clone(), commit.clone(), Some(desc.clone())).expect("valid snapshot");
            // Roundtrip
            let json = serde_json::to_string(&snapshot).expect("serialize");
            let deserialized: Snapshot = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(deserialized.branch_name, branch);
            prop_assert_eq!(deserialized.commit_hash, commit);
            prop_assert_eq!(deserialized.description, Some(desc));
            prop_assert!(deserialized.id.as_str().starts_with("snap-"));
            // created_at should be preserved
            prop_assert_eq!(deserialized.created_at, snapshot.created_at);
            // new fields should be preserved
            prop_assert_eq!(deserialized.expires_at, snapshot.expires_at);
            prop_assert_eq!(deserialized.snapshot_type, snapshot.snapshot_type);
        }

        #[test]
        fn snapshot_create_generates_unique_ids_for_same_input(branch in "[a-z]{1,5}", commit in "[a-f0-9]{7}") {
            let s1 = Snapshot::create(branch.clone(), commit.clone(), None).expect("valid");
            let s2 = Snapshot::create(branch.clone(), commit.clone(), None).expect("valid");
            prop_assert_ne!(s1.id, s2.id);
        }
    }
}
