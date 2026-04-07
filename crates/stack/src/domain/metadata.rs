//! Per-branch metadata for stacked PRs.
//!
//! Ported from stax `engine/metadata.rs`. Each tracked branch stores its parent
//! branch name, the parent's commit at last-rebase time, and optional PR info.
//! Data is persisted as JSON in git refs under `refs/branch-metadata/<branch>`.

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};

/// PR information stored alongside branch metadata.
///
/// Simpler than the full `PrInfo` in `domain/stack.rs` — this is the
/// on-disk format compatible with stax/freephite JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPrInfo {
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_draft: Option<bool>,
}

/// Metadata stored for each tracked branch.
///
/// Serialized as JSON to `refs/branch-metadata/<branch>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchMetadata {
    /// Name of the parent branch.
    #[serde(default)]
    pub parent_branch_name: String,
    /// Commit SHA of parent when this branch was last rebased.
    #[serde(default)]
    pub parent_branch_revision: String,
    /// PR information (if submitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_info: Option<MetadataPrInfo>,
}

impl BranchMetadata {
    /// Create new metadata for a branch.
    #[must_use]
    pub fn new(parent_name: &str, parent_revision: &str) -> Self {
        Self {
            parent_branch_name: parent_name.to_string(),
            parent_branch_revision: parent_revision.to_string(),
            pr_info: None,
        }
    }

    /// Create new metadata with PR info.
    #[must_use]
    pub fn with_pr(mut self, number: u64, state: &str, is_draft: Option<bool>) -> Self {
        self.pr_info = Some(MetadataPrInfo {
            number,
            state: state.to_string(),
            is_draft,
        });
        self
    }

    /// Check if the branch needs restacking (parent has moved from recorded revision).
    pub fn needs_restack(&self, current_parent_revision: &str) -> bool {
        current_parent_revision != self.parent_branch_revision
    }

    /// Get the parent branch name as a typed `BranchName`.
    #[must_use]
    pub fn parent_branch(&self) -> BranchName {
        BranchName::new(&self.parent_branch_name)
    }

    /// Validate that required fields are present and non-empty.
    pub fn validate(&self) -> Result<()> {
        if self.parent_branch_name.trim().is_empty() {
            return Err(StackError::InvalidBranchName(
                "parent_branch_name must not be empty".to_string(),
            ));
        }
        if self.parent_branch_revision.trim().is_empty() {
            return Err(StackError::InvalidBranchName(
                "parent_branch_revision must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| StackError::GitError(format!("metadata serialization failed: {e}")))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| StackError::GitError(format!("metadata deserialization failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_new() {
        let meta = BranchMetadata::new("main", "abc123");
        assert_eq!(meta.parent_branch_name, "main");
        assert_eq!(meta.parent_branch_revision, "abc123");
        assert!(meta.pr_info.is_none());
    }

    #[test]
    fn test_metadata_with_pr() {
        let meta = BranchMetadata::new("main", "abc123").with_pr(42, "OPEN", Some(false));
        assert!(meta.pr_info.is_some());
        let pr = meta.pr_info.expect("pr info");
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.is_draft, Some(false));
    }

    #[test]
    fn test_metadata_serialization_camel_case() {
        let meta = BranchMetadata::new("main", "abc123");
        let json = meta.to_json().expect("serialize");
        assert!(json.contains("parentBranchName"));
        assert!(json.contains("main"));
        assert!(json.contains("parentBranchRevision"));
    }

    #[test]
    fn test_metadata_deserialization() {
        let json = r#"{"parentBranchName":"main","parentBranchRevision":"abc123"}"#;
        let meta = BranchMetadata::from_json(json).expect("deserialize");
        assert_eq!(meta.parent_branch_name, "main");
        assert_eq!(meta.parent_branch_revision, "abc123");
    }

    #[test]
    fn test_metadata_with_pr_info_json() {
        let json = r#"{
            "parentBranchName": "main",
            "parentBranchRevision": "abc123",
            "prInfo": {
                "number": 42,
                "state": "OPEN",
                "isDraft": false
            }
        }"#;
        let meta = BranchMetadata::from_json(json).expect("deserialize");
        assert!(meta.pr_info.is_some());
        let pr = meta.pr_info.expect("pr info");
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.is_draft, Some(false));
    }

    #[test]
    fn test_metadata_deserialization_missing_parent_fields_uses_defaults() {
        let json = r#"{
            "prInfo": {
                "number": 99,
                "state": "OPEN"
            }
        }"#;
        let meta = BranchMetadata::from_json(json).expect("deserialize");
        assert_eq!(meta.parent_branch_name, "");
        assert_eq!(meta.parent_branch_revision, "");
        assert!(meta.pr_info.is_some());
    }

