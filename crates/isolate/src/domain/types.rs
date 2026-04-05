//! Workspace state types for the isolate domain.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::IsolateError;

/// Lifecycle states for an isolated git-clone workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    #[default]
    Created,
    Working,
    Ready,
    Merged,
    Abandoned,
    Conflict,
}

impl WorkspaceState {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Created, Self::Working, Self::Ready, Self::Merged, Self::Abandoned, Self::Conflict]
    }

    #[must_use]
    pub fn valid_next_states(self) -> &'static [Self] {
        match self {
            Self::Created => &[Self::Working],
            Self::Working => &[Self::Ready, Self::Conflict, Self::Abandoned],
            Self::Ready => &[Self::Working, Self::Merged, Self::Conflict, Self::Abandoned],
            Self::Conflict => &[Self::Working, Self::Abandoned],
            Self::Merged | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, target: Self) -> bool {
        self.valid_next_states().contains(&target)
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Abandoned)
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Working | Self::Conflict)
    }

    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Ready | Self::Merged)
    }
}

impl fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Created => "created",
            Self::Working => "working",
            Self::Ready => "ready",
            Self::Merged => "merged",
            Self::Abandoned => "abandoned",
            Self::Conflict => "conflict",
        })
    }
}

impl FromStr for WorkspaceState {
    type Err = IsolateError;
    fn from_str(s: &str) -> std::result::Result<Self, IsolateError> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(IsolateError::InvalidState(s.to_string())),
        }
    }
}

/// Unique identifier for an isolated workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Generate a new unique workspace ID.
    #[must_use]
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ts = Utc::now().timestamp_millis();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!("iso-{ts}-{seq}"))
    }

    /// Parse a workspace ID from a string, validating it's non-empty.
    pub fn parse(s: String) -> std::result::Result<Self, IsolateError> {
        if s.is_empty() {
            return Err(IsolateError::InvalidWorkspaceId("empty id".into()));
        }
        Ok(Self(s))
    }

    /// View the workspace ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Unique identifier for a bead (work unit) in the tracking system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    /// Parse a bead ID from a string, validating it's non-empty.
    pub fn parse(s: String) -> std::result::Result<Self, IsolateError> {
        if s.is_empty() {
            return Err(IsolateError::InvalidBeadId("empty id".into()));
        }
        Ok(Self(s))
    }

    /// View the bead ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BeadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Mapping between a bead (work unit) and its assigned workspace.
///
/// Each bead maps to at most one workspace. This is the link between
/// the task tracking system and the isolation system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadWorkspaceMapping {
    bead_id: BeadId,
    workspace_id: WorkspaceId,
    assigned_at: DateTime<Utc>,
}

impl BeadWorkspaceMapping {
    /// Create a new bead-to-workspace mapping.
    #[must_use]
    pub fn new(bead_id: BeadId, workspace_id: WorkspaceId) -> Self {
        Self {
            bead_id,
            workspace_id,
            assigned_at: Utc::now(),
        }
    }

    /// The bead ID in this mapping.
    #[must_use]
    pub fn bead_id(&self) -> &BeadId {
        &self.bead_id
    }

    /// The workspace ID in this mapping.
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// When this mapping was created.
    #[must_use]
    pub fn assigned_at(&self) -> DateTime<Utc> {
        self.assigned_at
    }
}
