//! Core domain types for isolate with contracts and validation
//!
//! All types implement the `HasContract` trait, providing:
//! - Type constraints and validation
//! - Contextual hints for AI agents
//! - JSON Schema generation
//! - Self-documenting APIs

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION NAME VALUE OBJECT
// ═══════════════════════════════════════════════════════════════════════════

// Re-export from domain module (single source of truth)
//
// The domain::SessionName is the canonical implementation with consistent validation
// rules (MAX_LENGTH = 63). This re-export provides backward compatibility for code
// using `types::SessionName` and ensures all parts of the codebase use the same type.
pub use crate::domain::SessionName;
use crate::{
    contracts::{Constraint, ContextualHint, FieldContract, HasContract, HintType, TypeContract},
    domain::{session::BranchState, AbsolutePath, SessionId},
    output::ValidatedMetadata,
    Error, Result, WorkspaceState,
};

// Backward compatibility: provide new() method that delegates to parse()
//
// This allows existing code using SessionName::parse() to continue working
// while new code can use SessionName::parse() for consistency with other domain types.
#[cfg_attr(test, allow(clippy::missing_const_for_fn))]
impl SessionName {
    /// Create a `SessionName` (backward compatibility alias for `parse()`)
    ///
    /// # Errors
    ///
    /// Returns `Error::ValidationError` if the name violates any invariant.
    ///
    /// # Note
    ///
    /// This is a compatibility alias for `SessionName::parse()`.
    /// New code should use `parse()` for consistency with domain types.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::parse(name).map_err(|e| Error::ValidationError {
            message: e.to_string(),
            field: Some("name".to_string()),
            value: None,
            constraints: vec![],
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATUS
// ═════════════════════════════════════════════════════════════════════════

/// Session lifecycle states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created (transient)
    Creating,
    /// Session is ready for use
    Active,
    /// Session exists but not currently in use
    Paused,
    /// Work completed, ready for removal
    Completed,
    /// Creation or hook failed
    Failed,
}

impl SessionStatus {
    /// Valid state transitions
    ///
    /// # Returns
    /// `true` if transition from current state to `next` is valid. The result
    /// should be checked before performing state transitions.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    /// Returns all valid next states from current state.
    ///
    /// Uses exhaustive pattern matching to ensure all states are covered.
    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Creating => vec![Self::Active, Self::Failed],
            Self::Active => vec![Self::Paused, Self::Completed],
            Self::Paused => vec![Self::Active, Self::Completed],
            // Terminal states - no transitions out
            Self::Completed | Self::Failed => vec![],
        }
    }

    /// Returns true if this is a terminal state (no transitions out).
    ///
    /// SessionStatus.Completed and SessionStatus.Failed are terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Returns all possible session status states.
    #[must_use]
    pub const fn all_states() -> &'static [Self] {
        &[
            Self::Creating,
            Self::Active,
            Self::Paused,
            Self::Completed,
            Self::Failed,
        ]
    }

    /// Allowed operations in this state
    ///
    /// # Returns
    ///
    /// Returns a slice of allowed operations. The result should be used
    /// for validation or authorization checks.
    #[must_use]
    pub const fn allowed_operations(self) -> &'static [Operation] {
        match self {
            Self::Creating => &[],
            Self::Active => &[
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => &[Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => &[Operation::Remove],
        }
    }

    /// Check if an operation is allowed in this state
    ///
    /// # Returns
    ///
    /// Returns `true` if the operation is allowed. The result should be checked
    /// before performing operations.
    #[must_use]
    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// LIFECYCLE TRAIT IMPLEMENTATION (bd-bzl)
// ═══════════════════════════════════════════════════════════════════════════

impl crate::lifecycle::LifecycleState for SessionStatus {
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

/// Operations that can be performed on sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// View session status
    Status,
    /// View diff
    Diff,
    /// Focus session
    Focus,
    /// Remove session
    Remove,
}

/// A session represents a parallel workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier (validated, non-empty, ASCII)
    pub id: SessionId,

    /// Human-readable session name
    ///
    /// # Contract
    /// - MUST start with a letter
    /// - MUST match regex: `^[a-zA-Z][a-zA-Z0-9_-]{0,63}$`
    /// - MUST be unique across all sessions
    /// - MUST NOT exceed 64 characters
    pub name: SessionName,

    /// Current session status
    pub status: SessionStatus,

    /// Workspace lifecycle state (tracks work progress)
    pub state: WorkspaceState,

    /// Absolute path to workspace directory
    ///
    /// # Contract
    /// - MUST be absolute path
    /// - MUST exist if status != Creating
    #[serde(serialize_with = "serialize_absolute_path")]
    #[serde(deserialize_with = "deserialize_absolute_path")]
    pub workspace_path: AbsolutePath,

    /// Branch state (explicit branch or detached)
    ///
    /// # Contract
    /// - Uses enum instead of Option<String> for clearer state
    pub branch: BranchState,

    /// Creation timestamp (UTC)
    pub created_at: DateTime<Utc>,

    /// Last update timestamp (UTC)
    pub updated_at: DateTime<Utc>,

    /// Last sync timestamp (UTC, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,

    /// Validated metadata (extensibility with type safety)
    #[serde(default)]
    pub metadata: ValidatedMetadata,
}

