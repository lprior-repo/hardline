//! Core domain types for SCP (Source Control Plane)
//!
//! Provides session, workspace, and change tracking types with zero-unwrap patterns.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Error, Result};
use crate::lifecycle::LifecycleState;
use crate::workspace_state::WorkspaceState;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    const MAX_LENGTH: usize = 64;

    pub fn parse(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(Error::InvalidState(
                "Session name cannot be empty".to_string(),
            ));
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(Error::InvalidState(format!(
                "Session name cannot exceed {} characters",
                Self::MAX_LENGTH
            )));
        }
        let first_char = name
            .chars()
            .next()
            .ok_or_else(|| Error::InvalidState("Session name cannot be empty".to_string()))?;
        if !first_char.is_alphabetic() {
            return Err(Error::InvalidState(
                "Session name must start with a letter".to_string(),
            ));
        }
        let valid_chars = name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if !valid_chars {
            return Err(Error::InvalidState(
                "Session name can only contain letters, numbers, dashes, and underscores"
                    .to_string(),
            ));
        }
        Ok(Self(name))
    }

    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::parse(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for SessionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn parse(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::InvalidState(
                "Session ID cannot be empty".to_string(),
            ));
        }
        let valid_chars = id.chars().all(|c| c.is_alphanumeric() || c == '-');
        if !valid_chars {
            return Err(Error::InvalidState(
                "Session ID can only contain alphanumeric characters and hyphens".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    pub fn parse(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(Error::InvalidState("Path must be absolute".to_string()));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap_or("")
    }
}

impl From<String> for AbsolutePath {
    fn from(s: String) -> Self {
        Self(PathBuf::from(s))
    }
}

impl std::str::FromStr for AbsolutePath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchState {
    Detached,
    OnBranch(String),
}

impl BranchState {
    pub fn detached() -> Self {
        Self::Detached
    }

    pub fn on_branch(branch: impl Into<String>) -> Self {
        Self::OnBranch(branch.into())
    }

    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch(name) => Some(name),
        }
    }

    pub fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }
}

impl Serialize for BranchState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Detached => serializer.serialize_str("detached"),
            Self::OnBranch(name) => serializer.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for BranchState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "detached" {
            Ok(Self::Detached)
        } else {
            Ok(Self::OnBranch(s))
        }
    }
}

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

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Creating => vec![Self::Active, Self::Failed],
            Self::Active => vec![Self::Paused, Self::Completed],
            Self::Paused => vec![Self::Active, Self::Completed],
            Self::Completed | Self::Failed => vec![],
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    pub fn all_states() -> &'static [Self] {
        &[
            Self::Creating,
            Self::Active,
            Self::Paused,
            Self::Completed,
            Self::Failed,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Status,
    Diff,
    Focus,
    Remove,
}

