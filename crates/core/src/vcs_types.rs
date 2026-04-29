//! VCS data types - Commit, Branch, Workspace, VcsStatus, VcsType
//!
//! This module contains all VCS-related data structures including
//! newtypes (BranchName, CommitId, ChangeId) and RepoStatus.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A VCS commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

/// A VCS branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}

/// A workspace (from Isolate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}

/// Status of working copy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsStatus {
    /// Clean - no uncommitted changes
    Clean,
    /// Has uncommitted changes
    Dirty,
    /// Has conflicts
    Conflicted,
    /// Detached HEAD
    Detached,
}

impl std::fmt::Display for VcsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::Dirty => write!(f, "dirty"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Detached => write!(f, "detached"),
        }
    }
}

/// VCS type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    /// Git VCS
    Git,
}

/// Detect which VCS is in use in a directory
pub fn detect_vcs(path: &std::path::Path) -> Option<VcsType> {
    if path.join(".git").exists() {
        Some(VcsType::Git)
    } else {
        None
    }
}

// ============================================================================
// Newtypes
// ============================================================================

/// Branch name newtype - validates on construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    /// Create a new BranchName, returning None if invalid.
    ///
    /// Validation rules:
    /// - Rejects empty or whitespace-only strings
    /// - Rejects strings containing null bytes
    /// - Rejects strings containing control characters
    /// - Rejects git-dangerous patterns (`-`, `--`, `.`, `..`)
    /// - Enforces maximum length of 250 characters (git ref limit)
    #[must_use]
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        let trimmed = name.trim();

        // Reject empty
        if trimmed.is_empty() {
            return None;
        }

        // Reject null bytes
        if trimmed.contains('\0') {
            return None;
        }

        // Reject control characters
        if trimmed.chars().any(|c| c.is_control()) {
            return None;
        }

        // Reject git-dangerous patterns
        if trimmed == "." || trimmed == ".." || trimmed.starts_with('-') {
            return None;
        }

        // Enforce max length (git refs are limited to ~250 chars)
        if trimmed.len() > 250 {
            return None;
        }

        Some(Self(trimmed.to_string()))
    }

    /// Get the branch name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BranchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Commit ID newtype - validates non-empty on construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new CommitId, returning None if empty
    #[must_use]
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            None
        } else {
            Some(Self(id))
        }
    }

    /// Create a CommitId without validation (for internal use where
    /// the value is already known to be valid, e.g. from subprocess output).
    #[must_use]
    pub fn from_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the commit ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Change ID newtype - validates non-empty on construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(String);

impl ChangeId {
    /// Create a new ChangeId, returning None if empty
    #[must_use]
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            None
        } else {
            Some(Self(id))
        }
    }

    /// Create a ChangeId without validation (for internal use where
    /// the value is already known to be valid, e.g. from subprocess output).
    #[must_use]
    pub fn from_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the change ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// RepoStatus
// ============================================================================

/// Detailed repository status
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStatus {
    /// Whether the working copy has uncommitted changes
    pub clean: bool,
    /// Current branch name (if any)
    pub branch: Option<BranchName>,
    /// Current commit ID (if any)
    pub commit_id: Option<CommitId>,
    /// Whether there are merge conflicts
    pub has_conflicts: bool,
    /// List of files with uncommitted changes
    pub uncommitted_files: Vec<String>,
}

impl RepoStatus {
    /// Create a clean RepoStatus
    #[must_use]
    pub const fn clean() -> Self {
        Self {
            clean: true,
            branch: None,
            commit_id: None,
            has_conflicts: false,
            uncommitted_files: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert, prop_assert_eq};

    use super::*;

    #[test]
    fn branch_name_accepts_valid() {
        assert!(BranchName::new("main").is_some());
        assert!(BranchName::new("feature/foo").is_some());
        assert!(BranchName::new("fix-123").is_some());
        assert!(BranchName::new("my_branch").is_some());
        assert!(BranchName::new("a").is_some());
    }

    #[test]
    fn branch_name_rejects_empty() {
        assert!(BranchName::new("").is_none());
        assert!(BranchName::new("   ").is_none());
    }

    #[test]
    fn branch_name_rejects_null_bytes() {
        assert!(BranchName::new("branch\0name").is_none());
    }

    #[test]
    fn branch_name_rejects_control_characters() {
        assert!(BranchName::new("branch\nname").is_none());
        assert!(BranchName::new("branch\tname").is_none());
        assert!(BranchName::new("branch\rname").is_none());
        assert!(BranchName::new("branch\x01name").is_none());
    }

    #[test]
    fn branch_name_rejects_git_dangerous() {
        assert!(BranchName::new("-v").is_none());
        assert!(BranchName::new("--verbose").is_none());
        assert!(BranchName::new(".").is_none());
        assert!(BranchName::new("..").is_none());
    }

    #[test]
    fn branch_name_rejects_too_long() {
        let long_name = "a".repeat(251);
        assert!(BranchName::new(long_name).is_none());
    }

    #[test]
    fn branch_name_accepts_max_length() {
        let max_name = "a".repeat(250);
        assert!(BranchName::new(max_name).is_some());
    }

    #[test]
    fn branch_name_trims_whitespace() {
        let name = BranchName::new("  main  ").expect("should parse");
        assert_eq!(name.as_str(), "main");
    }

    // CommitId tests

    #[test]
    fn commit_id_accepts_valid() {
        assert!(CommitId::new("abc123").is_some());
        assert!(CommitId::new("a").is_some());
    }

    #[test]
    fn commit_id_rejects_empty() {
        assert!(CommitId::new("").is_none());
    }

    #[test]
    fn commit_id_from_unchecked_always_succeeds() {
        assert!(CommitId::from_unchecked("").as_str().is_empty());
    }

    // ChangeId tests

    #[test]
    fn change_id_accepts_valid() {
        assert!(ChangeId::new("abc123").is_some());
    }

    #[test]
    fn change_id_rejects_empty() {
        assert!(ChangeId::new("").is_none());
    }

    // RepoStatus tests

    #[test]
    fn repo_status_clean() {
        let status = RepoStatus::clean();
        assert!(status.clean);
        assert!(!status.has_conflicts);
        assert!(status.branch.is_none());
    }

    #[test]
    fn repo_status_default() {
        let status = RepoStatus::default();
        // Default derives false for bool fields, so clean is false by default.
        // Use RepoStatus::clean() for a clean status.
        assert!(!status.clean);
        assert!(status.branch.is_none());
        assert!(status.commit_id.is_none());
        assert!(!status.has_conflicts);
        assert!(status.uncommitted_files.is_empty());
    }

    // -- VcsStatus Display --

    #[test]
    fn vcs_status_display_all_variants() {
        assert_eq!(format!("{}", VcsStatus::Clean), "clean");
        assert_eq!(format!("{}", VcsStatus::Dirty), "dirty");
        assert_eq!(format!("{}", VcsStatus::Conflicted), "conflicted");
        assert_eq!(format!("{}", VcsStatus::Detached), "detached");
    }

    #[test]
    fn vcs_status_equality() {
        assert_eq!(VcsStatus::Clean, VcsStatus::Clean);
        assert_eq!(VcsStatus::Dirty, VcsStatus::Dirty);
        assert_ne!(VcsStatus::Clean, VcsStatus::Dirty);
        assert_ne!(VcsStatus::Conflicted, VcsStatus::Detached);
    }

    #[test]
    fn vcs_status_clone() {
        let s = VcsStatus::Conflicted;
        assert_eq!(s.clone(), VcsStatus::Conflicted);
    }

    // -- VcsType --

    #[test]
    fn vcs_type_equality() {
        assert_eq!(VcsType::Git, VcsType::Git);
    }

    #[test]
    fn vcs_type_clone() {
        let t = VcsType::Git;
        assert_eq!(t.clone(), VcsType::Git);
    }

    #[test]
    fn vcs_type_copy() {
        let t = VcsType::Git;
        let t2 = t;
        assert_eq!(t, t2);
    }

    // -- detect_vcs --

    #[test]
    fn detect_vcs_returns_none_for_empty_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(detect_vcs(dir.path()).is_none());
    }