// Serde serialization helpers for AbsolutePath
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
    /// Validate session invariants (pure domain validation, no I/O)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Timestamps are in wrong order
    ///
    /// # Note
    ///
    /// Name, workspace path, and ID are already validated by their newtype constructors
    /// (`SessionName::parse()`, `AbsolutePath::parse()`, `SessionId::parse()`).
    pub fn validate_pure(&self) -> Result<()> {
        // SessionName is already validated by its parse() constructor
        // workspace_path is already validated by AbsolutePath::parse()
        // id is validated by SessionId::parse()

        if self.updated_at < self.created_at {
            return Err(Error::ValidationError {
                message: "Updated timestamp cannot be before created timestamp".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        Ok(())
    }

    /// Validate session invariants (pure domain validation only).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Timestamps are in wrong order
    ///
    /// # Note
    ///
    /// Name, workspace path, and ID are already validated by their newtype constructors.
    /// This does NOT check if the workspace exists on the filesystem.
    /// For filesystem validation, use `crate::validation::validate_session_workspace_exists()`
    /// from the infrastructure layer.
    pub fn validate(&self) -> Result<()> {
        self.validate_pure()
    }

    /// Get the session name as a string slice.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl HasContract for Session {
    fn contract() -> TypeContract {
        TypeContract::builder("Session")
            .description("A parallel workspace for isolating work")
            .field(
                "name",
                FieldContract::builder("name", "SessionName")
                    .required()
                    .description("Human-readable session name")
                    .constraint(Constraint::Regex {
                        pattern: r"^[a-zA-Z][a-zA-Z0-9_-]{0,63}$".to_string(),
                        description:
                            "starts with letter, alphanumeric/dash/underscore, max 64 chars"
                                .to_string(),
                    })
                    .constraint(Constraint::Length {
                        min: Some(1),
                        max: Some(64),
                    })
                    .constraint(Constraint::Unique)
                    .example("feature-auth")
                    .example("bugfix-123")
                    .example("experiment_idea")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "SessionStatus")
                    .required()
                    .description("Current lifecycle state of the session")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "creating".to_string(),
                            "active".to_string(),
                            "paused".to_string(),
                            "completed".to_string(),
                            "failed".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "state",
                FieldContract::builder("state", "WorkspaceState")
                    .required()
                    .description("Workspace lifecycle state")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "created".to_string(),
                            "working".to_string(),
                            "ready".to_string(),
                            "merged".to_string(),
                            "abandoned".to_string(),
                            "conflict".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "workspace_path",
                FieldContract::builder("workspace_path", "PathBuf")
                    .required()
                    .description("Absolute path to the workspace directory")
                    .constraint(Constraint::PathAbsolute)
                    .constraint(Constraint::Custom {
                        rule: "must exist if status != creating".to_string(),
                        description: "Workspace directory must exist for non-creating sessions"
                            .to_string(),
                    })
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::BestPractice,
                message: "Use descriptive session names that indicate the purpose of the work"
                    .to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Session name cannot be changed after creation".to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        self.validate()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHANGE TRACKING TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// File modification status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileStatus {
    /// File modified
    #[serde(rename = "M")]
    Modified,
    /// File added
    #[serde(rename = "A")]
    Added,
    /// File deleted
    #[serde(rename = "D")]
    Deleted,
    /// File renamed
    #[serde(rename = "R")]
    Renamed,
    /// File untracked
    #[serde(rename = "?")]
    Untracked,
}

/// A single file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// File path relative to workspace root
    pub path: PathBuf,

    /// Modification status
    pub status: FileStatus,

    /// Original path (only for `Renamed`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl HasContract for FileChange {
    fn contract() -> TypeContract {
        TypeContract::builder("FileChange")
            .description("Represents a change to a file in the workspace")
            .field(
                "path",
                FieldContract::builder("path", "PathBuf")
                    .required()
                    .description("File path relative to workspace root")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "FileStatus")
                    .required()
                    .description("Type of modification")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "M".to_string(),
                            "A".to_string(),
                            "D".to_string(),
                            "R".to_string(),
                            "?".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "old_path",
                FieldContract::builder("old_path", "Option<PathBuf>")
                    .description("Original path for renamed files")
                    .constraint(Constraint::Custom {
                        rule: "required when status is Renamed".to_string(),
                        description: "Must be set when file is renamed".to_string(),
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::ValidationError {
                message: "Renamed files must have old_path set".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }
        Ok(())
    }
}

/// Summary of changes in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangesSummary {
    /// Number of modified files
    pub modified: usize,

    /// Number of added files
    pub added: usize,

    /// Number of deleted files
    pub deleted: usize,

    /// Number of renamed files
    pub renamed: usize,

    /// Number of untracked files
    pub untracked: usize,
}

impl ChangesSummary {
    /// Total number of changed files
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    /// Has any changes?
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    /// Has any tracked changes (excluding untracked)?
    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified + self.added + self.deleted + self.renamed > 0
    }
}

impl HasContract for ChangesSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("ChangesSummary")
            .description("Summary of file changes in a workspace")
            .field(
                "modified",
                FieldContract::builder("modified", "usize")
                    .required()
                    .description("Number of modified files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "added",
                FieldContract::builder("added", "usize")
                    .required()
                    .description("Number of added files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "deleted",
                FieldContract::builder("deleted", "usize")
                    .required()
                    .description("Number of deleted files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Example,
                message: "Use total() method to get sum of all changes".to_string(),
                condition: None,
                related_to: None,
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        // All fields are usize, so always valid
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DIFF TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Diff statistics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    /// File path
    pub path: PathBuf,

    /// Lines inserted
    pub insertions: usize,

    /// Lines deleted
    pub deletions: usize,

    /// File status (`A`/`M`/`D`/`R`)
    pub status: FileStatus,
}

/// Summary of diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of lines inserted
    pub insertions: usize,

    /// Number of lines deleted
    pub deletions: usize,

    /// Number of files changed
    pub files_changed: usize,

    /// Per-file statistics
    pub files: Vec<FileDiffStat>,
}

impl HasContract for DiffSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("DiffSummary")
            .description("Summary of differences between commits or workspace state")
            .field(
                "insertions",
                FieldContract::builder("insertions", "usize")
                    .required()
                    .description("Total number of lines inserted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "deletions",
                FieldContract::builder("deletions", "usize")
                    .required()
                    .description("Total number of lines deleted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "files_changed",
                FieldContract::builder("files_changed", "usize")
                    .required()
                    .description("Number of files changed")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::ValidationError {
                message: format!(
                    "files_changed ({}) does not match files array length ({})",
                    self.files_changed,
                    self.files.len()
                ),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BEADS TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Issue status from beads
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

/// A beads issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsIssue {
    /// Issue ID (e.g., "isolate-abc")
    pub id: String,

    /// Issue title
    pub title: String,

    /// Issue status
    pub status: IssueStatus,

    /// Priority (e.g., "P1", "P2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,

    /// Issue type (e.g., "task", "bug", "feature")
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
}

/// Summary of beads issues
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeadsSummary {
    /// Number of open issues
    pub open: usize,

    /// Number of in-progress issues
    pub in_progress: usize,

    /// Number of blocked issues
    pub blocked: usize,

    /// Number of closed issues
    pub closed: usize,
}

impl BeadsSummary {
    /// Total number of issues
    #[must_use]
    pub const fn total(&self) -> usize {
        self.open + self.in_progress + self.blocked + self.closed
    }

    /// Number of active issues (open + `in_progress`)
    #[must_use]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    /// Has blocking issues?
    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

impl HasContract for BeadsSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("BeadsSummary")
            .description("Summary of beads issues in a workspace")
            .field(
                "open",
                FieldContract::builder("open", "usize")
                    .required()
                    .description("Number of open issues")
                    .default("0")
                    .build(),
            )
            .field(
                "in_progress",
                FieldContract::builder("in_progress", "usize")
                    .required()
                    .description("Number of in-progress issues")
                    .default("0")
                    .build(),
            )
            .field(
                "blocked",
                FieldContract::builder("blocked", "usize")
                    .required()
                    .description("Number of blocked issues")
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Blocked issues prevent progress - resolve blockers first".to_string(),
                condition: Some("blocked > 0".to_string()),
                related_to: Some("blocked".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

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
        assert_eq!(SessionStatus::Creating.allowed_operations().len(), 0);
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
    fn test_session_validate_path_not_absolute() {
        // This test is no longer relevant since AbsolutePath enforces absoluteness
        // at construction time. The path cannot be created if it's not absolute.
        // We'll test that AbsolutePath rejects relative paths instead.
        let result = AbsolutePath::parse("relative/path");
        assert!(result.is_err());
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
            updated_at: earlier, // updated before created!
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
    fn test_beads_summary_no_blockers() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 0,
            closed: 5,
        };

        assert!(!summary.has_blockers());
    }

    #[test]
    fn test_file_change_renamed_validation() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: None, // Missing old_path!
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
            files_changed: 5, // Mismatch!
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
    fn test_session_contract() {
        let contract = Session::contract();
        assert_eq!(contract.name, "Session");
        assert!(!contract.fields.is_empty());
        assert!(contract.fields.contains_key("name"));
        assert!(contract.fields.contains_key("status"));
    }

    #[test]
    fn test_session_json_schema() {
        let schema = Session::json_schema();
        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
        assert_eq!(
            schema.get("title").and_then(|v| v.as_str()),
            Some("Session")
        );
        assert!(schema
            .get("properties")
            .and_then(|v| v.as_object())
            .is_some());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // LIFECYCLE TRAIT CONFORMANCE TESTS (bd-bzl)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_status_implements_lifecycle() {
        // SessionStatus implements LifecycleState trait
        let _can_transition = SessionStatus::Creating.can_transition_to(SessionStatus::Active);
        let _valid_next = SessionStatus::Creating.valid_next_states();
        let _is_terminal = SessionStatus::Completed.is_terminal();
        let _all = SessionStatus::all_states();
    }

    #[test]
    fn test_session_status_conformance() {
        use crate::lifecycle::conformance_tests;

        // SessionStatus passes all conformance tests
        conformance_tests::run_all_tests::<SessionStatus>();
    }

    #[test]
    fn test_session_status_terminal_states() {
        // SessionStatus.Completed and Failed are terminal
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Failed.is_terminal());
        assert!(!SessionStatus::Creating.is_terminal());
        assert!(!SessionStatus::Active.is_terminal());
        assert!(!SessionStatus::Paused.is_terminal());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSION NAME VALUE OBJECT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_name_from_str() {
        use std::str::FromStr;
        let name = SessionName::from_str("my-feature").expect("valid name");
        assert_eq!(name.as_str(), "my-feature");
    }

    #[test]
    fn test_session_name_display() {
        let name = SessionName::parse("test-session").expect("valid name");
        assert_eq!(format!("{name}"), "test-session");
    }

    #[test]
    fn test_session_name_as_ref() {
        let name = SessionName::parse("session-name").expect("valid name");
        let ref_str: &str = name.as_ref();
        assert_eq!(ref_str, "session-name");
    }

    #[test]
    fn test_session_name_into_string() {
        let name = SessionName::parse("session").expect("valid name");
        let s: String = name.into();
        assert_eq!(s, "session");
    }

    #[test]
    fn test_session_name_max_length() {
        let exactly_63: String = "a".repeat(63);
        assert!(
            SessionName::parse(&exactly_63).is_ok(),
            "63 chars should be valid"
        );

        let too_long: String = "a".repeat(64);
        assert!(
            SessionName::parse(&too_long).is_err(),
            "64 chars should be invalid"
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
}
