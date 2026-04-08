//! Metadata value objects: Labels, DependsOn, Priority, IssueType, WorkspaceName

use std::str::FromStr;

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
        for label in &labels {
            if label.trim().is_empty() {
                return Err(SessionError::InvalidIdentifier(
                    "Label cannot be empty or whitespace".into(),
                ));
            }
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

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    #[must_use]
    pub fn sorted(&self) -> Self {
        let mut sorted = self.0.clone();
        sorted.sort();
        Self(sorted)
    }
}

impl FromStr for Labels {
    type Err = SessionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self(vec![]));
        }
        let labels: Vec<String> = s
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect();
        Self::new(labels)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl FromStr for Priority {
    type Err = SessionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s.parse().map_err(|_| {
            SessionError::InvalidPriority(format!("Priority must be 0-4, got \"{s}\""))
        })?;
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

        // --- Range enforcement: P0-P4 valid ---

        #[test]
        fn priority_zero_valid() {
            let p = Priority::new(0).expect("P0 critical");
            assert_eq!(p.as_u8(), 0);
        }

        #[test]
        fn priority_one_valid() {
            let p = Priority::new(1).expect("P1 high");
            assert_eq!(p.as_u8(), 1);
        }

        #[test]
        fn priority_two_valid() {
            let p = Priority::new(2).expect("P2 medium");
            assert_eq!(p.as_u8(), 2);
        }

        #[test]
        fn priority_three_valid() {
            let p = Priority::new(3).expect("P3 low");
            assert_eq!(p.as_u8(), 3);
        }

        #[test]
        fn priority_four_valid() {
            let p = Priority::new(4).expect("P4 backlog");
            assert_eq!(p.as_u8(), 4);
        }

        #[test]
        fn priority_all_valid_levels() {
            for level in 0..=4u8 {
                let p = Priority::new(level).unwrap_or_else(|_| panic!("P{level} should be valid"));
                assert_eq!(p.as_u8(), level);
            }
        }

        // --- Out-of-range rejection ---

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
        fn priority_six_rejects() {
            let result = Priority::new(6);
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
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_boundary_values_above_range() {
            for invalid in [5u8, 10, 50, 100, 200, 255] {
                assert!(
                    Priority::new(invalid).is_err(),
                    "Priority {invalid} should be rejected"
                );
            }
        }

        // --- Ordering: Ord semantics (lower number = higher priority, sorts first) ---

        #[test]
        fn priority_ordering_lower_is_greater_priority() {
            let p0 = Priority::new(0).expect("P0");
            let p1 = Priority::new(1).expect("P1");
            let p2 = Priority::new(2).expect("P2");
            let p3 = Priority::new(3).expect("P3");
            let p4 = Priority::new(4).expect("P4");

            // P0 < P1 < P2 < P3 < P4 (natural u8 ordering)
            assert!(p0 < p1);
            assert!(p1 < p2);
            assert!(p2 < p3);
            assert!(p3 < p4);
        }

        #[test]
        fn priority_ordering_transitive() {
            let p0 = Priority::new(0).expect("P0");
            let p4 = Priority::new(4).expect("P4");
            // P0 < P4 via transitivity
            assert!(p0 < p4);
            assert!(p4 > p0);
        }

        #[test]
        fn priority_ordering_sort() {
            let mut priorities = vec![
                Priority::new(4).expect("P4"),
                Priority::new(0).expect("P0"),
                Priority::new(2).expect("P2"),
                Priority::new(1).expect("P1"),
                Priority::new(3).expect("P3"),
            ];
            priorities.sort();
            assert_eq!(
                priorities.iter().map(|p| p.as_u8()).collect::<Vec<_>>(),
                vec![0, 1, 2, 3, 4]
            );
        }

        #[test]
        fn priority_ordering_reverse_sort() {
            let mut priorities = vec![
                Priority::new(1).expect("P1"),
                Priority::new(3).expect("P3"),
                Priority::new(0).expect("P0"),
            ];
            priorities.sort_by(|a, b| b.cmp(a));
            assert_eq!(
                priorities.iter().map(|p| p.as_u8()).collect::<Vec<_>>(),
                vec![3, 1, 0]
            );
        }

        #[test]
        fn priority_cmp_min_max() {
            let p0 = Priority::new(0).expect("P0");
            let p4 = Priority::new(4).expect("P4");
            assert_eq!(p0.cmp(&p4), std::cmp::Ordering::Less);
            assert_eq!(p4.cmp(&p0), std::cmp::Ordering::Greater);
        }

        // --- Equality ---

        #[test]
        fn priority_equality_same_level() {
            let a = Priority::new(2).expect("P2");
            let b = Priority::new(2).expect("P2");
            assert_eq!(a, b);
            assert!(a <= b);
            assert!(a >= b);
        }

        #[test]
        fn priority_inequality_different_levels() {
            let p0 = Priority::new(0).expect("P0");
            let p4 = Priority::new(4).expect("P4");
            assert_ne!(p0, p4);
        }

        #[test]
        fn priority_hash_consistency() {
            use std::collections::HashSet;
            let p1 = Priority::new(1).expect("P1");
            let p2 = Priority::new(1).expect("P1");
            let mut set = HashSet::new();
            set.insert(p1);
            assert!(set.contains(&p2));
        }

        // --- Parsing from string ---

        #[test]
        fn priority_from_str_valid_all_levels() {
            for level in 0..=4u8 {
                let p = format!("{level}").parse::<Priority>().unwrap_or_else(|_| {
                    panic!("Parsing \"{level}\" should succeed")
                });
                assert_eq!(p.as_u8(), level);
            }
        }

        #[test]
        fn priority_from_str_zero() {
            let p = "0".parse::<Priority>().expect("P0 from string");
            assert_eq!(p.as_u8(), 0);
        }

        #[test]
        fn priority_from_str_four() {
            let p = "4".parse::<Priority>().expect("P4 from string");
            assert_eq!(p.as_u8(), 4);
        }

        #[test]
        fn priority_from_str_out_of_range() {
            let result = "5".parse::<Priority>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_from_str_invalid_text() {
            let result = "high".parse::<Priority>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_from_str_empty() {
            let result = "".parse::<Priority>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_from_str_negative() {
            let result = "-1".parse::<Priority>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        #[test]
        fn priority_from_str_large_number() {
            let result = "100".parse::<Priority>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        // --- Parsing from integer (TryFrom<u8>) ---

        #[test]
        fn priority_try_from_u8_all_valid() {
            for level in 0..=4u8 {
                let p = Priority::try_from(level)
                    .unwrap_or_else(|_| panic!("TryFrom u8 {level} should succeed"));
                assert_eq!(p.as_u8(), level);
            }
        }

        #[test]
        fn priority_try_from_u8_invalid() {
            let result = Priority::try_from(10);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidPriority(_)
            ));
        }

        // --- Display / Conversion ---

        #[test]
        fn priority_display() {
            let p = Priority::new(2).expect("valid");
            assert_eq!(format!("{p}"), "2");
        }

        #[test]
        fn priority_display_all_levels() {
            for level in 0..=4u8 {
                let p = Priority::new(level).expect("valid");
                assert_eq!(format!("{p}"), format!("{level}"));
            }
        }

        #[test]
        fn priority_into_inner() {
            let p = Priority::new(1).expect("valid");
            assert_eq!(p.into_inner(), 1);
        }

        #[test]
        fn priority_as_u8_all_levels() {
            for level in 0..=4u8 {
                let p = Priority::new(level).expect("valid");
                assert_eq!(p.as_u8(), level);
            }
        }

        // --- Copy semantics ---

        #[test]
        fn priority_copy_semantics() {
            let p1 = Priority::new(2).expect("valid");
            let p2 = p1; // Copy, not move
            assert_eq!(p1, p2); // p1 still valid after copy
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
        fn labels_empty_string_item_rejected() {
            let result = Labels::new(vec!["".to_string()]);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn labels_whitespace_only_item_rejected() {
            let result = Labels::new(vec!["   ".to_string()]);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }
    }

    // =========================================================================
    // Labels Comprehensive Tests — parsing, containment, sorting, equality
    // =========================================================================

    mod labels_comprehensive_tests {
        use super::*;

        // --- FromStr: comma-separated parsing ---

        #[test]
        fn labels_from_str_simple() {
            let labels: Labels = "bug,feature,urgent".parse().expect("valid");
            assert_eq!(
                labels.as_slice(),
                &["bug".to_string(), "feature".to_string(), "urgent".to_string()]
            );
        }

        #[test]
        fn labels_from_str_single() {
            let labels: Labels = "bug".parse().expect("valid");
            assert_eq!(labels.as_slice(), &["bug".to_string()]);
        }

        #[test]
        fn labels_from_str_empty_string() {
            let labels: Labels = "".parse().expect("empty is valid");
            assert!(labels.is_empty());
        }

        #[test]
        fn labels_from_str_trims_whitespace() {
            let labels: Labels = "  bug  ,  feature  ,  urgent  ".parse().expect("valid");
            assert_eq!(
                labels.as_slice(),
                &["bug".to_string(), "feature".to_string(), "urgent".to_string()]
            );
        }

        #[test]
        fn labels_from_str_with_spaces_between() {
            let labels: Labels = "bug, feature, urgent".parse().expect("valid");
            assert_eq!(
                labels.as_slice(),
                &["bug".to_string(), "feature".to_string(), "urgent".to_string()]
            );
        }

        #[test]
        fn labels_from_str_rejects_duplicates() {
            let result = "bug,feature,bug".parse::<Labels>();
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::InvalidIdentifier(_)
            ));
        }

        #[test]
        fn labels_from_str_skips_empty_segments() {
            // "a,,b" splits to ["a", "", "b"] — empty filtered out → ["a", "b"]
            let labels: Labels = "a,,b".parse().expect("empty segments filtered");
            assert_eq!(
                labels.as_slice(),
                &["a".to_string(), "b".to_string()]
            );
        }

        #[test]
        fn labels_from_str_trailing_comma() {
            let labels: Labels = "a,b,".parse().expect("trailing comma ok");
            assert_eq!(labels.as_slice(), &["a".to_string(), "b".to_string()]);
        }

        #[test]
        fn labels_from_str_leading_comma() {
            let labels: Labels = ",a,b".parse().expect("leading comma ok");
            assert_eq!(labels.as_slice(), &["a".to_string(), "b".to_string()]);
        }

        #[test]
        fn labels_from_str_only_commas() {
            let labels: Labels = ",,,".parse().expect("only commas = empty");
            assert!(labels.is_empty());
        }

        #[test]
        fn labels_from_str_whitespace_only_between_commas() {
            let labels: Labels = "a,   ,b".parse().expect("whitespace segments filtered");
            assert_eq!(labels.as_slice(), &["a".to_string(), "b".to_string()]);
        }

        #[test]
        fn labels_from_str_exceeds_max() {
            let input: String = (0..=Labels::MAX_LABELS)
                .map(|i| format!("label-{i}"))
                .collect::<Vec<_>>()
                .join(",");
            let result = input.parse::<Labels>();
            assert!(result.is_err());
        }

        // --- Empty label rejection ---

        #[test]
        fn labels_rejects_empty_string_in_list() {
            let result = Labels::new(vec!["bug".to_string(), "".to_string()]);
            assert!(result.is_err());
        }

        #[test]
        fn labels_rejects_whitespace_only_in_list() {
            let result = Labels::new(vec!["bug".to_string(), "   ".to_string()]);
            assert!(result.is_err());
        }

        #[test]
        fn labels_rejects_tab_only_in_list() {
            let result = Labels::new(vec!["bug".to_string(), "\t".to_string()]);
            assert!(result.is_err());
        }

        #[test]
        fn labels_accepts_label_with_internal_spaces() {
            let labels = Labels::new(vec!["high priority".to_string()]).expect("valid");
            assert_eq!(labels.as_slice(), &["high priority".to_string()]);
        }

        // --- Containment ---

        #[test]
        fn labels_contains_existing() {
            let labels = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            assert!(labels.contains("bug"));
            assert!(labels.contains("feature"));
        }

        #[test]
        fn labels_contains_missing() {
            let labels = Labels::new(vec!["bug".to_string()]).expect("valid");
            assert!(!labels.contains("feature"));
        }

        #[test]
        fn labels_contains_empty_string() {
            let labels = Labels::new(vec!["bug".to_string()]).expect("valid");
            assert!(!labels.contains(""));
        }

        #[test]
        fn labels_contains_case_sensitive() {
            let labels = Labels::new(vec!["Bug".to_string()]).expect("valid");
            assert!(labels.contains("Bug"));
            assert!(!labels.contains("bug"));
            assert!(!labels.contains("BUG"));
        }

        #[test]
        fn labels_contains_empty_labels() {
            let labels = Labels::new(vec![]).expect("valid");
            assert!(!labels.contains("anything"));
        }

        // --- Equality ---

        #[test]
        fn labels_equal_same_order() {
            let a = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            let b = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            assert_eq!(a, b);
        }

        #[test]
        fn labels_not_equal_different_order() {
            let a = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            let b = Labels::new(vec!["feature".to_string(), "bug".to_string()]).expect("valid");
            assert_ne!(a, b);
        }

        #[test]
        fn labels_not_equal_different_labels() {
            let a = Labels::new(vec!["bug".to_string()]).expect("valid");
            let b = Labels::new(vec!["feature".to_string()]).expect("valid");
            assert_ne!(a, b);
        }

        #[test]
        fn labels_equal_both_empty() {
            let a = Labels::new(vec![]).expect("valid");
            let b = Labels::new(vec![]).expect("valid");
            assert_eq!(a, b);
        }

        #[test]
        fn labels_not_equal_different_count() {
            let a = Labels::new(vec!["bug".to_string()]).expect("valid");
            let b = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            assert_ne!(a, b);
        }

        #[test]
        fn labels_hash_consistency() {
            use std::collections::HashSet;
            let a = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            let b = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
        }

        // --- Sorting ---

        #[test]
        fn labels_sorted_alphabetical() {
            let labels = Labels::new(vec![
                "zebra".to_string(),
                "apple".to_string(),
                "mango".to_string(),
            ])
            .expect("valid");
            let sorted = labels.sorted();
            assert_eq!(
                sorted.as_slice(),
                &["apple".to_string(), "mango".to_string(), "zebra".to_string()]
            );
        }

        #[test]
        fn labels_sorted_does_not_mutate_original() {
            let labels = Labels::new(vec![
                "zebra".to_string(),
                "apple".to_string(),
            ])
            .expect("valid");
            let _sorted = labels.sorted();
            // Original preserves insertion order
            assert_eq!(
                labels.as_slice(),
                &["zebra".to_string(), "apple".to_string()]
            );
        }

        #[test]
        fn labels_sorted_empty() {
            let labels = Labels::new(vec![]).expect("valid");
            let sorted = labels.sorted();
            assert!(sorted.is_empty());
        }

        #[test]
        fn labels_sorted_single() {
            let labels = Labels::new(vec!["only".to_string()]).expect("valid");
            let sorted = labels.sorted();
            assert_eq!(sorted.as_slice(), &["only".to_string()]);
        }

        #[test]
        fn labels_sorted_already_sorted() {
            let labels = Labels::new(vec![
                "apple".to_string(),
                "banana".to_string(),
                "cherry".to_string(),
            ])
            .expect("valid");
            let sorted = labels.sorted();
            assert_eq!(
                sorted.as_slice(),
                &["apple".to_string(), "banana".to_string(), "cherry".to_string()]
            );
        }

        // --- len / is_empty ---

        #[test]
        fn labels_len() {
            let labels = Labels::new(vec!["a".to_string(), "b".to_string()]).expect("valid");
            assert_eq!(labels.len(), 2);
        }

        #[test]
        fn labels_len_empty() {
            let labels = Labels::new(vec![]).expect("valid");
            assert_eq!(labels.len(), 0);
        }

        #[test]
        fn labels_is_empty_true() {
            let labels = Labels::new(vec![]).expect("valid");
            assert!(labels.is_empty());
        }

        #[test]
        fn labels_is_empty_false() {
            let labels = Labels::new(vec!["a".to_string()]).expect("valid");
            assert!(!labels.is_empty());
        }

        // --- Clone ---

        #[test]
        fn labels_clone_equal() {
            let labels = Labels::new(vec!["bug".to_string(), "feature".to_string()]).expect("valid");
            let cloned = labels.clone();
            assert_eq!(labels, cloned);
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