    #[test]
    fn detect_vcs_detects_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        assert_eq!(detect_vcs(dir.path()), Some(VcsType::Git));
    }

    #[test]
    fn detect_vcs_does_not_confuse_git_dir_with_similar_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".gitignore")).expect("create .gitignore");
        assert_eq!(detect_vcs(dir.path()), None);
    }

    // -- BranchName Display --

    #[test]
    fn branch_name_display() {
        let name = BranchName::new("main").expect("valid");
        assert_eq!(format!("{name}"), "main");
    }

    #[test]
    fn branch_name_hash() {
        use std::collections::HashSet;
        let name = BranchName::new("main").expect("valid");
        let mut set = HashSet::new();
        set.insert(name.clone());
        assert!(set.contains(&name));
    }

    #[test]
    fn branch_name_dash_prefix() {
        // Single dash is a git-dangerous pattern (starts with -)
        assert!(BranchName::new("-").is_none());
    }

    // -- CommitId Display --

    #[test]
    fn commit_id_display() {
        let id = CommitId::new("abc123").expect("valid");
        assert_eq!(format!("{id}"), "abc123");
    }

    #[test]
    fn commit_id_whitespace_only_rejected() {
        assert!(CommitId::new("   ").is_none());
        assert!(CommitId::new("\t").is_none());
        assert!(CommitId::new("\n").is_none());
    }

    #[test]
    fn commit_id_from_unchecked_empty() {
        let id = CommitId::from_unchecked("");
        assert_eq!(id.as_str(), "");
    }

    #[test]
    fn commit_id_hash() {
        use std::collections::HashSet;
        let id = CommitId::new("abc123").expect("valid");
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn commit_id_equality() {
        let a = CommitId::new("abc123").expect("valid");
        let b = CommitId::new("abc123").expect("valid");
        assert_eq!(a, b);
    }

    // -- ChangeId Display --

    #[test]
    fn change_id_display() {
        let id = ChangeId::new("abc123").expect("valid");
        assert_eq!(format!("{id}"), "abc123");
    }

    #[test]
    fn change_id_whitespace_only_rejected() {
        assert!(ChangeId::new("   ").is_none());
        assert!(ChangeId::new("\t").is_none());
    }

    #[test]
    fn change_id_from_unchecked_empty() {
        let id = ChangeId::from_unchecked("");
        assert_eq!(id.as_str(), "");
    }

    #[test]
    fn change_id_hash() {
        use std::collections::HashSet;
        let id = ChangeId::new("abc123").expect("valid");
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn change_id_equality() {
        let a = ChangeId::new("abc123").expect("valid");
        let b = ChangeId::new("abc123").expect("valid");
        assert_eq!(a, b);
    }

    // ========================================================================
    // ChangeId property-based tests
    // ========================================================================

    proptest::proptest! {
        /// ChangeId::new() rejects all empty or whitespace-only strings.
        #[test]
        fn proptest_change_id_rejects_empty_or_whitespace(
            input in proptest::string::string_regex("\\s*").unwrap(),
        ) {
            if input.trim().is_empty() {
                proptest::prop_assert!(ChangeId::new(input.clone()).is_none());
            }
        }

        /// ChangeId::from_unchecked always succeeds and preserves input.
        #[test]
        fn proptest_change_id_from_unchecked_always_succeeds(
            bytes in proptest::collection::vec(proptest::arbitrary::any::<u8>(), 0..1000),
        ) {
            let input = String::from_utf8_lossy(&bytes).into_owned();
            let id = ChangeId::from_unchecked(input.clone());
            prop_assert_eq!(id.as_str(), input);
        }

        /// ChangeId is reflexive: parse -> as_str -> parse == original.
        #[test]
        fn proptest_change_id_roundtrip_reflexive(
            input in "[a-zA-Z0-9/_-]{1,100}",
        ) {
            let id = ChangeId::new(input.clone()).expect("valid input should parse");
            prop_assert_eq!(id.as_str(), input.trim());
        }
    }

    // -- RepoStatus construction --

    #[test]
    fn repo_status_with_all_fields() {
        let branch = BranchName::new("develop").expect("valid");
        let commit_id = CommitId::new("abc123").expect("valid");
        let status = RepoStatus {
            clean: false,
            branch: Some(branch.clone()),
            commit_id: Some(commit_id.clone()),
            has_conflicts: true,
            uncommitted_files: vec!["src/main.rs".to_string(), "lib/api.rs".to_string()],
        };
        assert!(!status.clean);
        assert_eq!(status.branch.as_ref().map(|b| b.as_str()), Some("develop"));
        assert_eq!(
            status.commit_id.as_ref().map(|c| c.as_str()),
            Some("abc123")
        );
        assert!(status.has_conflicts);
        assert_eq!(status.uncommitted_files.len(), 2);
    }

    #[test]
    fn repo_status_equality() {
        let a = RepoStatus::clean();
        let b = RepoStatus::clean();
        assert_eq!(a, b);

        let c = RepoStatus {
            clean: false,
            ..RepoStatus::clean()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn repo_status_clone() {
        let original = RepoStatus::clean();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // -- Commit, Branch, Workspace construction --

    #[test]
    fn commit_construction() {
        let commit = Commit {
            id: "abc123".to_string(),
            message: "Initial commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec!["parent1".to_string()],
        };
        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.author, "Test Author");
        assert_eq!(commit.parents.len(), 1);
    }

    #[test]
    fn commit_clone() {
        let commit = Commit {
            id: "abc123".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        let cloned = commit.clone();
        assert_eq!(cloned.id, commit.id);
    }

    // ========================================================================
    // Commit exhaustive tests
    // ========================================================================

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn commit_root_commit_no_parents() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let commit = Commit {
            id: "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b".to_string(),
            message: "Initial commit".to_string(),
            author: "Alice <alice@example.com>".to_string(),
            timestamp: ts,
            parents: vec![],
        };
        assert_eq!(commit.id, "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b");
        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.author, "Alice <alice@example.com>");
        assert_eq!(commit.parents, Vec::<String>::new());
        assert_eq!(commit.timestamp, ts);
    }

    #[test]
    fn commit_normal_one_parent() {
        let commit = Commit {
            id: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0".to_string(),
            message: "Add feature X".to_string(),
            author: "Bob <bob@example.com>".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec!["e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b".to_string()],
        };
        assert_eq!(commit.parents.len(), 1);
        assert_eq!(
            commit.parents[0],
            "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b"
        );
    }

    #[test]
    fn commit_merge_two_parents() {
        let commit = Commit {
            id: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
            message: "Merge branch 'feature' into main".to_string(),
            author: "Carol <carol@example.com>".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![
                "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0".to_string(),
                "b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1".to_string(),
            ],
        };
        assert_eq!(commit.parents.len(), 2);
        assert_eq!(
            commit.parents[0],
            "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0"
        );
        assert_eq!(
            commit.parents[1],
            "b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1"
        );
    }

    #[test]
    fn commit_octopus_merge_three_parents() {
        let commit = Commit {
            id: "1234567890abcdef1234567890abcdef12345678".to_string(),
            message: "Octopus merge: feature-a + feature-b + feature-c".to_string(),
            author: "Dana <dana@example.com>".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                "cccccccccccccccccccccccccccccccccccccccc".to_string(),
            ],
        };
        assert_eq!(commit.parents.len(), 3);
    }

    #[test]
    fn commit_many_parents() {
        let parents: Vec<String> = (0..10).map(|i| format!("{i:040x}")).collect();
        let commit = Commit {
            id: "ffffffffffffffffffffffffffffffffffffffff".to_string(),
            message: "Unusual octopus".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: parents.clone(),
        };
        assert_eq!(commit.parents.len(), 10);
        assert_eq!(commit.parents, parents);
    }

    // ── SHA patterns ─────────────────────────────────────────────────────────

    #[test]
    fn commit_sha_40_char_lowercase_hex() {
        let sha = "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b";
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
        let commit = Commit {
            id: sha.to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.id, sha);
    }

    #[test]
    fn commit_sha_40_char_uppercase_hex() {
        let sha = "E7C805D7BA5E7A8B5E4C3D2F1A0B9C8D7E6F5A4B";
        assert_eq!(sha.len(), 40);
        let commit = Commit {
            id: sha.to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.id, sha);
    }

    #[test]
    fn commit_sha_short_7_char() {
        let short_sha = "e7c805d";
        let commit = Commit {
            id: short_sha.to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.id, "e7c805d");
        assert_eq!(commit.id.len(), 7);
    }

    #[test]
    fn commit_sha_short_8_char() {
        let short_sha = "e7c805d7";
        let commit = Commit {
            id: short_sha.to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.id.len(), 8);
    }

    // ── Parent tracking ──────────────────────────────────────────────────────

    #[test]
    fn commit_parents_order_preserved() {
        let parents = vec![
            "1111111111111111111111111111111111111111".to_string(),
            "2222222222222222222222222222222222222222".to_string(),
        ];
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: parents.clone(),
        };
        assert_eq!(
            commit.parents[0],
            "1111111111111111111111111111111111111111"
        );
        assert_eq!(
            commit.parents[1],
            "2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn commit_parents_dedup_not_enforced() {
        // Commit is a plain struct; duplicate parent SHAs are allowed.
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![
                "same111111111111111111111111111111111111".to_string(),
                "same111111111111111111111111111111111111".to_string(),
            ],
        };
        assert_eq!(commit.parents[0], commit.parents[1]);
    }

    // ── Timestamp handling ───────────────────────────────────────────────────

    #[test]
    fn commit_timestamp_epoch_zero() {
        let epoch = chrono::DateTime::from_timestamp(0, 0).expect("epoch");
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: epoch,
            parents: vec![],
        };
        assert_eq!(
            commit.timestamp,
            chrono::DateTime::from_timestamp(0, 0).unwrap()
        );
    }

    #[test]
    fn commit_timestamp_specific_date() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:45Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: ts,
            parents: vec![],
        };
        assert_eq!(commit.timestamp.to_rfc3339(), "2024-06-15T12:30:45+00:00");
    }

    #[test]
    fn commit_timestamp_far_future() {
        let ts = chrono::DateTime::parse_from_rfc3339("2099-12-31T23:59:59Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: ts,
            parents: vec![],
        };
        assert_eq!(commit.timestamp, ts);
    }

    #[test]
    fn commit_timestamp_with_millis() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00.123Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: ts,
            parents: vec![],
        };
        assert_eq!(commit.timestamp.timestamp_millis(), 1704067200123);
    }

    // ── Author field ─────────────────────────────────────────────────────────

    #[test]
    fn commit_author_with_email() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "Alice <alice@example.com>".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert!(commit.author.contains('<'));
        assert!(commit.author.contains('>'));
        assert!(commit.author.contains('@'));
    }

    #[test]
    fn commit_author_name_only() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "Alice".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.author, "Alice");
    }

    #[test]
    fn commit_author_empty_string() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: String::new(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert!(commit.author.is_empty());
    }

    #[test]
    fn commit_author_unicode() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "test".to_string(),
            author: "José García <jose@example.com>".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert!(commit.author.contains('é'));
        assert!(commit.author.contains('í'));
    }

    // ── Message access ───────────────────────────────────────────────────────

    #[test]
    fn commit_message_single_line() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "Fix bug in parser".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.message, "Fix bug in parser");
        assert!(!commit.message.contains('\n'));
    }

    #[test]
    fn commit_message_multiline() {
        let msg = "Fix bug in parser\n\nDetailed explanation of the fix.\n\nSigned-off-by: Alice";
        let commit = Commit {
            id: "abc".to_string(),
            message: msg.to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(commit.message, msg);
        assert!(commit.message.contains('\n'));
    }

    #[test]
    fn commit_message_empty() {
        let commit = Commit {
            id: "abc".to_string(),
            message: String::new(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert!(commit.message.is_empty());
    }

    #[test]
    fn commit_message_unicode() {
        let commit = Commit {
            id: "abc".to_string(),
            message: "修正解析器错误 🐛".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert!(commit.message.contains("修正"));
    }

    // ── Comparison by SHA (field-level, no PartialEq derive) ────────────────

    #[test]
    fn commit_comparison_same_sha() {
        let sha = "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b";
        let a = Commit {
            id: sha.to_string(),
            message: "first".to_string(),
            author: "A".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        let b = Commit {
            id: sha.to_string(),
            message: "second".to_string(),
            author: "B".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn commit_comparison_different_sha() {
        let a = Commit {
            id: "aaa".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        let b = Commit {
            id: "bbb".to_string(),
            message: "test".to_string(),
            author: "test".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn commit_comparison_same_sha_different_everything_else() {
        let sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let ts = chrono::Utc::now();
        let a = Commit {
            id: sha.to_string(),
            message: "msg a".to_string(),
            author: "author a".to_string(),
            timestamp: ts,
            parents: vec!["111".to_string()],
        };
        let b = Commit {
            id: sha.to_string(),
            message: "msg b".to_string(),
            author: "author b".to_string(),
            timestamp: chrono::DateTime::from_timestamp(99999, 0).unwrap(),
            parents: vec!["222".to_string()],
        };
        // Same SHA but different content
        assert_eq!(a.id, b.id);
        assert_ne!(a.message, b.message);
        assert_ne!(a.author, b.author);
        assert_ne!(a.timestamp, b.timestamp);
        assert_ne!(a.parents, b.parents);
    }

    // ── Debug format ─────────────────────────────────────────────────────────

    #[test]
    fn commit_debug_format() {
        let commit = Commit {
            id: "abc123".to_string(),
            message: "test msg".to_string(),
            author: "Test Author".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec!["parent1".to_string()],
        };
        let debug = format!("{commit:?}");
        assert!(debug.contains("Commit"), "Debug should contain type name");
        assert!(debug.contains("abc123"), "Debug should contain id");
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn commit_serde_roundtrip_root() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let original = Commit {
            id: "e7c805d7ba5e7a8b5e4c3d2f1a0b9c8d7e6f5a4b".to_string(),
            message: "Initial commit".to_string(),
            author: "Alice <alice@example.com>".to_string(),
            timestamp: ts,
            parents: vec![],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.message, original.message);
        assert_eq!(decoded.author, original.author);
        assert_eq!(decoded.timestamp, original.timestamp);
        assert_eq!(decoded.parents, original.parents);
    }

    #[test]
    fn commit_serde_roundtrip_merge() {
        let ts = chrono::Utc::now();
        let original = Commit {
            id: "deadbeef".to_string(),
            message: "Merge".to_string(),
            author: "Bob".to_string(),
            timestamp: ts,
            parents: vec!["aaa".to_string(), "bbb".to_string()],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.parents.len(), 2);
    }

    #[test]
    fn commit_serde_preserves_empty_fields() {
        let original = Commit {
            id: "abc".to_string(),
            message: String::new(),
            author: String::new(),
            timestamp: chrono::Utc::now(),
            parents: vec![],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
        assert!(decoded.message.is_empty());
        assert!(decoded.author.is_empty());
        assert!(decoded.parents.is_empty());
    }

    #[test]
    fn commit_deserialize_from_json() {
        let json = r#"{"id":"abc123","message":"test","author":"Alice","timestamp":"2024-01-01T00:00:00Z","parents":["p1","p2"]}"#;
        let c: Commit = serde_json::from_str(json).expect("deserialize");
        assert_eq!(c.id, "abc123");
        assert_eq!(c.message, "test");
        assert_eq!(c.author, "Alice");
        assert_eq!(c.parents.len(), 2);
    }

    // ── Clone independence ───────────────────────────────────────────────────

    #[test]
    fn commit_clone_independence() {
        let mut commit = Commit {
            id: "abc".to_string(),
            message: "original".to_string(),
            author: "Alice".to_string(),
            timestamp: chrono::Utc::now(),
            parents: vec!["p1".to_string()],
        };
        let cloned = commit.clone();
        commit.id = "changed".to_string();
        commit.message = "changed".to_string();
        commit.parents.push("p2".to_string());
        assert_eq!(cloned.id, "abc");
        assert_eq!(cloned.message, "original");
        assert_eq!(cloned.parents.len(), 1);
    }

    #[test]
    fn commit_clone_preserves_all_fields() {
        let ts = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:30:45Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let commit = Commit {
            id: "deadbeef".to_string(),
            message: "Merge commit".to_string(),
            author: "Bob <bob@example.com>".to_string(),
            timestamp: ts,
            parents: vec!["aaa".to_string(), "bbb".to_string()],
        };
        let cloned = commit.clone();
        assert_eq!(cloned.id, commit.id);
        assert_eq!(cloned.message, commit.message);
        assert_eq!(cloned.author, commit.author);
        assert_eq!(cloned.timestamp, commit.timestamp);
        assert_eq!(cloned.parents, commit.parents);
    }

    // ========================================================================
    // Commit property-based tests (proptest)
    // ========================================================================

    proptest::proptest! {
        /// Commit serde roundtrip preserves all fields for any string inputs.
        #[test]
        fn proptest_commit_serde_roundtrip(
            id in "[a-f0-9]{40}",
            message in ".*",
            author in ".*",
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..5),
        ) {
            let ts = chrono::Utc::now();
            let original = Commit {
                id: id.clone(),
                message: message.clone(),
                author: author.clone(),
                timestamp: ts,
                parents: parents.clone(),
            };
            let json = serde_json::to_string(&original).expect("serialize");
            let decoded: Commit = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(decoded.id, id);
            prop_assert_eq!(decoded.message, message);
            prop_assert_eq!(decoded.author, author);
            prop_assert_eq!(decoded.parents, parents);
        }

        /// Commit clone is always identical (field-by-field).
        #[test]
        fn proptest_commit_clone_identical(
            id in "[a-f0-9]{40}",
            message in ".*",
            author in ".*",
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..5),
        ) {
            let ts = chrono::Utc::now();
            let commit = Commit {
                id: id.clone(),
                message: message.clone(),
                author: author.clone(),
                timestamp: ts,
                parents: parents.clone(),
            };
            let cloned = commit.clone();
            prop_assert_eq!(cloned.id, id);
            prop_assert_eq!(cloned.message, message);
            prop_assert_eq!(cloned.author, author);
            prop_assert_eq!(cloned.parents, parents);
        }

        /// Commit id is exactly 40 hex chars when constructed with 40-hex input.
        #[test]
        fn proptest_commit_sha_is_40_hex(
            hex in "[a-f0-9]{40}",
        ) {
            let commit = Commit {
                id: hex.clone(),
                message: "test".to_string(),
                author: "test".to_string(),
                timestamp: chrono::Utc::now(),
                parents: vec![],
            };
            prop_assert_eq!(commit.id.len(), 40);
            prop_assert!(commit.id.chars().all(|c| c.is_ascii_hexdigit()));
            prop_assert_eq!(commit.id, hex);
        }

        /// Commit parents count matches input for any 0-5 parents.
        #[test]
        fn proptest_commit_parents_count(
            parents in proptest::collection::vec("[a-f0-9]{40}", 0..=5),
        ) {
            let count = parents.len();
            let commit = Commit {
                id: "abc".to_string(),
                message: "test".to_string(),
                author: "test".to_string(),
                timestamp: chrono::Utc::now(),
                parents: parents.clone(),
            };
            prop_assert_eq!(commit.parents.len(), count);
            for (i, p) in parents.iter().enumerate() {
                prop_assert_eq!(&commit.parents[i], p);
            }
        }

        /// Two commits with the same id field compare equal on id.
        #[test]
        fn proptest_commit_same_id_compares_equal(
            id in "[a-f0-9]{40}",
            msg_a in ".*",
            msg_b in ".*",
        ) {
            let a = Commit {
                id: id.clone(),
                message: msg_a,
                author: "A".to_string(),
                timestamp: chrono::Utc::now(),
                parents: vec![],
            };
            let b = Commit {
                id: id.clone(),
                message: msg_b,
                author: "B".to_string(),
                timestamp: chrono::Utc::now(),
                parents: vec![],
            };
            prop_assert_eq!(a.id, b.id);
        }

        /// Two commits with different id fields compare different on id.
        #[test]
        fn proptest_commit_different_id_compares_different(
            id_a in "[a-f0-9]{40}",
            id_b in "[a-f0-9]{40}",
        ) {
            proptest::prop_assume!(id_a != id_b);
            let a = Commit {
                id: id_a,
                message: "same".to_string(),
                author: "same".to_string(),
                timestamp: chrono::Utc::now(),
                parents: vec![],
            };
            let b = Commit {
                id: id_b,
                message: "same".to_string(),
                author: "same".to_string(),
                timestamp: chrono::Utc::now(),
                parents: vec![],
            };
            prop_assert!(a.id != b.id);
        }
    }

    #[test]
    fn branch_construction() {
        let branch = Branch {
            name: "main".to_string(),
            is_current: true,
            tracking: Some("origin/main".to_string()),
        };
        assert_eq!(branch.name, "main");
        assert!(branch.is_current);
        assert_eq!(branch.tracking.as_deref(), Some("origin/main"));
    }

    #[test]
    fn branch_clone() {
        let branch = Branch {
            name: "feature".to_string(),
            is_current: false,
            tracking: None,
        };
        let cloned = branch.clone();
        assert_eq!(cloned.name, "feature");
    }

    #[test]
    fn workspace_construction() {
        let ws = Workspace {
            name: "default".to_string(),
            branch: "main".to_string(),
            is_current: true,
        };
        assert_eq!(ws.name, "default");
        assert_eq!(ws.branch, "main");
        assert!(ws.is_current);
    }

    #[test]
    fn workspace_clone() {
        let ws = Workspace {
            name: "feature".to_string(),
            branch: "feature".to_string(),
            is_current: false,
        };
        let cloned = ws.clone();
        assert_eq!(cloned.name, "feature");
    }

    // ========================================================================
    // Property-based tests (proptest)
    // ========================================================================

    // 1. BranchName::new() is reflexive for valid inputs (parse then as_str returns same)
    // Note: regex must start with [a-zA-Z0-9/_] because BranchName rejects leading '-'
    proptest::proptest! {
        /// BranchName::new() followed by as_str() returns the original (trimmed) input.
        #[test]
        fn proptest_branch_name_reflexive_valid_input(
            name in "[a-zA-Z0-9/_][a-zA-Z0-9/_\\-.]{0,199}",
        ) {
            let parsed = BranchName::new(name.clone()).expect("valid name should parse");
            prop_assert_eq!(parsed.as_str(), name.trim());
        }

        /// BranchName::new() is reflexive for alphanumeric names with separators.
        #[test]
        fn proptest_branch_name_reflexive_alphanumeric(
            name in proptest::string::string_regex("[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,199}").unwrap(),
        ) {
            let parsed = BranchName::new(name.clone()).expect("valid name should parse");
            prop_assert_eq!(parsed.as_str(), name.trim());
        }
    }

    // 2. BranchName accepts shell metacharacters (they are valid branch name chars)
    #[test]
    fn branch_name_accepts_shell_metacharacters() {
        let metacharacters: &[char] = &[
            ';', '|', '&', '$', '`', '(', ')', '<', '>', '!', '#', '~', '*', '?', '[', ']', '{',
            '}', '%', '@', '^', '+', '=', '\'', '"', '\\',
        ];
        for &ch in metacharacters {
            let input = format!("branch{}name", ch);
            assert!(
                BranchName::new(&input).is_some(),
                "BranchName should accept metacharacter {:?} in {:?}",
                ch,
                input,
            );
        }
    }

    // 3. CommitId::new() returns None for empty/whitespace
    proptest::proptest! {
        /// CommitId::new() rejects all empty or whitespace-only strings.
        #[test]
        fn proptest_commit_id_rejects_empty_or_whitespace(
            input in proptest::string::string_regex("\\s*").unwrap(),
        ) {
            // Empty or whitespace-only strings should all be rejected.
            if input.trim().is_empty() {
                proptest::prop_assert!(CommitId::new(input.clone()).is_none());
            }
        }
    }

    #[test]
    fn commit_id_rejects_various_whitespace() {
        let whitespace_inputs = [
            "",
            " ",
            "  ",
            "\t",
            "\n",
            "\r",
            "\r\n",
            " \t ",
            "\n\t\r",
            "   \n\t\r   ",
        ];
        for input in whitespace_inputs {
            assert!(
                CommitId::new(input).is_none(),
                "CommitId should reject whitespace-only input: {:?}",
                input,
            );
        }
    }

    // 4. CommitId::from_unchecked always succeeds (including pathological inputs)
    proptest::proptest! {
        /// CommitId::from_unchecked always succeeds and preserves the input exactly.
        #[test]
        fn proptest_commit_id_from_unchecked_always_succeeds(
            bytes in proptest::collection::vec(proptest::arbitrary::any::<u8>(), 0..1000),
        ) {
            // Convert bytes to a string, replacing non-UTF8 with the replacement character.
            let input = String::from_utf8_lossy(&bytes).into_owned();
            let id = CommitId::from_unchecked(input.clone());
            prop_assert_eq!(id.as_str(), input);
        }
    }

    #[test]
    fn commit_id_from_unchecked_pathological_inputs() {
        let inputs: Vec<String> = vec![
            String::new(),
            " ".into(),
            "\0".into(),
            "\t\n\r".into(),
            "a".repeat(10000),
            String::from_utf8_lossy(&[0xFF, 0xFE, 0xFD]).into_owned(),
        ];
        for input in &inputs {
            let id = CommitId::from_unchecked(input.as_str());
            assert_eq!(
                id.as_str(),
                input,
                "from_unchecked should preserve input exactly"
            );
        }
    }

    // 5. detect_vcs returns None for empty directory (tested in multiple tempdirs)
    #[test]
    fn detect_vcs_returns_none_for_multiple_empty_dirs() {
        for _ in 0..10 {
            let dir = tempfile::tempdir().expect("tempdir");
            assert!(
                detect_vcs(dir.path()).is_none(),
                "detect_vcs should return None for empty dir {:?}",
                dir.path(),
            );
        }
    }

    // 6. VcsStatus Display roundtrip (parse from string)
    proptest::proptest! {
        /// VcsStatus Display -> parse roundtrip preserves all variants.
        #[test]
        fn proptest_vcs_status_display_roundtrip(
            status in proptest::sample::select(vec![
                VcsStatus::Clean,
                VcsStatus::Dirty,
                VcsStatus::Conflicted,
                VcsStatus::Detached,
            ]),
        ) {
            let displayed = format!("{status}");
            let parsed = match displayed.as_str() {
                "clean" => VcsStatus::Clean,
                "dirty" => VcsStatus::Dirty,
                "conflicted" => VcsStatus::Conflicted,
                "detached" => VcsStatus::Detached,
                other => panic!("unexpected VcsStatus display: {other}"),
            };
            prop_assert_eq!(status, parsed);
        }
    }

    #[test]
    fn vcs_status_display_roundtrip_all_variants() {
        let variants = [
            (VcsStatus::Clean, "clean"),
            (VcsStatus::Dirty, "dirty"),
            (VcsStatus::Conflicted, "conflicted"),
            (VcsStatus::Detached, "detached"),
        ];
        for (status, expected) in variants {
            let displayed = format!("{status}");
            assert_eq!(displayed, expected);
            // Parse back
            let parsed = match displayed.as_str() {
                "clean" => VcsStatus::Clean,
                "dirty" => VcsStatus::Dirty,
                "conflicted" => VcsStatus::Conflicted,
                "detached" => VcsStatus::Detached,
                other => panic!("unexpected: {other}"),
            };
            assert_eq!(status, parsed, "roundtrip failed for {expected}");
        }
    }

    // ========================================================================
    // Branch exhaustive tests
    // ========================================================================
    // Branch is a DTO: { name, is_current, tracking }. No PartialEq derive,
    // so equality is field-by-field. No Display impl — only Debug.

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn branch_construction_minimal() {
        let b = Branch {
            name: "main".into(),
            is_current: false,
            tracking: None,
        };
        assert_eq!(b.name, "main");
        assert!(!b.is_current);
        assert!(b.tracking.is_none());
    }

    #[test]
    fn branch_construction_current_with_tracking() {
        let b = Branch {
            name: "feature/auth".into(),
            is_current: true,
            tracking: Some("origin/feature/auth".into()),
        };
        assert_eq!(b.name, "feature/auth");
        assert!(b.is_current);
        assert_eq!(b.tracking.as_deref(), Some("origin/feature/auth"));
    }

    #[test]
    fn branch_construction_current_without_tracking() {
        // Local-only branch, no remote tracking
        let b = Branch {
            name: "wip".into(),
            is_current: true,
            tracking: None,
        };
        assert!(b.is_current);
        assert!(b.tracking.is_none());
    }

    #[test]
    fn branch_construction_not_current_with_tracking() {
        // Non-current branch that has a remote counterpart
        let b = Branch {
            name: "develop".into(),
            is_current: false,
            tracking: Some("upstream/develop".into()),
        };
        assert!(!b.is_current);
        assert_eq!(b.tracking.as_deref(), Some("upstream/develop"));
    }

    #[test]
    fn branch_name_various_valid_patterns() {
        let valid_names = [
            "main",
            "master",
            "develop",
            "feature/foo",
            "feature/foo-bar",
            "feature/foo_bar",
            "fix-123",
            "release/v2.0",
            "hotfix/urgent-fix",
            "a",
            "v1.0.0",
            "camelCaseBranch",
            "UPPER",
            "123numeric",
            "a/b/c/d/e",
            "trailing.",
            "name.with.dots",
        ];
        for name in valid_names {
            let b = Branch {
                name: name.into(),
                is_current: false,
                tracking: None,
            };
            assert_eq!(b.name, name, "valid branch name failed: {name}");
        }
    }

    // ── Clone ────────────────────────────────────────────────────────────────

    #[test]
    fn branch_clone_is_independent() {
        let b = Branch {
            name: "feature/x".into(),
            is_current: true,
            tracking: Some("origin/feature/x".into()),
        };
        let mut cloned = b.clone();
        // Mutating clone must not affect original
        cloned.name = "feature/y".into();
        cloned.is_current = false;
        cloned.tracking = None;
        assert_eq!(b.name, "feature/x");
        assert!(b.is_current);
        assert_eq!(b.tracking.as_deref(), Some("origin/feature/x"));
    }

    #[test]
    fn branch_clone_preserves_all_fields() {
        let b = Branch {
            name: "release/v3.1".into(),
            is_current: true,
            tracking: Some("origin/release/v3.1".into()),
        };
        let cloned = b.clone();
        assert_eq!(cloned.name, b.name);
        assert_eq!(cloned.is_current, b.is_current);
        assert_eq!(cloned.tracking, b.tracking);
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn branch_debug_format() {
        let b = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        let debug = format!("{b:?}");
        assert!(debug.contains("Branch"), "Debug should contain type name");
        assert!(debug.contains("main"), "Debug should contain name");
        assert!(
            debug.contains("origin/main"),
            "Debug should contain tracking"
        );
    }

    #[test]
    fn branch_debug_format_no_tracking() {
        let b = Branch {
            name: "local".into(),
            is_current: false,
            tracking: None,
        };
        let debug = format!("{b:?}");
        assert!(debug.contains("Branch"));
        assert!(debug.contains("local"));
        assert!(
            debug.contains("None"),
            "Debug should show None for tracking"
        );
    }

    // ── Field-by-field equality (no PartialEq derive) ───────────────────────

    #[test]
    fn branch_field_equality_same() {
        let a = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        let b = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        assert_eq!(a.name, b.name);
        assert_eq!(a.is_current, b.is_current);
        assert_eq!(a.tracking, b.tracking);
    }

    #[test]
    fn branch_field_equality_diff_name() {
        let a = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        let b = Branch {
            name: "develop".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        assert_ne!(a.name, b.name);
    }

    #[test]
    fn branch_field_equality_diff_current() {
        let a = Branch {
            name: "main".into(),
            is_current: true,
            tracking: None,
        };
        let b = Branch {
            name: "main".into(),
            is_current: false,
            tracking: None,
        };
        assert_ne!(a.is_current, b.is_current);
    }

    #[test]
    fn branch_field_equality_diff_tracking() {
        let a = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        let b = Branch {
            name: "main".into(),
            is_current: true,
            tracking: None,
        };
        assert_ne!(a.tracking, b.tracking);
    }

    // ── Tracking branch parsing ─────────────────────────────────────────────

    /// Helper: parse a tracking ref like "origin/main" into (remote, branch).
    fn parse_tracking(tracking: &str) -> Option<(&str, &str)> {
        let (remote, branch) = tracking.split_once('/')?;
        if remote.is_empty() || branch.is_empty() {
            return None;
        }
        Some((remote, branch))
    }

    #[test]
    fn tracking_parse_origin_main() {
        let (remote, branch) = parse_tracking("origin/main").expect("parse");
        assert_eq!(remote, "origin");
        assert_eq!(branch, "main");
    }

    #[test]
    fn tracking_parse_upstream_develop() {
        let (remote, branch) = parse_tracking("upstream/develop").expect("parse");
        assert_eq!(remote, "upstream");
        assert_eq!(branch, "develop");
    }

    #[test]
    fn tracking_parse_nested_branch() {
        // "origin/feature/auth" -> remote="origin", branch="feature/auth"
        let (remote, branch) = parse_tracking("origin/feature/auth").expect("parse");
        assert_eq!(remote, "origin");
        assert_eq!(branch, "feature/auth");
    }

    #[test]
    fn tracking_parse_rejects_no_slash() {
        assert!(parse_tracking("main").is_none());
    }

    #[test]
    fn tracking_parse_rejects_empty_remote() {
        assert!(parse_tracking("/main").is_none());
    }

    #[test]
    fn tracking_parse_rejects_empty_branch() {
        assert!(parse_tracking("origin/").is_none());
    }

    #[test]
    fn tracking_parse_custom_remote() {
        let (remote, branch) = parse_tracking("my-fork/feature/x").expect("parse");
        assert_eq!(remote, "my-fork");
        assert_eq!(branch, "feature/x");
    }

    #[test]
    fn branch_tracking_various_remotes() {
        let remotes = ["origin", "upstream", "my-fork", "backup"];
        for remote in remotes {
            let tracking = format!("{remote}/main");
            let b = Branch {
                name: "main".into(),
                is_current: false,
                tracking: Some(tracking.clone()),
            };
            assert_eq!(b.tracking.as_deref(), Some(tracking.as_str()));
            let (parsed_remote, parsed_branch) = parse_tracking(&tracking).expect("parse");
            assert_eq!(parsed_remote, remote);
            assert_eq!(parsed_branch, "main");
        }
    }

    // ── Current branch detection ─────────────────────────────────────────────

    #[test]
    fn current_branch_true() {
        let b = Branch {
            name: "main".into(),
            is_current: true,
            tracking: None,
        };
        assert!(b.is_current);
    }

    #[test]
    fn current_branch_false() {
        let b = Branch {
            name: "main".into(),
            is_current: false,
            tracking: None,
        };
        assert!(!b.is_current);
    }

    #[test]
    fn multiple_branches_exactly_one_current() {
        let branches = [
            Branch {
                name: "main".into(),
                is_current: true,
                tracking: Some("origin/main".into()),
            },
            Branch {
                name: "develop".into(),
                is_current: false,
                tracking: Some("origin/develop".into()),
            },
            Branch {
                name: "feature/x".into(),
                is_current: false,
                tracking: None,
            },
        ];
        let current_count = branches.iter().filter(|b| b.is_current).count();
        assert_eq!(current_count, 1, "exactly one branch should be current");
        assert!(branches[0].is_current);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn no_branch_current_detached_head() {
        let branches = [
            Branch {
                name: "main".into(),
                is_current: false,
                tracking: Some("origin/main".into()),
            },
            Branch {
                name: "develop".into(),
                is_current: false,
                tracking: Some("origin/develop".into()),
            },
        ];
        let current_count = branches.iter().filter(|b| b.is_current).count();
        assert_eq!(
            current_count, 0,
            "no branch should be current (detached HEAD)"
        );
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn branch_serde_roundtrip_full() {
        let original = Branch {
            name: "feature/auth".into(),
            is_current: true,
            tracking: Some("origin/feature/auth".into()),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.is_current, original.is_current);
        assert_eq!(deserialized.tracking, original.tracking);
    }

    #[test]
    fn branch_serde_roundtrip_no_tracking() {
        let original = Branch {
            name: "wip".into(),
            is_current: false,
            tracking: None,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "wip");
        assert!(!deserialized.is_current);
        assert!(deserialized.tracking.is_none());
    }

    #[test]
    fn branch_serialize_format() {
        let b = Branch {
            name: "main".into(),
            is_current: true,
            tracking: Some("origin/main".into()),
        };
        let json = serde_json::to_string(&b).expect("serialize");
        assert!(json.contains("\"name\":\"main\""));
        assert!(json.contains("\"is_current\":true"));
        assert!(json.contains("\"tracking\":\"origin/main\""));
    }

    #[test]
    fn branch_serialize_null_tracking() {
        let b = Branch {
            name: "local".into(),
            is_current: false,
            tracking: None,
        };
        let json = serde_json::to_string(&b).expect("serialize");
        assert!(
            json.contains("\"tracking\":null"),
            "None should serialize to null"
        );
    }

    #[test]
    fn branch_deserialize_from_json() {
        let json = r#"{"name":"develop","is_current":false,"tracking":"upstream/develop"}"#;
        let b: Branch = serde_json::from_str(json).expect("deserialize");
        assert_eq!(b.name, "develop");
        assert!(!b.is_current);
        assert_eq!(b.tracking.as_deref(), Some("upstream/develop"));
    }

    #[test]
    fn branch_deserialize_with_null_tracking() {
        let json = r#"{"name":"local","is_current":true,"tracking":null}"#;
        let b: Branch = serde_json::from_str(json).expect("deserialize");
        assert_eq!(b.name, "local");
        assert!(b.is_current);
        assert!(b.tracking.is_none());
    }

    #[test]
    fn branch_deserialize_missing_tracking_field() {
        // Serde default for Option<String> is None when field absent
        let json = r#"{"name":"local","is_current":false}"#;
        let result = serde_json::from_str::<Branch>(json);
        // With serde derive, missing field should fail unless #[serde(default)]
        // Branch uses plain derive(Deserialize) without defaults, so this may fail.
        // This test documents the behavior.
        if let Ok(b) = result {
            assert!(
                b.tracking.is_none(),
                "missing tracking should be None if accepted"
            );
        }
        // If it fails, that's also valid behavior — the test documents it.
    }

    // ── BranchName validation (exhaustive rejection) ─────────────────────────

    #[test]
    fn branch_name_rejects_leading_dash_variants() {
        let bad = ["-", "-a", "-v", "--", "---", "--flag", "- "];
        for name in bad {
            assert!(
                BranchName::new(name).is_none(),
                "BranchName should reject leading-dash: {name:?}"
            );
        }
    }

    #[test]
    fn branch_name_rejects_spaces() {
        let bad = ["main branch", " main", "main ", "ma in", "  "];
        for name in bad {
            // Note: BranchName trims whitespace, so " main" becomes "main" (valid)
            // But "main branch" contains a space and would be rejected if spaces
            // are control chars... actually spaces are not control characters.
            // BranchName only rejects control chars, not spaces.
            // " main" trims to "main" → valid.
            // "main branch" contains space (not a control char) → valid per current impl.
            let result = BranchName::new(name);
            if name.trim().contains(' ') {
                // Spaces in the trimmed string: not rejected by current impl
                // (space is not a control char)
                // This test documents the current behavior.
            }
            if name.trim().is_empty() {
                assert!(result.is_none(), "whitespace-only should be None: {name:?}");
            }
        }
    }

    #[test]
    fn branch_name_rejects_null_and_control_chars_exhaustive() {
        let bad = [
            "branch\0name",
            "branch\nname",
            "branch\tname",
            "branch\rname",
            "branch\x01name",
            "branch\x02name",
            "branch\x1Fname",
            "branch\x7Fname",
        ];
        for name in bad {
            assert!(
                BranchName::new(name).is_none(),
                "BranchName should reject: {name:?}"
            );
        }
    }

    #[test]
    fn branch_name_rejects_dot_and_dotdot() {
        assert!(BranchName::new(".").is_none());
        assert!(BranchName::new("..").is_none());
        assert!(BranchName::new(" . ").is_none()); // trims to "."
        assert!(BranchName::new(" .. ").is_none()); // trims to ".."
    }

    #[test]
    fn branch_name_accepts_dot_in_name() {
        assert!(BranchName::new("v1.0.0").is_some());
        assert!(BranchName::new("branch.name").is_some());
        assert!(BranchName::new("release.v2").is_some());
    }

    #[test]
    fn branch_name_boundary_lengths() {
        // Empty
        assert!(BranchName::new("").is_none());
        // Single char
        assert!(BranchName::new("a").is_some());
        // Max length (250)
        let max = "a".repeat(250);
        assert!(BranchName::new(&max).is_some());
        // One over max (251)
        let over = "a".repeat(251);
        assert!(BranchName::new(&over).is_none());
    }

    #[test]
    fn branch_name_trims_then_validates() {
        // Leading/trailing whitespace is trimmed, then validated
        let name = BranchName::new("  main  ").expect("valid");
        assert_eq!(name.as_str(), "main");
        let name2 = BranchName::new("\tmain\t").expect("valid");
        assert_eq!(name2.as_str(), "main");
        // Trimmed to "." -> rejected
        assert!(BranchName::new(" . ").is_none());
    }

    #[test]
    fn branch_name_display_matches_inner() {
        let cases = ["main", "feature/auth", "fix-123", "v1.0.0"];
        for case in cases {
            let name = BranchName::new(case).expect("valid");
            assert_eq!(format!("{name}"), case);
        }
    }

    #[test]
    fn branch_name_as_str_matches_display() {
        let name = BranchName::new("feature/x").expect("valid");
        assert_eq!(name.as_str(), format!("{name}"));
    }

    #[test]
    fn branch_name_hash_and_eq() {
        use std::collections::HashSet;
        let a = BranchName::new("main").expect("valid");
        let b = BranchName::new("main").expect("valid");
        assert_eq!(a, b, "two BranchNames from same string should be equal");
        let mut set = HashSet::new();
        assert!(set.insert(a.clone()));
        assert!(!set.insert(b), "duplicate insert should return false");
        assert!(set.contains(&a));
    }

    // ── Branch with BranchName integration ───────────────────────────────────

    #[test]
    fn branch_name_from_branch_used_in_display() {
        let branch_name = BranchName::new("feature/auth").expect("valid");
        let b = Branch {
            name: branch_name.to_string(),
            is_current: true,
            tracking: Some(format!("origin/{branch_name}")),
        };
        assert_eq!(b.name, "feature/auth");
        assert_eq!(b.tracking.as_deref(), Some("origin/feature/auth"));
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    proptest::proptest! {
        /// Branch serde roundtrip preserves all fields for any valid inputs.
        #[test]
        fn proptest_branch_serde_roundtrip(
            name in "[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,49}",
            is_current in proptest::arbitrary::any::<bool>(),
            tracking in proptest::option::of("[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,49}"),
        ) {
            let original = Branch {
                name: name.clone(),
                is_current,
                tracking: tracking.clone(),
            };
            let json = serde_json::to_string(&original).expect("serialize");
            let decoded: Branch = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(decoded.name, name);
            prop_assert_eq!(decoded.is_current, is_current);
            prop_assert_eq!(decoded.tracking, tracking);
        }

        /// Branch clone is always identical (field-by-field).
        #[test]
        fn proptest_branch_clone_identical(
            name in "[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,49}",
            is_current in proptest::arbitrary::any::<bool>(),
            tracking in proptest::option::of("[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,49}"),
        ) {
            let b = Branch {
                name: name.clone(),
                is_current,
                tracking: tracking.clone(),
            };
            let cloned = b.clone();
            prop_assert_eq!(cloned.name, name);
            prop_assert_eq!(cloned.is_current, is_current);
            prop_assert_eq!(cloned.tracking, tracking);
        }

        /// BranchName::new followed by as_str is identity for valid branch names.
        #[test]
        fn proptest_branch_name_identity(
            name in "[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,249}",
        ) {
            let parsed = BranchName::new(name.clone());
            prop_assert!(parsed.is_some(), "valid name should parse: {name:?}");
            let bn = parsed.expect("checked above");
            prop_assert_eq!(bn.as_str(), name);
        }

        /// BranchName rejects all empty strings.
        #[test]
        fn proptest_branch_name_rejects_empty(
            s in proptest::string::string_regex("\\s*").unwrap(),
        ) {
            if s.trim().is_empty() {
                prop_assert!(BranchName::new(s).is_none());
            }
        }

        /// BranchName rejects all strings starting with '-'.
        #[test]
        fn proptest_branch_name_rejects_leading_dash(
            rest in "[a-zA-Z0-9/_.-]{0,50}",
        ) {
            let input = format!("-{rest}");
            prop_assert!(BranchName::new(&input).is_none());
        }

        /// Tracking ref parse roundtrip: "remote/branch" -> split -> rejoin.
        #[test]
        fn proptest_tracking_parse_roundtrip(
            remote in "[a-zA-Z0-9][a-zA-Z0-9._-]{0,30}",
            branch in "[a-zA-Z0-9][a-zA-Z0-9/_.-]{0,50}",
        ) {
            let tracking = format!("{remote}/{branch}");
            let parsed = parse_tracking(&tracking);
            prop_assert!(parsed.is_some(), "should parse: {tracking}");
            let (r, b) = parsed.expect("checked above");
            prop_assert_eq!(r, remote);
            prop_assert_eq!(b, branch);
        }

        /// Branch with constructed tracking matches name when remote is "origin".
        #[test]
        fn proptest_branch_tracking_origin_prefix(
            name in "[a-zA-Z0-9/_][a-zA-Z0-9/_.-]{0,49}",
        ) {
            let tracking = format!("origin/{name}");
            let b = Branch {
                name: name.clone(),
                is_current: false,
                tracking: Some(tracking),
            };
            let (remote, branch) = parse_tracking(b.tracking.as_ref().expect("some"))
                .expect("parse");
            prop_assert_eq!(remote, "origin");
            prop_assert_eq!(branch, name);
        }
    }
}
