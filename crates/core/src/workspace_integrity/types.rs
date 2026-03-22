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
    /// .jj directory is missing
    MissingJjDir,
    /// .jj directory is corrupted (e.g. empty or missing files)
    CorruptedJjDir,
    /// Stale lock files exist
    StaleLocks,
    /// Permission issues
    PermissionDenied,
    /// Git index is corrupted (if using Git colocation)
    CorruptedGitIndex,
}

impl std::fmt::Display for CorruptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDirectory => write!(f, "missing_directory"),
            Self::MissingJjDir => write!(f, "missing_jj_dir"),
            Self::CorruptedJjDir => write!(f, "corrupted_jj_dir"),
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
            "missing_jj_dir" => Ok(Self::MissingJjDir),
            "corrupted_jj_dir" => Ok(Self::CorruptedJjDir),
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
    /// Attempt to fix JJ directory structure
    FixJjDir,
    /// Re-initialize JJ in the workspace
    RecreateWorkspace,
    /// Forget workspace in JJ and add it again
    ForgetAndRecreate,
}

impl RepairStrategy {
    /// Get a human-readable description of the strategy
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::NoRepair | Self::NoRepairPossible => "No automated repair possible",
            Self::ClearLocks => "Clear stale lock files",
            Self::FixJjDir => "Fix JJ directory structure",
            Self::RecreateWorkspace => "Recreate workspace",
            Self::ForgetAndRecreate => "Forget and recreate workspace",
        }
    }
}

impl std::fmt::Display for RepairStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRepair => write!(f, "no_repair"),
            Self::NoRepairPossible => write!(f, "no_repair_possible"),
            Self::ClearLocks => write!(f, "clear_locks"),
            Self::FixJjDir => write!(f, "fix_jj_dir"),
            Self::RecreateWorkspace => write!(f, "recreate_workspace"),
            Self::ForgetAndRecreate => write!(f, "forget_and_recreate"),
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
            "fix_jj_dir" => Ok(Self::FixJjDir),
            "recreate_workspace" => Ok(Self::RecreateWorkspace),
            "forget_and_recreate" => Ok(Self::ForgetAndRecreate),
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
