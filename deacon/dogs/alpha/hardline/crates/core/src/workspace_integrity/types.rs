//! Types and enums for workspace integrity
//!
//! Contains all the type definitions for corruption detection,
//! severity levels, repair strategies, and result types.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// CORRUPTION TYPE
// ═══════════════════════════════════════════════════════════════════════════

/// Types of workspace corruption/issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorruptionType {
    /// Workspace directory is missing
    MissingDirectory,
    /// .git directory is missing
    MissingGitDir,
    /// .git directory is corrupted (e.g. empty or missing objects)
    CorruptedGitDir,
    /// Stale lock files exist
    StaleLocks,
    /// Permission issues
    PermissionDenied,
    /// Git index is corrupted
    CorruptedGitIndex,
}

impl std::fmt::Display for CorruptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDirectory => write!(f, "missing_directory"),
            Self::MissingGitDir => write!(f, "missing_git_dir"),
            Self::CorruptedGitDir => write!(f, "corrupted_git_dir"),
            Self::StaleLocks => write!(f, "stale_locks"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::CorruptedGitIndex => write!(f, "corrupted_git_index"),
        }
    }
}

impl FromStr for CorruptionType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "missing_directory" => Ok(Self::MissingDirectory),
            "missing_git_dir" => Ok(Self::MissingGitDir),
            "corrupted_git_dir" => Ok(Self::CorruptedGitDir),
            "stale_locks" => Ok(Self::StaleLocks),
            "permission_denied" => Ok(Self::PermissionDenied),
            "corrupted_git_index" => Ok(Self::CorruptedGitIndex),
            _ => Err(()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR STRATEGY
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy for repairing a corruption issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// No automated repair possible
    NoRepair,
    /// No automated repair possible (alternative name for compatibility)
    NoRepairPossible,
    /// Remove stale lock files
    ClearLocks,
    /// Attempt to fix Git directory structure
    FixGitDir,
    /// Re-initialize Git in the workspace
    RecreateWorkspace,
    /// Remove workspace and re-clone it
    RemoveAndReclone,
}

impl RepairStrategy {
    /// Get a human-readable description of the strategy
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::NoRepair | Self::NoRepairPossible => "No automated repair possible",
            Self::ClearLocks => "Clear stale lock files",
            Self::FixGitDir => "Fix Git directory structure",
            Self::RecreateWorkspace => "Recreate workspace",
            Self::RemoveAndReclone => "Remove and re-clone workspace",
        }
    }
}

impl std::fmt::Display for RepairStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRepair => write!(f, "no_repair"),
            Self::NoRepairPossible => write!(f, "no_repair_possible"),
            Self::ClearLocks => write!(f, "clear_locks"),
            Self::FixGitDir => write!(f, "fix_git_dir"),
            Self::RecreateWorkspace => write!(f, "recreate_workspace"),
            Self::RemoveAndReclone => write!(f, "remove_and_reclone"),
        }
    }
}

impl FromStr for RepairStrategy {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "no_repair" => Ok(Self::NoRepair),
            "no_repair_possible" => Ok(Self::NoRepairPossible),
            "clear_locks" => Ok(Self::ClearLocks),
            "fix_git_dir" => Ok(Self::FixGitDir),
            "recreate_workspace" => Ok(Self::RecreateWorkspace),
            "remove_and_reclone" => Ok(Self::RemoveAndReclone),
            _ => Err(()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SEVERITY
// ═══════════════════════════════════════════════════════════════════════════

/// Severity level of an integrity issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational or minor warning
    Info,
    /// Warning - potential issues, but may work
    Warn,
    /// Error - workspace is unusable without repair
    Fail,
    /// Critical - multiple failures or unfixable corruption
    Critical,
}