impl SessionStatus {
    pub fn allowed_operations(self) -> Vec<Operation> {
        match self {
            Self::Creating => vec![],
            Self::Active => vec![
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => vec![Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => vec![Operation::Remove],
        }
    }

    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

impl LifecycleState for SessionStatus {
    fn can_transition_to(self, next: Self) -> bool {
        self.can_transition_to(next)
    }

    fn valid_next_states(self) -> Vec<Self> {
        self.valid_next_states()
    }

    fn is_terminal(self) -> bool {
        self.is_terminal()
    }

    fn all_states() -> &'static [Self] {
        Self::all_states()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: SessionName,
    pub status: SessionStatus,
    pub state: WorkspaceState,
    #[serde(serialize_with = "serialize_absolute_path")]
    #[serde(deserialize_with = "deserialize_absolute_path")]
    pub workspace_path: AbsolutePath,
    pub branch: BranchState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: ValidatedMetadata,
}

fn serialize_absolute_path<S>(
    path: &AbsolutePath,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(path.as_str())
}

fn deserialize_absolute_path<'de, D>(deserializer: D) -> std::result::Result<AbsolutePath, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    AbsolutePath::parse(s).map_err(serde::de::Error::custom)
}

impl Session {
    pub fn validate_pure(&self) -> Result<()> {
        if self.updated_at < self.created_at {
            return Err(Error::InvalidState(
                "Updated timestamp cannot be before created timestamp".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_pure()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileStatus {
    #[serde(rename = "M")]
    Modified,
    #[serde(rename = "A")]
    Added,
    #[serde(rename = "D")]
    Deleted,
    #[serde(rename = "R")]
    Renamed,
    #[serde(rename = "?")]
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl FileChange {
    pub fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::InvalidState(
                "Renamed files must have old_path set".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangesSummary {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub untracked: usize,
}

impl ChangesSummary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified + self.added + self.deleted + self.renamed > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    pub path: PathBuf,
    pub insertions: usize,
    pub deletions: usize,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
    pub files: Vec<FileDiffStat>,
}

impl DiffSummary {
    pub fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::InvalidState(format!(
                "files_changed ({}) does not match files array length ({})",
                self.files_changed,
                self.files.len()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeadsSummary {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub closed: usize,
}

impl BeadsSummary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.open + self.in_progress + self.blocked + self.closed
    }

    #[must_use]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));

        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));

        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_session_status_allowed_operations() {
        assert!(SessionStatus::Creating.allowed_operations().is_empty());
        assert!(SessionStatus::Active.allows_operation(Operation::Status));
        assert!(SessionStatus::Active.allows_operation(Operation::Focus));
        assert!(SessionStatus::Paused.allows_operation(Operation::Remove));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
    }

    #[test]
    fn test_session_name_rejects_invalid() {
        assert!(SessionName::parse("invalid name").is_err());
        assert!(SessionName::parse("123-start-with-number").is_err());
        assert!(SessionName::parse("").is_err());
        assert!(SessionName::parse(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_session_name_accepts_valid() {
        assert!(SessionName::parse("valid-name").is_ok());
        assert!(SessionName::parse("Feature_Auth").is_ok());
        assert!(SessionName::parse("a").is_ok());
    }

    #[test]
    fn test_absolute_path_rejects_relative() {
        assert!(AbsolutePath::parse("relative/path").is_err());
    }

    #[test]
    fn test_session_validate_timestamps() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(60);

        let session = Session {
            id: SessionId::parse("id123").expect("valid id"),
            name: SessionName::parse("valid-name").expect("valid name"),
            status: SessionStatus::Creating,
            state: WorkspaceState::Created,
            workspace_path: AbsolutePath::parse("/tmp/test").expect("valid path"),
            branch: BranchState::Detached,
            created_at: now,
            updated_at: earlier,
            last_synced: None,
            metadata: ValidatedMetadata::empty(),
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_changes_summary_total() {
        let summary = ChangesSummary {
            modified: 5,
            added: 3,
            deleted: 2,
            renamed: 1,
            untracked: 4,
        };

        assert_eq!(summary.total(), 11);
        assert!(summary.has_changes());
        assert!(summary.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_no_changes() {
        let summary = ChangesSummary::default();
        assert_eq!(summary.total(), 0);
        assert!(!summary.has_changes());
    }

    #[test]
    fn test_beads_summary_active() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };

        assert_eq!(summary.total(), 11);
        assert_eq!(summary.active(), 5);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_file_change_renamed_validation() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: None,
        };

        assert!(change.validate().is_err());
    }

    #[test]
    fn test_file_change_renamed_valid() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old/path.txt")),
        };

        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_validation() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 2,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("file1.txt"),
                    insertions: 5,
                    deletions: 2,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("file2.txt"),
                    insertions: 5,
                    deletions: 3,
                    status: FileStatus::Added,
                },
            ],
        };

        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_mismatch() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 5,
            files: vec![FileDiffStat {
                path: PathBuf::from("file1.txt"),
                insertions: 5,
                deletions: 2,
                status: FileStatus::Modified,
            }],
        };

        assert!(diff.validate().is_err());
    }

    #[test]
    fn test_session_status_terminal_states() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Failed.is_terminal());
        assert!(!SessionStatus::Creating.is_terminal());
        assert!(!SessionStatus::Active.is_terminal());
        assert!(!SessionStatus::Paused.is_terminal());
    }

    #[test]
    fn test_session_name_max_length() {
        let exactly_64: String = "a".repeat(64);
        assert!(
            SessionName::parse(&exactly_64).is_ok(),
            "64 chars should be valid"
        );

        let too_long: String = "a".repeat(65);
        assert!(
            SessionName::parse(&too_long).is_err(),
            "65 chars should be invalid"
        );
    }

    #[test]
    fn test_session_name_special_chars() {
        assert!(SessionName::parse("name-with-dash").is_ok());
        assert!(SessionName::parse("name_with_underscore").is_ok());
        assert!(SessionName::parse("NameWithCaps123").is_ok());
        assert!(SessionName::parse("name with space").is_err());
        assert!(SessionName::parse("name@special").is_err());
        assert!(SessionName::parse("name.dots").is_err());
    }

    #[test]
    fn test_session_name_must_start_with_letter() {
        assert!(SessionName::parse("a").is_ok());
        assert!(SessionName::parse("A").is_ok());
        assert!(SessionName::parse("1start-with-number").is_err());
        assert!(SessionName::parse("_start-with-underscore").is_err());
        assert!(SessionName::parse("-start-with-dash").is_err());
    }

    #[test]
    fn test_branch_state_serialization() {
        let detached = BranchState::detached();
        let on_branch = BranchState::on_branch("feature/test");

        let detached_json = serde_json::to_string(&detached).unwrap();
        let branch_json = serde_json::to_string(&on_branch).unwrap();

        assert!(detached_json.contains("detached"));
        assert!(branch_json.contains("feature/test"));
    }
}