    #[test]
    fn test_freephite_compatibility() {
        let freephite_json = r#"{
            "parentBranchName": "main",
            "parentBranchRevision": "deadbeef1234567890",
            "prInfo": {
                "number": 123,
                "state": "OPEN",
                "isDraft": true
            }
        }"#;
        let meta = BranchMetadata::from_json(freephite_json).expect("deserialize");
        assert_eq!(meta.parent_branch_name, "main");
        assert_eq!(meta.parent_branch_revision, "deadbeef1234567890");
    }

    #[test]
    fn test_needs_restack_parent_moved() {
        let meta = BranchMetadata::new("main", "abc123");
        assert!(meta.needs_restack("def456"));
    }

    #[test]
    fn test_needs_restack_parent_same() {
        let meta = BranchMetadata::new("main", "abc123");
        assert!(!meta.needs_restack("abc123"));
    }

    #[test]
    fn test_parent_branch() {
        let meta = BranchMetadata::new("develop", "abc");
        assert_eq!(meta.parent_branch(), BranchName::new("develop"));
    }

    #[test]
    fn test_validate_ok() {
        let meta = BranchMetadata::new("main", "abc123");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_parent_name() {
        let meta = BranchMetadata::new("", "abc");
        assert!(meta.validate().is_err());
    }

    #[test]
    fn test_validate_empty_parent_revision() {
        let meta = BranchMetadata::new("main", "");
        assert!(meta.validate().is_err());
    }

    #[test]
    fn test_validate_whitespace_only_parent_name() {
        let meta = BranchMetadata::new("   ", "abc");
        assert!(meta.validate().is_err());
    }

    #[test]
    fn test_validate_whitespace_only_parent_revision() {
        let meta = BranchMetadata::new("main", "  ");
        assert!(meta.validate().is_err());
    }

    #[test]
    fn test_roundtrip_json() {
        let meta = BranchMetadata::new("main", "abc123").with_pr(1, "OPEN", Some(false));
        let json = meta.to_json().expect("serialize");
        let back = BranchMetadata::from_json(&json).expect("deserialize");
        assert_eq!(meta, back);
    }

    #[test]
    fn test_from_json_invalid() {
        let result = BranchMetadata::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_pr_info_equality() {
        let a = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: Some(false),
        };
        let b = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: Some(false),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_pr_info_inequality_different_number() {
        let a = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: None,
        };
        let b = MetadataPrInfo {
            number: 2,
            state: "OPEN".to_string(),
            is_draft: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_pr_info_inequality_different_state() {
        let a = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: None,
        };
        let b = MetadataPrInfo {
            number: 1,
            state: "CLOSED".to_string(),
            is_draft: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_pr_info_inequality_different_draft() {
        let a = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: Some(true),
        };
        let b = MetadataPrInfo {
            number: 1,
            state: "OPEN".to_string(),
            is_draft: Some(false),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_metadata_equality() {
        let a = BranchMetadata::new("main", "abc").with_pr(1, "OPEN", None);
        let b = BranchMetadata::new("main", "abc").with_pr(1, "OPEN", None);
        assert_eq!(a, b);
    }

    #[test]
    fn test_metadata_inequality_different_parent() {
        let a = BranchMetadata::new("main", "abc");
        let b = BranchMetadata::new("develop", "abc");
        assert_ne!(a, b);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_metadata_json_roundtrip(
            parent in "[a-zA-Z0-9/_-]{1,50}",
            revision in "[a-f0-9]{1,40}",
        ) {
            let meta = BranchMetadata::new(&parent, &revision);
            let json = meta.to_json().expect("serialize");
            let back = BranchMetadata::from_json(&json).expect("deserialize");
            assert_eq!(meta, back);
        }

        #[test]
        fn prop_metadata_pr_roundtrip(
            parent in "[a-z]{1,20}",
            revision in "[a-f0-9]{8}",
            number in 1u64..1_000_000u64,
            state in "(OPEN|CLOSED|MERGED)",
        ) {
            let meta = BranchMetadata::new(&parent, &revision)
                .with_pr(number, &state, Some(number % 2 == 0));
            let json = meta.to_json().expect("serialize");
            let back = BranchMetadata::from_json(&json).expect("deserialize");
            assert_eq!(meta, back);
        }

        #[test]
        fn prop_needs_restack_detects_change(
            original in "[a-f0-9]{8}",
            current in "[a-f0-9]{8}",
        ) {
            let meta = BranchMetadata::new("main", &original);
            assert_eq!(meta.needs_restack(&current), original != current);
        }

        #[test]
        fn prop_validate_rejects_empty(parent: String, revision: String) {
            let meta = BranchMetadata::new(parent.trim(), revision.trim());
            let parent_ok = !parent.trim().is_empty();
            let rev_ok = !revision.trim().is_empty();
            let result = meta.validate();
            assert_eq!(result.is_ok(), parent_ok && rev_ok);
        }
    }
}
