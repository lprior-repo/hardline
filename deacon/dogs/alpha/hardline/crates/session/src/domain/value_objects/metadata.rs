//! Metadata value objects: Labels, DependsOn, Priority, IssueType, WorkspaceName

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(name: impl Into<String>) -> Result<Self, SessionError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "WorkspaceName cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
                "WorkspaceName exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for WorkspaceName {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(Vec<String>);

impl Labels {
    pub const MAX_LABELS: usize = 50;

    pub fn new(labels: Vec<String>) -> Result<Self, SessionError> {
        if labels.len() > Self::MAX_LABELS {
            return Err(SessionError::InvalidIdentifier(format!(
                "Too many labels (max {})",
                Self::MAX_LABELS
            )));
        }
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        if unique.len() != labels.len() {
            return Err(SessionError::InvalidIdentifier(
                "Labels contain duplicates".into(),
            ));
        }
        Ok(Self(labels))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl std::fmt::Display for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependsOn(String);

impl DependsOn {
    pub fn new(bead_id: impl Into<String>) -> Result<Self, SessionError> {
        let bead_id = bead_id.into();

        // Validate BeadId format (bd- prefix + hex)
        if bead_id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn cannot be empty".into(),
            ));
        }

        if !bead_id.starts_with("bd-") {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn must start with 'bd-'".into(),
            ));
        }

        let hex_part = &bead_id[3..];
        if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn must be valid hex after 'bd-'".into(),
            ));
        }

        Ok(Self(bead_id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for DependsOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for DependsOn {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

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

    #[must_use]
    pub fn into_inner(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u8> for Priority {
    type Error = SessionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueType(String);

impl IssueType {
    pub fn new(issue_type: impl Into<String>) -> Result<Self, SessionError> {
        let issue_type = issue_type.into();
        let valid_types = ["bug", "feature", "task", "epic", "chore"];
        if !valid_types.contains(&issue_type.as_str()) {
            return Err(SessionError::InvalidIssueType(format!(
                "Invalid issue type: {}. Must be one of: {}",
                issue_type,
                valid_types.join(", ")
            )));
        }
        Ok(Self(issue_type))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IssueType {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // WorkspaceName Tests (metadata variant)
    // =========================================================================

    mod workspace_name_tests {
        use super::*;

        #[test]
        fn workspace_name_valid() {
            let name = WorkspaceName::new("my-workspace").expect("valid");
            assert_eq!(name.as_str(), "my-workspace");
        }

        #[test]
        fn workspace_name_trims_whitespace() {
            let name = WorkspaceName::new("  padded  ").expect("valid");
            assert_eq!(name.as_str(), "padded");
        }

        #[test]
        fn workspace_name_empty_rejects() {
            let result = WorkspaceName::new("");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn workspace_name_whitespace_only_rejects() {
            let result = WorkspaceName::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_name_max_length_boundary() {
            let max_name = "w".repeat(WorkspaceName::MAX_LENGTH);
            let name = WorkspaceName::new(max_name).expect("at max length");
            assert_eq!(name.as_str().len(), WorkspaceName::MAX_LENGTH);
        }

        #[test]
        fn workspace_name_exceeds_max_length_rejects() {
            let too_long = "w".repeat(WorkspaceName::MAX_LENGTH + 1);
            let result = WorkspaceName::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn workspace_name_display() {
            let name = WorkspaceName::new("test-ws").expect("valid");
            assert_eq!(format!("{name}"), "test-ws");
        }

        #[test]
        fn workspace_name_try_from_string() {
            let name = WorkspaceName::try_from("ws".to_string()).expect("valid");
            assert_eq!(name.as_str(), "ws");
        }

        #[test]
        fn workspace_name_into_inner() {
            let name = WorkspaceName::new("inner-ws").expect("valid");
            assert_eq!(name.into_inner(), "inner-ws");
        }
    }

    // =========================================================================
    // Labels Tests
    // =========================================================================

    mod labels_tests {
        use super::*;

        #[test]
        fn labels_empty_list() {
            let labels = Labels::new(vec![]).expect("empty is valid");
            assert!(labels.as_slice().is_empty());
        }

        #[test]
        fn labels_valid_list() {
            let labels = Labels::new(vec![
                "bug".to_string(),
                "urgent".to_string(),
                "backend".to_string(),
            ])
            .expect("valid");
            assert_eq!(labels.as_slice().len(), 3);
        }

        #[test]
        fn labels_exceeds_max_rejects() {
            let too_many: Vec<String> = (0..=Labels::MAX_LABELS)
                .map(|i| format!("label-{i}"))
                .collect();
            let result = Labels::new(too_many);
            assert!(result.is_err());
        }

        #[test]
        fn labels_duplicates_reject() {
            let result = Labels::new(vec![
                "bug".to_string(),
                "feature".to_string(),
                "bug".to_string(),
            ]);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn labels_at_max_boundary() {
            let labels_vec: Vec<String> = (0..Labels::MAX_LABELS)
                .map(|i| format!("label-{i}"))
                .collect();
            let labels = Labels::new(labels_vec).expect("at max boundary");
            assert_eq!(labels.as_slice().len(), Labels::MAX_LABELS);
        }

        #[test]
        fn labels_display_empty() {
            let labels = Labels::new(vec![]).expect("valid");
            assert_eq!(format!("{labels}"), "[]");
        }

        #[test]
        fn labels_display_with_items() {
            let labels = Labels::new(vec!["a".to_string(), "b".to_string()]).expect("valid");
            assert_eq!(format!("{labels}"), "[a, b]");
        }

        #[test]
        fn labels_into_inner() {
            let labels = Labels::new(vec!["x".to_string()]).expect("valid");
            let inner = labels.into_inner();
            assert_eq!(inner, vec!["x".to_string()]);
        }
    }

    // =========================================================================
    // DependsOn Tests
    // =========================================================================

    mod depends_on_tests {
        use super::*;

        #[test]
        fn depends_on_valid() {
            let dep = DependsOn::new("bd-abc123").expect("valid");
            assert_eq!(dep.as_str(), "bd-abc123");
        }

        #[test]
        fn depends_on_empty_rejects() {
            let result = DependsOn::new("");
            assert!(result.is_err());
        }

        #[test]
        fn depends_on_missing_prefix_rejects() {
            let result = DependsOn::new("abc-123");
            assert!(result.is_err());
        }

        #[test]
        fn depends_on_empty_suffix_rejects() {
            let result = DependsOn::new("bd-");
            assert!(result.is_err());
        }

        #[test]
        fn depends_on_invalid_hex_rejects() {
            let result = DependsOn::new("bd-xyz");
            assert!(result.is_err());
        }

        #[test]
        fn depends_on_uppercase_hex() {
            let dep = DependsOn::new("bd-ABCDEF").expect("valid");
            assert_eq!(dep.as_str(), "bd-ABCDEF");
        }

        #[test]
        fn depends_on_display() {
            let dep = DependsOn::new("bd-123").expect("valid");
            assert_eq!(format!("{dep}"), "bd-123");
        }

        #[test]
        fn depends_on_try_from_string() {
            let dep = DependsOn::try_from("bd-deadbeef".to_string()).expect("valid");
            assert_eq!(dep.as_str(), "bd-deadbeef");
        }

        #[test]
        fn depends_on_into_inner() {
            let dep = DependsOn::new("bd-1").expect("valid");
            assert_eq!(dep.into_inner(), "bd-1");
        }
    }

    // =========================================================================
    // Priority Tests (metadata variant)
    // =========================================================================

    mod priority_tests {
        use super::*;

        #[test]
        fn priority_zero_valid() {
            let p = Priority::new(0).expect("critical");
            assert_eq!(p.as_u8(), 0);
        }

        #[test]
        fn priority_four_valid() {
            let p = Priority::new(4).expect("backlog");
            assert_eq!(p.as_u8(), 4);
        }

        #[test]
        fn priority_five_rejects() {
            let result = Priority::new(5);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_max_u8_rejects() {
            let result = Priority::new(255);
            assert!(result.is_err());
        }

        #[test]
        fn priority_display() {
            let p = Priority::new(2).expect("valid");
            assert_eq!(format!("{p}"), "2");
        }

        #[test]
        fn priority_try_from_u8() {
            let p = Priority::try_from(3).expect("valid");
            assert_eq!(p.as_u8(), 3);
        }

        #[test]
        fn priority_try_from_u8_invalid() {
            let result = Priority::try_from(10);
            assert!(result.is_err());
        }

        #[test]
        fn priority_into_inner() {
            let p = Priority::new(1).expect("valid");
            assert_eq!(p.into_inner(), 1);
        }
    }

    // =========================================================================
    // IssueType Tests
    // =========================================================================

    mod issue_type_tests {
        use super::*;

        #[test]
        fn issue_type_bug_valid() {
            let it = IssueType::new("bug").expect("valid");
            assert_eq!(it.as_str(), "bug");
        }

        #[test]
        fn issue_type_feature_valid() {
            let it = IssueType::new("feature").expect("valid");
            assert_eq!(it.as_str(), "feature");
        }

        #[test]
        fn issue_type_task_valid() {
            let it = IssueType::new("task").expect("valid");
            assert_eq!(it.as_str(), "task");
        }

        #[test]
        fn issue_type_epic_valid() {
            let it = IssueType::new("epic").expect("valid");
            assert_eq!(it.as_str(), "epic");
        }

        #[test]
        fn issue_type_chore_valid() {
            let it = IssueType::new("chore").expect("valid");
            assert_eq!(it.as_str(), "chore");
        }

        #[test]
        fn issue_type_invalid_rejects() {
            let result = IssueType::new("invalid-type");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIssueType(_)
            ));
        }

        #[test]
        fn issue_type_empty_rejects() {
            let result = IssueType::new("");
            assert!(result.is_err());
        }

        #[test]
        fn issue_type_case_sensitive() {
            let result = IssueType::new("Bug");
            assert!(result.is_err());
        }

        #[test]
        fn issue_type_display() {
            let it = IssueType::new("feature").expect("valid");
            assert_eq!(format!("{it}"), "feature");
        }

        #[test]
        fn issue_type_try_from_string() {
            let it = IssueType::try_from("chore".to_string()).expect("valid");
            assert_eq!(it.as_str(), "chore");
        }

        #[test]
        fn issue_type_try_from_invalid() {
            let result = IssueType::try_from("story".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn issue_type_into_inner() {
            let it = IssueType::new("epic").expect("valid");
            assert_eq!(it.into_inner(), "epic");
        }
    }

    // =========================================================================
    // WorkspaceName Serde Tests
    // =========================================================================

    mod workspace_name_serde_tests {
        use super::*;

        #[test]
        fn workspace_name_serde_roundtrip() {
            let name = WorkspaceName::new("my-workspace").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(name, parsed);
        }

        #[test]
        fn workspace_name_serde_json_output() {
            let name = WorkspaceName::new("ws").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            assert_eq!(json, "\"ws\"");
        }
    }

    // =========================================================================
    // Labels Serde Tests
    // =========================================================================

    mod labels_serde_tests {
        use super::*;

        #[test]
        fn labels_serde_roundtrip() {
            let labels = Labels::new(vec!["bug".to_string(), "urgent".to_string()]).expect("valid");
            let json = serde_json::to_string(&labels).expect("serialize");
            let parsed: Labels = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(labels, parsed);
        }

        #[test]
        fn labels_serde_roundtrip_empty() {
            let labels = Labels::new(vec![]).expect("valid");
            let json = serde_json::to_string(&labels).expect("serialize");
            let parsed: Labels = serde_json::from_str(&json).expect("deserialize");
            assert!(parsed.as_slice().is_empty());
        }

        #[test]
        fn labels_serde_json_output() {
            let labels = Labels::new(vec!["a".to_string()]).expect("valid");
            let json = serde_json::to_string(&labels).expect("serialize");
            assert_eq!(json, "[\"a\"]");
        }
    }

    // =========================================================================
    // DependsOn Serde Tests
    // =========================================================================

    mod depends_on_serde_tests {
        use super::*;

        #[test]
        fn depends_on_serde_roundtrip() {
            let dep = DependsOn::new("bd-abc123").expect("valid");
            let json = serde_json::to_string(&dep).expect("serialize");
            let parsed: DependsOn = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(dep, parsed);
        }

        #[test]
        fn depends_on_serde_json_output() {
            let dep = DependsOn::new("bd-cafe").expect("valid");
            let json = serde_json::to_string(&dep).expect("serialize");
            assert_eq!(json, "\"bd-cafe\"");
        }
    }

    // =========================================================================
    // Priority Serde Tests
    // =========================================================================

    mod priority_serde_tests {
        use super::*;

        #[test]
        fn priority_serde_roundtrip_all_levels() {
            for level in 0..=4u8 {
                let p = Priority::new(level).expect("valid");
                let json = serde_json::to_string(&p).expect("serialize");
                let parsed: Priority = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(p, parsed);
            }
        }

        #[test]
        fn priority_serde_json_output() {
            let p = Priority::new(0).expect("valid");
            let json = serde_json::to_string(&p).expect("serialize");
            assert_eq!(json, "0");
        }
    }

    // =========================================================================
    // IssueType Serde Tests
    // =========================================================================

    mod issue_type_serde_tests {
        use super::*;

        #[test]
        fn issue_type_serde_roundtrip_all_variants() {
            let variants = ["bug", "feature", "task", "epic", "chore"];
            for v in variants {
                let it = IssueType::new(v).expect("valid");
                let json = serde_json::to_string(&it).expect("serialize");
                let parsed: IssueType = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(it, parsed);
            }
        }

        #[test]
        fn issue_type_serde_json_output() {
            let it = IssueType::new("bug").expect("valid");
            let json = serde_json::to_string(&it).expect("serialize");
            assert_eq!(json, "\"bug\"");
        }
    }

    // =========================================================================
    // WorkspaceName Edge Cases
    // =========================================================================

    mod workspace_name_edge_tests {
        use super::*;

        #[test]
        fn workspace_name_single_char() {
            let name = WorkspaceName::new("a").expect("single char valid");
            assert_eq!(name.as_str(), "a");
        }

        #[test]
        fn workspace_name_with_spaces_inside() {
            let name = WorkspaceName::new("my workspace name").expect("spaces valid");
            assert_eq!(name.as_str(), "my workspace name");
        }

        #[test]
        fn workspace_name_equality() {
            let n1 = WorkspaceName::new("same").expect("valid");
            let n2 = WorkspaceName::new("same").expect("valid");
            let n3 = WorkspaceName::new("different").expect("valid");
            assert_eq!(n1, n2);
            assert_ne!(n1, n3);
        }

        #[test]
        fn workspace_name_hash_consistency() {
            use std::collections::HashSet;
            let n1 = WorkspaceName::new("hash-test").expect("valid");
            let n2 = WorkspaceName::new("hash-test").expect("valid");
            let mut set = HashSet::new();
            set.insert(n1);
            assert!(set.contains(&n2));
        }
    }

    // =========================================================================
    // Labels Edge Cases
    // =========================================================================

    mod labels_edge_tests {
        use super::*;

        #[test]
        fn labels_single_item() {
            let labels = Labels::new(vec!["only".to_string()]).expect("valid");
            assert_eq!(labels.as_slice(), &["only".to_string()]);
        }

        #[test]
        fn labels_with_special_chars() {
            let labels =
                Labels::new(vec!["bug:critical".to_string(), "ui/ux".to_string()]).expect("valid");
            assert_eq!(labels.as_slice().len(), 2);
        }

        #[test]
        fn labels_empty_string_item_rejected_for_duplication() {
            // Empty strings are technically valid as unique items
            let labels = Labels::new(vec!["".to_string()]).expect("empty string valid");
            assert_eq!(labels.as_slice(), &["".to_string()]);
        }
    }

    // =========================================================================
    // Proptests
    // =========================================================================

    mod metadata_proptests {
        use super::*;

        #[test]
        fn priority_all_valid_values() {
            for p in 0..=4u8 {
                let result = Priority::new(p);
                assert!(result.is_ok());
                assert_eq!(result.unwrap().as_u8(), p);
            }
        }

        #[test]
        fn priority_invalid_values_rejected() {
            for p in [5u8, 10, 100, 255] {
                assert!(Priority::new(p).is_err());
            }
        }

        #[test]
        fn depends_on_various_valid() {
            assert!(DependsOn::new("bd-abc123").is_ok());
            assert!(DependsOn::new("bd-ABCDEF").is_ok());
            assert!(DependsOn::new("bd-1").is_ok());
        }

        #[test]
        fn depends_on_various_invalid() {
            assert!(DependsOn::new("").is_err());
            assert!(DependsOn::new("abc123").is_err());
            assert!(DependsOn::new("bd-").is_err());
            assert!(DependsOn::new("bd-xyz").is_err());
        }

        #[test]
        fn labels_duplicate_detection() {
            assert!(Labels::new(vec!["bug".into(), "bug".into()]).is_err());
            assert!(Labels::new(vec!["a".into(), "b".into(), "a".into()]).is_err());
        }

        #[test]
        fn labels_unique_allowed() {
            assert!(Labels::new(vec!["a".into(), "b".into()]).is_ok());
        }
    }
}
