//! VCS data types for Source Control Plane.
//!
//! Immutable data structures representing VCS concepts.

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
    /// Jujutsu VCS
    Jujutsu,
    /// Git VCS
    Git,
}
